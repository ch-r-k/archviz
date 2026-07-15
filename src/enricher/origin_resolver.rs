use std::collections::HashSet;

use crate::enricher::GraphEnricher;
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
        let local_names: HashSet<String> = graph.nodes.iter().map(|n| n.name.clone()).collect();
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

/// Emits a synthetic node for `name` if it isn't already present locally
/// and hasn't been emitted during this pass. Skips things that look like
/// generic type parameters (see [`looks_like_type_param`]).
fn maybe_emit(
    name: &str,
    local_names: &HashSet<String>,
    created: &mut HashSet<String>,
    new_nodes: &mut Vec<Node>,
) {
    if created.contains(name) || looks_like_type_param(name) {
        return;
    }
    let module_path = match classify(name, local_names) {
        TypeOrigin::Local => return,
        other => other.module_path().unwrap_or_default(),
    };

    new_nodes.push(Node {
        name: name.to_string(),
        kind: NodeKind::Synthetic { params: None },
        module_path,
    });
    created.insert(name.to_string());
}
