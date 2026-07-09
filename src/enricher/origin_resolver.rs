use std::collections::HashSet;

use crate::enricher::GraphEnricher;
use crate::enricher::origin::{classify, TypeOrigin};
use crate::model::{Graph, Node, NodeKind, TypeExpr};

/// Walks every node in the graph, inspects each outgoing edge, and classifies
/// the referenced type as local, from the standard library, or from an
/// external crate. Types that have no node yet are materialized as synthetic
/// nodes placed inside the `std` or `external` package so they show up in the
/// generated diagram.
///
/// Local types (already present as nodes) are left untouched.
///
/// This is a heuristic pass driven purely by type names — a later revision
/// could use rust-analyzer to resolve types precisely.
pub struct OriginResolver;

impl GraphEnricher for OriginResolver {
    fn enrich(&self, graph: &mut Graph) {
        let local_names = local_node_names(graph);
        let mut created: HashSet<String> = HashSet::new();
        let mut new_nodes: Vec<Node> = Vec::new();

        for edge in &graph.edges {
            let target = edge.to.resolve_refs();
            classify_expr(target, &local_names, &mut created, &mut new_nodes);
        }

        graph.nodes.extend(new_nodes);
    }
}

/// Names of nodes that are part of the analyzed project (structs, traits,
/// enums, type aliases, impl blocks, and any previously-generated synthetic
/// nodes).
fn local_node_names(graph: &Graph) -> HashSet<String> {
    let mut names: HashSet<String> = HashSet::new();
    for n in &graph.nodes {
        names.insert(n.name.clone());
    }
    names
}

/// Recurses through a type expression, emitting a synthetic node for each
/// leaf name whose origin is `Std` or `External` and that isn't already
/// represented in the graph.
fn classify_expr(
    expr: &TypeExpr,
    local_names: &HashSet<String>,
    created: &mut HashSet<String>,
    new_nodes: &mut Vec<Node>,
) {
    match expr {
        TypeExpr::Simple(name) => {
            maybe_emit(name, local_names, created, new_nodes);
        }
        TypeExpr::Generic { base, args } => {
            maybe_emit(base, local_names, created, new_nodes);
            for arg in args {
                classify_expr(arg.resolve_refs(), local_names, created, new_nodes);
            }
        }
        TypeExpr::Slice(inner) | TypeExpr::Array(inner) => {
            classify_expr(inner.resolve_refs(), local_names, created, new_nodes);
        }
        TypeExpr::Tuple(items) => {
            for item in items {
                classify_expr(item.resolve_refs(), local_names, created, new_nodes);
            }
        }
        TypeExpr::DynTrait(traits) | TypeExpr::ImplTrait(traits) => {
            for t in traits {
                maybe_emit(t, local_names, created, new_nodes);
            }
        }
        TypeExpr::Reference(_) | TypeExpr::Unknown => {}
    }
}

/// Emits a synthetic node for `name` if it isn't already present locally and
/// hasn't been emitted during this pass. Local types and things that look
/// like generic type parameters (single upper-case letter or letter+digit,
/// e.g. `T`, `K`, `V`, `E`, `R`, `T1`) are skipped — the parser currently
/// surfaces those as edge targets but they don't correspond to real types.
fn maybe_emit(
    name: &str,
    local_names: &HashSet<String>,
    created: &mut HashSet<String>,
    new_nodes: &mut Vec<Node>,
) {
    if created.contains(name) || looks_like_type_param(name) {
        return;
    }
    let origin = classify(name, local_names);
    let module_path = match origin {
        TypeOrigin::Local => return,
        other => other.module_path().unwrap_or_default(),
    };

    new_nodes.push(Node {
        name: name.to_string(),
        kind: NodeKind::Synthetic { expr: Some(TypeExpr::Simple(name.to_string())) },
        module_path,
    });
    created.insert(name.to_string());
}

/// Heuristic: single upper-case letter, optionally followed by a digit,
/// looks like a Rust generic type parameter (`T`, `K`, `V`, `R`, `E`, `T1`).
fn looks_like_type_param(name: &str) -> bool {
    let mut chars = name.chars();
    match (chars.next(), chars.next(), chars.next()) {
        (Some(c), None, _) => c.is_ascii_uppercase(),
        (Some(c), Some(d), None) => c.is_ascii_uppercase() && d.is_ascii_digit(),
        _ => false,
    }
}
