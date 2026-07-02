use std::collections::HashMap;
use std::collections::HashSet;

use crate::enricher::GraphEnricher;
use crate::model::{Edge, Graph, Node, NodeKind, Relation, TypeExpr};

/// Pipeline enricher that expands complex type expressions (generics, slices,
/// tuples, …) into explicit synthetic nodes and composition edges.
///
/// For example, a field of type `Vec<String>` produces:
/// - a synthetic node `Vec<String>`
/// - a composition edge  `Vec<String> *-- String`
///
/// `dyn Trait` / `impl Trait` args are resolved directly to the trait node
/// rather than producing an intermediate `"dyn Trait"` node.
///
/// Synthetic nodes inherit the module path of the first node that references
/// them, so they appear inside the correct package in the diagram.
pub struct TypeExpander;

impl GraphEnricher for TypeExpander {
    fn enrich(&self, graph: &mut Graph) {
        // Names that already have a node — never emit a synthetic duplicate.
        let existing: HashSet<String> = graph.nodes.iter().map(|n| n.name.clone()).collect();

        // Map from node name → module path, used to place synthetic nodes in
        // the same package as the first struct/enum that references them.
        let node_module: HashMap<String, Vec<String>> = graph
            .nodes
            .iter()
            .map(|n| (n.name.clone(), n.module_path.clone()))
            .collect();

        let mut seen: HashSet<String> = HashSet::new();
        let mut new_nodes: Vec<Node> = Vec::new();
        let mut new_edges: Vec<Edge> = Vec::new();

        // Snapshot current edges so we don't iterate while mutating.
        let edges: Vec<Edge> = graph.edges.clone();
        for edge in &edges {
            let module_path = node_module.get(&edge.from).cloned().unwrap_or_default();

            collect(
                resolve_refs(&edge.to),
                &module_path,
                &existing,
                &mut seen,
                &mut new_nodes,
                &mut new_edges,
            );
        }

        graph.nodes.extend(new_nodes);
        graph.edges.extend(new_edges);
    }
}

/// Strips top-level `&` / `& mut` references to reach the underlying type.
fn resolve_refs(expr: &TypeExpr) -> &TypeExpr {
    match expr {
        TypeExpr::Reference(inner) => resolve_refs(inner),
        other => other,
    }
}

/// Returns the display name used as the node identifier (e.g. `Vec<String>`).
fn type_name(expr: &TypeExpr) -> String {
    match expr {
        TypeExpr::Simple(name) => name.clone(),
        TypeExpr::Generic { base, args } => {
            if args.is_empty() {
                base.clone()
            } else {
                let rendered = args
                    .iter()
                    .map(|a| type_name(resolve_refs(a)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}<{}>", base, rendered)
            }
        }
        TypeExpr::Reference(inner) => type_name(inner),
        TypeExpr::Slice(inner) => format!("[{}]", type_name(resolve_refs(inner))),
        TypeExpr::Array(inner) => format!("[{}; N]", type_name(resolve_refs(inner))),
        TypeExpr::Tuple(items) => {
            let rendered = items
                .iter()
                .map(|i| type_name(resolve_refs(i)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("({})", rendered)
        }
        TypeExpr::DynTrait(traits) => format!("dyn {}", traits.join(" + ")),
        TypeExpr::ImplTrait(traits) => format!("impl {}", traits.join(" + ")),
        TypeExpr::Unknown => "_".to_string(),
    }
}

/// Returns the edge target name(s) for a type argument:
/// - `dyn Trait` / `impl Trait` → the individual trait names, pointing directly
///   at the existing trait nodes (no intermediate `"dyn Foo"` node).
/// - everything else → `[type_name(expr)]`
fn edge_targets(expr: &TypeExpr) -> Vec<String> {
    match expr {
        TypeExpr::DynTrait(traits) | TypeExpr::ImplTrait(traits) => traits.clone(),
        other => vec![type_name(other)],
    }
}

/// Recursively walks `expr`, emitting a synthetic node for every compound type
/// that doesn't already exist in the graph, plus composition edges to its inner
/// type arguments.
fn collect(
    expr: &TypeExpr,
    module_path: &[String],
    existing: &HashSet<String>,
    seen: &mut HashSet<String>,
    new_nodes: &mut Vec<Node>,
    new_edges: &mut Vec<Edge>,
) {
    let name = type_name(expr);
    match expr {
        TypeExpr::Simple(_) => {}

        TypeExpr::DynTrait(_) | TypeExpr::ImplTrait(_) => {
            // Resolved directly to trait names by the parent via edge_targets().
            // No synthetic node needed.
        }

        TypeExpr::Generic { args, .. } if !args.is_empty() => {
            if seen.insert(name.clone()) {
                if !existing.contains(&name) {
                    new_nodes.push(synthetic_node(expr, module_path.to_vec()));
                }
                for arg in args {
                    let arg = resolve_refs(arg);
                    for target in edge_targets(arg) {
                        new_edges.push(composition(name.clone(), target));
                    }
                    collect(arg, module_path, existing, seen, new_nodes, new_edges);
                }
            }
        }

        TypeExpr::Slice(inner) | TypeExpr::Array(inner) => {
            let inner = resolve_refs(inner);
            if seen.insert(name.clone()) {
                if !existing.contains(&name) {
                    new_nodes.push(synthetic_node(expr, module_path.to_vec()));
                }
                for target in edge_targets(inner) {
                    new_edges.push(composition(name.clone(), target));
                }
                collect(inner, module_path, existing, seen, new_nodes, new_edges);
            }
        }

        TypeExpr::Tuple(items) => {
            if seen.insert(name.clone()) {
                if !existing.contains(&name) {
                    new_nodes.push(synthetic_node(expr, module_path.to_vec()));
                }
                for item in items {
                    let item = resolve_refs(item);
                    for target in edge_targets(item) {
                        new_edges.push(composition(name.clone(), target));
                    }
                    collect(item, module_path, existing, seen, new_nodes, new_edges);
                }
            }
        }

        // Generic with no args, Reference (already resolved above), Unknown —
        // no synthetic node needed.
        _ => {}
    }
}

fn synthetic_node(expr: &TypeExpr, module_path: Vec<String>) -> Node {
    Node {
        name: type_name(expr),
        kind: NodeKind::Synthetic {
            expr: Some(expr.clone()),
        },
        module_path,
    }
}

fn composition(from: String, to: String) -> Edge {
    Edge {
        from,
        to: TypeExpr::Simple(to),
        relation: Relation::Composition,
    }
}
