use std::collections::HashSet;

use crate::enricher::GraphEnricher;
use crate::model::node::NodeId;
use crate::model::origin::{is_lang_root, is_std_name, looks_like_type_param};
use crate::model::{Graph, Node, NodeKind};
use crate::resolution::{CrateIndex, ResolutionContext};

/// Iterates every edge target, classifies the referenced type and
/// materializes a synthetic stub node under the `std` or `external` package
/// for types with no matching node yet — so they still appear in the
/// diagram.
///
/// Qualified targets (e.g. `anyhow::Error`, `std::collections::HashMap`)
/// are classified by their leading path segment: language roots
/// (`std`/`core`/`alloc`) go under `std`, crate names found in the
/// [`CrateIndex`] go under `external::<crate>::…`, and `crate::`/`self::`/
/// `super::` paths are left unresolved for lexical resolution. Bare names
/// fall back to the name-based std/primitive list.
pub struct OriginResolver;

impl GraphEnricher for OriginResolver {
    fn enrich(&self, graph: &mut Graph, ctx: &ResolutionContext<'_>) {
        // Any node that already exists — matched by either its `id` or
        // its `display_name` — counts as "local" for the purposes of
        // origin classification. Cross-module bare edge targets (which
        // resolution left as bare `NodeId`s because of an ambiguous
        // display name) match via `display_name` and must not be
        // duplicated as external stubs.
        let mut local_names: HashSet<String> = HashSet::new();
        for n in &graph.nodes {
            local_names.insert(n.display_name.clone());
            local_names.insert(n.id.0.clone());
        }

        let mut created: HashSet<String> = HashSet::new();
        let mut new_nodes: Vec<Node> = Vec::new();

        for edge in &graph.edges {
            maybe_emit(
                &edge.to,
                &local_names,
                ctx.crate_index,
                &mut created,
                &mut new_nodes,
            );
        }

        for node in new_nodes {
            graph.push_node(node);
        }
    }
}

fn maybe_emit(
    target: &NodeId,
    local_names: &HashSet<String>,
    crate_index: Option<&CrateIndex>,
    created: &mut HashSet<String>,
    new_nodes: &mut Vec<Node>,
) {
    let name = target.as_str();
    if created.contains(name) || looks_like_type_param(name) {
        return;
    }

    // A target that already names a source-level node (by id or display
    // name) is local and needs no stub.
    if local_names.contains(name) {
        return;
    }

    let Some((display, module_path)) = classify_target(name, crate_index) else {
        return;
    };

    new_nodes.push(Node {
        id: target.clone(),
        display_name: display,
        kind: NodeKind::Synthetic { params: None },
        module_path,
    });
    created.insert(name.to_string());
}

/// Classifies an *unresolved* edge target into a stub's display name and
/// module path. Returns `None` when no stub should be materialized (e.g.
/// `crate::`-qualified paths, which lexical resolution will handle).
fn classify_target(
    name: &str,
    crate_index: Option<&CrateIndex>,
) -> Option<(String, Vec<String>)> {
    let mut segments: Vec<&str> = name.split("::").collect();

    if segments.len() == 1 {
        let base = segments[0];
        if is_std_name(base) {
            return Some((base.to_string(), vec!["std".to_string()]));
        }
        return Some((base.to_string(), vec!["external".to_string()]));
    }

    let display = segments.pop().unwrap().to_string();
    let root = segments[0];

    if root == "crate" || root == "self" || root == "super" {
        // These need scope knowledge to resolve; without it we'd guess.
        return None;
    }

    if is_lang_root(root) {
        return Some((display, vec!["std".to_string()]));
    }

    if let Some(index) = crate_index {
        if index.is_external(root) {
            let mut module_path = vec!["external".to_string(), root.to_string()];
            for segment in &segments[1..] {
                module_path.push((*segment).to_string());
            }
            return Some((display, module_path));
        }
    }

    // Unknown qualified path without a crate index: keep the old
    // flat-`external` behavior rather than dropping the type.
    Some((display, vec!["external".to_string()]))
}
