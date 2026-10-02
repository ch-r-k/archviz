//! Post-parse pass that rewrites edge targets to fully-qualified
//! [`NodeId`]s using the lexical module index.

use std::collections::HashSet;

use crate::enricher::GraphEnricher;
use crate::model::node::NodeId;
use crate::model::{Graph, NodeKind};
use crate::resolution::ResolutionContext;
use crate::resolution::scope::resolve_qualified;

/// Replaces the old name-based `EdgeTargetResolver` with a lexical
/// resolver that uses the project's import table and definition index.
pub struct LexicalResolver;

impl GraphEnricher for LexicalResolver {
    fn enrich(&self, graph: &mut Graph, ctx: &ResolutionContext<'_>) {
        let module_index = ctx.module_index;

        // Build a set of synthetic node display names (compound types, etc.)
        // that should keep their bare ids.
        let synthetic_ids: HashSet<String> = graph
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Synthetic { .. }))
            .map(|n| n.display_name.clone())
            .collect();

        // Build a lookup: base name → candidate source-level nodes.
        let mut by_display: std::collections::HashMap<String, Vec<NodeId>> = std::collections::HashMap::new();
        for n in &graph.nodes {
            if !matches!(n.kind, NodeKind::Synthetic { .. }) {
                by_display
                    .entry(n.display_name.clone())
                    .or_default()
                    .push(n.id.clone());
            }
        }
        // Sort candidates for deterministic tie-breaking.
        for candidates in by_display.values_mut() {
            candidates.sort_by(|a, b| a.0.cmp(&b.0));
        }

        for edge in &mut graph.edges {
            let name = edge.to.as_str();

            // Synthetic nodes (compound types, generic bases) keep their bare ids.
            if synthetic_ids.contains(name) {
                continue;
            }

            let owner_module = edge.from.module_prefix().unwrap_or_default().split("::").map(|s| s.to_string()).collect::<Vec<_>>();

            // Handle both bare and qualified targets.
            if name.contains("::") {
                // Qualified target: resolve crate::/self::/super:: prefixes.
                let path: Vec<String> = name.split("::").map(|s| s.to_string()).collect();
                let resolved = resolve_qualified(&path, &owner_module, module_index);
                if resolved != path {
                    // Rewrote the path (e.g., stripped crate::).
                    let module_path = if resolved.len() > 1 {
                        resolved[..resolved.len() - 1].to_vec()
                    } else {
                        Vec::new()
                    };
                    let name = resolved.last().cloned().unwrap_or_default();
                    edge.to = NodeId::from_parts(&module_path, &name);
                    continue;
                }
                // Path unchanged - leave for OriginResolver or local lookup.
            } else {
                // Bare target: try lexical resolution.
                if let Some(resolved) = resolve_via_index(name, &owner_module, module_index) {
                    edge.to = resolved;
                    continue;
                }
            }

            // Fallback: try local graph lookup (same logic as old resolver).
            // This preserves parity for cases where the index missed something.
            if let Some(candidates) = graph
                .nodes
                .iter()
                .filter(|n| n.display_name == name && !matches!(n.kind, NodeKind::Synthetic { .. }))
                .map(|n| n.id.clone())
                .collect::<Vec<_>>()
                .into_iter()
                .next()
            {
                edge.to = candidates;
            }
            // Otherwise leave bare for OriginResolver.
        }
    }
}

/// Attempts to resolve a bare target `name` in the context of `owner_module`
/// using the ModuleIndex (imports + definitions).
fn resolve_via_index(name: &str, owner_module: &[String], module_index: &crate::resolution::ModuleIndex) -> Option<NodeId> {
    use crate::resolution::scope::resolve_bare;

    let resolution = resolve_bare(name, owner_module, module_index, None);
    match resolution {
        crate::resolution::scope::Resolution::Local(path) => {
            // Build the fully-qualified NodeId from the module path + name.
            let name = path.last().cloned().unwrap_or_default();
            let module_path = if path.len() > 1 {
                path[..path.len() - 1].to_vec()
            } else {
                Vec::new()
            };
            Some(NodeId::from_parts(&module_path, &name))
        }
        _ => None,
    }
}