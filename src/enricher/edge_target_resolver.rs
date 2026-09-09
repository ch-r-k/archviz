//! Post-parse pass that rewrites bare edge targets to fully-qualified
//! [`NodeId`]s.
//!
//! At parse time [`crate::parser::graph_builder::GraphBuilder`] emits
//! edge targets as `NodeId::bare(name)` because the full set of nodes
//! (across all files) isn't known yet. This enricher runs once the graph
//! is complete and promotes each bare target to a resolved id whenever
//! the display name unambiguously identifies a source-level node.
//!
//! Runs before [`crate::enricher::origin_resolver::OriginResolver`] so
//! that std/external stub creation sees only genuinely unresolved
//! targets.

use std::collections::HashMap;

use crate::enricher::GraphEnricher;
use crate::model::node::NodeId;
use crate::model::{Graph, NodeKind};

/// Resolution rules for each bare target `Foo` seen on an edge:
/// 1. If a synthetic node already has bare id `Foo` (compound types,
///    generics), keep the bare id — it already matches by construction.
/// 2. Else, gather all source-level nodes whose `display_name == Foo`:
///    * exactly one candidate → rewrite to that node's id;
///    * one candidate lives in the same module as the edge's owner →
///      prefer it;
///    * still ambiguous → warn on stderr, pick the first (sorted) so
///      output stays deterministic.
/// 3. No matching local node → leave bare; [`OriginResolver`] will turn
///    it into a std/external stub.
pub struct EdgeTargetResolver;

impl GraphEnricher for EdgeTargetResolver {
    fn enrich(&self, graph: &mut Graph) {
        let (synthetic_ids, by_display) = index_nodes(graph);

        for edge in &mut graph.edges {
            if !edge.to.is_bare() {
                continue;
            }
            let name = edge.to.as_str();

            if synthetic_ids.contains(name) {
                continue;
            }

            let Some(candidates) = by_display.get(name) else {
                continue;
            };

            edge.to = choose_candidate(name, &edge.from, candidates);
        }
    }
}

fn index_nodes(
    graph: &Graph,
) -> (
    std::collections::HashSet<String>,
    HashMap<String, Vec<NodeId>>,
) {
    let mut synthetic_ids = std::collections::HashSet::new();
    let mut by_display: HashMap<String, Vec<NodeId>> = HashMap::new();

    for n in &graph.nodes {
        match n.kind {
            NodeKind::Synthetic { .. } => {
                synthetic_ids.insert(n.display_name.clone());
            }
            _ => {
                by_display
                    .entry(n.display_name.clone())
                    .or_default()
                    .push(n.id.clone());
            }
        }
    }
    for candidates in by_display.values_mut() {
        candidates.sort_by(|a, b| a.0.cmp(&b.0));
    }

    (synthetic_ids, by_display)
}

fn choose_candidate(name: &str, owner: &NodeId, candidates: &[NodeId]) -> NodeId {
    if candidates.len() == 1 {
        return candidates[0].clone();
    }

    if let Some(same_module) = pick_same_module(candidates, owner.module_prefix()) {
        return same_module;
    }

    eprintln!(
        "archviz: ambiguous target `{}` for edge from `{}` \
         (candidates: {}); picking first",
        name,
        owner,
        candidates
            .iter()
            .map(|c| c.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    candidates[0].clone()
}

fn pick_same_module(candidates: &[NodeId], owner_module: Option<&str>) -> Option<NodeId> {
    let owner_module = owner_module?;
    candidates
        .iter()
        .find(|c| c.module_prefix() == Some(owner_module))
        .cloned()
}
