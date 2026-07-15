use std::collections::HashSet;

use crate::enricher::GraphEnricher;
use crate::model::node::NodeId;
use crate::model::origin::{classify, looks_like_type_param, TypeOrigin};
use crate::model::{Graph, Node, NodeKind};

/// Iterates every edge target, classifies the referenced type as `Local`
/// (a node already exists), `Std` (matches a known stdlib/primitive name),
/// or `External`, and materializes a synthetic stub node under the `std`
/// or `external` package for types with no matching node yet — so they
/// still appear in the diagram.
///
/// The classification is name-based (see `src/model/origin.rs`); a later
/// revision could delegate to rust-analyzer for precise resolution.
pub struct OriginResolver;

impl GraphEnricher for OriginResolver {
    fn enrich(&self, graph: &mut Graph) {
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
            maybe_emit(&edge.to, &local_names, &mut created, &mut new_nodes);
        }

        for node in new_nodes {
            graph.push_node(node);
        }
    }
}

fn maybe_emit(
    target: &NodeId,
    local_names: &HashSet<String>,
    created: &mut HashSet<String>,
    new_nodes: &mut Vec<Node>,
) {
    let name = target.as_str();
    if created.contains(name) || looks_like_type_param(name) {
        return;
    }
    let module_path = match classify(name, local_names) {
        TypeOrigin::Local => return,
        other => other.module_path().unwrap_or_default(),
    };

    new_nodes.push(Node {
        id: target.clone(),
        display_name: name.to_string(),
        kind: NodeKind::Synthetic { params: None },
        module_path,
    });
    created.insert(name.to_string());
}
