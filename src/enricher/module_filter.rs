//! `GraphEnricher` that applies a [`FilterSpec`] to the graph.
//!
//! Runs at the *end* of the enricher chain (after `EdgeTargetResolver`
//! and `OriginResolver`), so every edge target is already a resolved
//! [`NodeId`] and node drop / edge redirection are mechanical rewrites,
//! not heuristics.
//!
//! Steps:
//! 1. Compute per-node decisions with [`FilterSpec::decides`].
//! 2. Synthesize one [`NodeKind::Package`] node per unique collapse
//!    root (outermost collapse wins when they nest).
//! 3. Drop nodes marked `Drop` or `Collapse` (except the synthesized
//!    package itself).
//! 4. For each edge, redirect endpoints under a collapse root to the
//!    synthesized package, drop edges touching a dropped node, then
//!    drop self-edges and duplicates.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::enricher::GraphEnricher;
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

    fn collect_decisions(&self, graph: &Graph) -> HashMap<NodeId, Decision> {
        let mut out: HashMap<NodeId, Decision> = HashMap::new();
        for node in &graph.nodes {
            out.insert(node.id.clone(), self.spec.decides(&node.module_path));
        }
        out
    }

    /// Maps every node whose module_path matches a collapse pattern to
    /// the module_path of the outermost matching collapse pattern (the
    /// one with the shortest matching module prefix). Nodes whose module
    /// path directly matches a collapse pattern map to their own path.
    fn collect_collapse_roots(
        &self,
        graph: &Graph,
        decisions: &HashMap<NodeId, Decision>,
    ) -> HashMap<NodeId, Vec<String>> {
        let mut roots: HashMap<NodeId, Vec<String>> = HashMap::new();
        for node in &graph.nodes {
            if decisions.get(&node.id) != Some(&Decision::Collapse) {
                continue;
            }
            let root = shortest_collapsing_prefix(&self.spec, &node.module_path);
            roots.insert(node.id.clone(), root);
        }
        roots
    }
}

impl GraphEnricher for ModuleFilter {
    fn enrich(&self, graph: &mut Graph) {
        if self.spec.is_empty() {
            return;
        }

        let decisions = self.collect_decisions(graph);
        let collapse_roots = self.collect_collapse_roots(graph, &decisions);

        // Unique collapse roots, sorted for deterministic output.
        let mut unique_roots: BTreeMap<Vec<String>, NodeId> = BTreeMap::new();
        for root_path in collapse_roots.values() {
            let id = package_node_id(root_path);
            unique_roots.insert(root_path.clone(), id);
        }
        // Also seed empty collapse patterns (roots that match no
        // existing node) so users still see the placeholder they asked
        // for.
        for pat in &self.spec.collapse {
            for extra in patterns_without_nodes(pat, &collapse_roots) {
                let id = package_node_id(&extra);
                unique_roots.entry(extra).or_insert(id);
            }
        }

        // Build a per-collapsed-node redirect map (NodeId → package NodeId).
        let mut redirect: HashMap<NodeId, NodeId> = HashMap::new();
        for (node_id, root_path) in &collapse_roots {
            redirect.insert(node_id.clone(), package_node_id(root_path));
        }

        let dropped: HashSet<NodeId> = decisions
            .iter()
            .filter_map(|(id, d)| match d {
                Decision::Drop => Some(id.clone()),
                _ => None,
            })
            .collect();

        // Retain non-dropped, non-collapsed nodes.
        graph.nodes.retain(|n| {
            let d = decisions.get(&n.id);
            !matches!(d, Some(Decision::Drop) | Some(Decision::Collapse))
        });

        // Rebuild the node_index by pushing every retained node through
        // a fresh graph, then move nodes/edges back. This keeps
        // Graph's internal index consistent without exposing new API.
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

        // Rewrite edges.
        let mut seen: HashSet<(NodeId, NodeId, u8)> = HashSet::new();
        let mut new_edges: Vec<Edge> = Vec::with_capacity(graph.edges.len());
        for edge in graph.edges.drain(..) {
            let from = redirect.get(&edge.from).cloned().unwrap_or(edge.from);
            let to = redirect.get(&edge.to).cloned().unwrap_or(edge.to);

            if dropped.contains(&from) || dropped.contains(&to) {
                continue;
            }
            if from == to {
                // Self-edge introduced by collapse (or already present).
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

/// Among the collapse patterns matching `module_path`, return the
/// shortest prefix of `module_path` that itself still matches — that is
/// the outermost collapse root. Falls back to `module_path` itself.
fn shortest_collapsing_prefix(spec: &FilterSpec, module_path: &[String]) -> Vec<String> {
    let mut best: Option<Vec<String>> = None;
    for len in 0..=module_path.len() {
        let prefix = &module_path[..len];
        for pat in &spec.collapse {
            if pat.matches(prefix) {
                let candidate = prefix.to_vec();
                match &best {
                    None => best = Some(candidate),
                    Some(cur) if candidate.len() < cur.len() => best = Some(candidate),
                    _ => {}
                }
            }
        }
        if best.is_some() {
            // Shortest wins; a shorter matching prefix can't appear
            // later in the loop since `len` is increasing.
            break;
        }
    }
    best.unwrap_or_else(|| module_path.to_vec())
}

/// Literal collapse patterns that didn't match any real node still
/// produce an empty package placeholder, per the plan. We only
/// synthesize placeholders for fully-literal patterns — wildcards would
/// match an unbounded set of module paths.
fn patterns_without_nodes(
    pat: &crate::filter::pattern::ModulePattern,
    existing: &HashMap<NodeId, Vec<String>>,
) -> Vec<Vec<String>> {
    let Some(literal) = pat.literal_path() else {
        return Vec::new();
    };
    for path in existing.values() {
        if path == &literal {
            return Vec::new();
        }
    }
    vec![literal]
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
