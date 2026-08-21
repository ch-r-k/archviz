//! `GraphEnricher` that applies a [`FilterSpec`] to the graph.
//!
//! Runs at the *end* of the enricher chain (after `EdgeTargetResolver`
//! and `OriginResolver`), so every edge target is already a resolved
//! [`NodeId`] and node drop / edge redirection are mechanical rewrites,
//! not heuristics.
//!
//! Steps:
//! 1. Compute per-node `(Decision, Option<collapse_root>)` from
//!    `FilterSpec::decides` (exclude/include) plus [`Self::collapse_root_for`]
//!    (permissive subtree semantics + depth).
//! 2. Synthesize one [`NodeKind::Package`] node per unique collapse
//!    root (outermost collapse wins when they nest).
//! 3. Drop nodes marked `Drop` or `Collapse` (except the synthesized
//!    package itself).
//! 4. For each edge, redirect endpoints under a collapse root to the
//!    synthesized package, drop edges touching a dropped node, then
//!    drop self-edges and duplicates.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::enricher::GraphEnricher;
use crate::filter::pattern::ModulePattern;
use crate::filter::spec::{Decision, FilterSpec};
use crate::model::node::NodeId;
use crate::model::{Edge, Graph, Node, NodeKind};

pub struct ModuleFilter {
    spec: FilterSpec,
}

impl ModuleFilter {
    pub fn new(spec: FilterSpec) -> Self {
        Self { spec }
    }

    /// Decides what to do with a single module path.
    ///
    /// Collapse uses **subtree semantics**: a collapse pattern `a::b`
    /// (or `a::b::**` — both are treated the same here) matches
    /// `a::b`, `a::b::c`, and every deeper descendant. The collapse
    /// root is the outermost matching prefix.
    ///
    /// `collapse_depth = Some(n)` additionally forces any module at
    /// depth > n to collapse into its ancestor at depth n. The
    /// shallower of the two roots wins when both apply.
    ///
    /// `synthetic = true` skips the include-drop step: synthetic nodes
    /// (compound types, std/external stubs) survive include filters and
    /// are pruned later if no surviving edge references them.
    fn decide(&self, module_path: &[String], include_exempt: bool) -> (Decision, Option<Vec<String>>) {
        let decision = if include_exempt {
            // Exclude still applies (users may explicitly drop std),
            // but include does not — include-exempt nodes are always
            // kept unless explicitly excluded.
            if self.spec.exclude.iter().any(|p| any_prefix_match(p, module_path)) {
                Decision::Drop
            } else {
                Decision::Keep
            }
        } else {
            self.spec.decides(module_path)
        };
        match decision {
            Decision::Drop => return (Decision::Drop, None),
            Decision::Keep | Decision::Collapse => {}
        }
        if !include_exempt {
            if let Some(root) = self.collapse_root_for(module_path) {
                return (Decision::Collapse, Some(root));
            }
        }
        (Decision::Keep, None)
    }

    /// Shortest prefix of `module_path` that is either:
    /// * matched by any `--collapse` pattern (subtree semantics: the
    ///   pattern may match the prefix itself), or
    /// * the truncation `module_path[..N]` for `collapse_depth = Some(N)`,
    ///   when `module_path.len() > N`.
    fn collapse_root_for(&self, module_path: &[String]) -> Option<Vec<String>> {
        let mut best: Option<Vec<String>> = None;

        for len in 0..=module_path.len() {
            let prefix = &module_path[..len];
            if any_matches(&self.spec.collapse, prefix) {
                best = Some(prefix.to_vec());
                break;
            }
        }

        if let Some(n) = self.spec.collapse_depth {
            if module_path.len() > n {
                let candidate = module_path[..n].to_vec();
                best = match best {
                    None => Some(candidate),
                    Some(cur) if candidate.len() < cur.len() => Some(candidate),
                    Some(cur) => Some(cur),
                };
            }
        }

        best
    }
}

impl GraphEnricher for ModuleFilter {
    fn enrich(&self, graph: &mut Graph) {
        if self.spec.is_empty() {
            return;
        }

        // Per-node decisions and collapse roots. Synthetic std/external
        // stubs are exempt from include-drop — they exist purely as
        // edge targets, and users don't type `--include std`. Compound
        // types (`Vec<Segment>`) that live under a real module path
        // follow that module's include/exclude rules like any other
        // node; only nodes at empty module_path or under `std`/`external`
        // are treated as universal.
        let mut dropped: HashSet<NodeId> = HashSet::new();
        let mut collapse_roots: HashMap<NodeId, Vec<String>> = HashMap::new();
        for node in &graph.nodes {
            let include_exempt = is_stub_module_path(&node.module_path)
                && matches!(node.kind, NodeKind::Synthetic { .. });
            match self.decide(&node.module_path, include_exempt) {
                (Decision::Drop, _) => {
                    dropped.insert(node.id.clone());
                }
                (Decision::Collapse, Some(root)) => {
                    collapse_roots.insert(node.id.clone(), root);
                }
                _ => {}
            }
        }

        // Deterministic set of unique collapse roots.
        let mut unique_roots: BTreeMap<Vec<String>, NodeId> = BTreeMap::new();
        for root_path in collapse_roots.values() {
            let id = package_node_id(root_path);
            unique_roots.insert(root_path.clone(), id);
        }
        // Literal collapse patterns that matched *nothing* still get a
        // placeholder package, so the user sees what they asked for.
        // "Matched nothing" here means: no node in the original graph
        // lives at the literal path or under it.
        let all_paths: Vec<&[String]> =
            graph.nodes.iter().map(|n| n.module_path.as_slice()).collect();
        for pat in &self.spec.collapse {
            for extra in placeholder_for_pattern(pat, &all_paths) {
                let id = package_node_id(&extra);
                unique_roots.entry(extra).or_insert(id);
            }
        }

        // Redirect map: each collapsed node → its collapse-root package.
        let mut redirect: HashMap<NodeId, NodeId> = HashMap::new();
        for (node_id, root_path) in &collapse_roots {
            redirect.insert(node_id.clone(), package_node_id(root_path));
        }

        // Retain non-dropped, non-collapsed nodes.
        let collapsed_ids: HashSet<NodeId> = collapse_roots.keys().cloned().collect();
        graph.nodes.retain(|n| {
            !dropped.contains(&n.id) && !collapsed_ids.contains(&n.id)
        });
        rebuild_node_index(graph);

        // Synthesize collapse packages (deterministic order).
        for (root_path, id) in &unique_roots {
            let (parent, name) = split_parent_name(root_path);
            graph.push_node(Node {
                id: id.clone(),
                display_name: name,
                kind: NodeKind::Package,
                module_path: parent,
            });
        }

        // Rewrite edges: redirect endpoints, drop edges touching a
        // dropped node, drop self-edges introduced by collapse, dedupe.
        let mut seen: HashSet<(NodeId, NodeId, u8)> = HashSet::new();
        let mut new_edges: Vec<Edge> = Vec::with_capacity(graph.edges.len());
        for edge in graph.edges.drain(..) {
            let from = redirect.get(&edge.from).cloned().unwrap_or(edge.from);
            let to = redirect.get(&edge.to).cloned().unwrap_or(edge.to);

            if dropped.contains(&from) || dropped.contains(&to) {
                continue;
            }
            if from == to {
                continue;
            }

            let tag = relation_tag(&edge.relation);
            if !seen.insert((from.clone(), to.clone(), tag)) {
                continue;
            }
            new_edges.push(Edge {
                from,
                to,
                relation: edge.relation,
            });
        }
        graph.edges = new_edges;

        // Prune Synthetic nodes (compound types / std / external stubs)
        // that no surviving edge references — they were kept as
        // include-exempt "just in case" and became floaters.
        prune_orphan_synthetics(graph);
    }
}

fn prune_orphan_synthetics(graph: &mut Graph) {
    let mut referenced: HashSet<NodeId> = HashSet::new();
    for edge in &graph.edges {
        referenced.insert(edge.from.clone());
        referenced.insert(edge.to.clone());
    }
    let before = graph.nodes.len();
    graph.nodes.retain(|n| {
        !matches!(n.kind, NodeKind::Synthetic { .. }) || referenced.contains(&n.id)
    });
    if graph.nodes.len() != before {
        rebuild_node_index(graph);
    }
}

fn any_matches(patterns: &[ModulePattern], module_path: &[String]) -> bool {
    for p in patterns {
        if p.matches(module_path) {
            return true;
        }
    }
    false
}

/// True when `pat` matches `module_path` or any of its prefixes.
fn any_prefix_match(pat: &ModulePattern, module_path: &[String]) -> bool {
    for len in 0..=module_path.len() {
        if pat.matches(&module_path[..len]) {
            return true;
        }
    }
    false
}

/// True for module paths where a synthetic node is a *stub* rather than
/// a compound type belonging to a specific source module: the empty
/// path (bare generic bases from parsing) and the `std` / `external`
/// packages emitted by [`crate::enricher::origin_resolver::OriginResolver`].
/// These are the module paths a user should never have to name in a
/// `--include` filter.
fn is_stub_module_path(module_path: &[String]) -> bool {
    match module_path.first() {
        None => true,
        Some(seg) => seg == "std" || seg == "external",
    }
}

/// The `NodeId` used for a collapsed module's synthetic package: the
/// module path joined by `::`. Guaranteed distinct from any source-level
/// class node id (which always has a trailing `::<Name>` segment).
fn package_node_id(module_path: &[String]) -> NodeId {
    if module_path.is_empty() {
        NodeId::bare("<root>")
    } else {
        NodeId::bare(&module_path.join("::"))
    }
}

fn split_parent_name(module_path: &[String]) -> (Vec<String>, String) {
    match module_path.split_last() {
        None => (Vec::new(), "<root>".to_string()),
        Some((last, parent)) => (parent.to_vec(), last.clone()),
    }
}

fn relation_tag(rel: &crate::model::Relation) -> u8 {
    use crate::model::Relation;
    match rel {
        Relation::Composition => 0,
        Relation::Implements => 1,
        Relation::Specializes => 2,
    }
}

/// Emits a placeholder package for a fully-literal collapse pattern
/// only when *no* node in the original graph lives at (or under) the
/// literal path — that means the user asked for a subtree that isn't
/// there. Wildcard patterns get no placeholder (they'd match an
/// unbounded set of module paths).
fn placeholder_for_pattern(pat: &ModulePattern, all_paths: &[&[String]]) -> Vec<Vec<String>> {
    let Some(literal) = pat.literal_path() else {
        return Vec::new();
    };
    for path in all_paths {
        if path_starts_with(path, &literal) {
            return Vec::new();
        }
    }
    vec![literal]
}

fn path_starts_with(path: &[String], prefix: &[String]) -> bool {
    if path.len() < prefix.len() {
        return false;
    }
    for i in 0..prefix.len() {
        if path[i] != prefix[i] {
            return false;
        }
    }
    true
}

/// Reconstruct `Graph`'s internal `node_index` after in-place retention
/// via `Vec::retain`. `Graph` doesn't expose the field; the cheapest
/// portable fix is to swap in a fresh graph.
fn rebuild_node_index(graph: &mut Graph) {
    let mut fresh = Graph::default();
    for node in graph.nodes.drain(..) {
        fresh.push_node(node);
    }
    fresh.edges = std::mem::take(&mut graph.edges);
    *graph = fresh;
}
