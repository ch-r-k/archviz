use std::collections::HashMap;
use std::collections::HashSet;

use crate::enricher::GraphEnricher;
use crate::enricher::origin::is_std_name;
use crate::model::{Edge, Graph, Node, NodeKind, Relation, TypeExpr};

/// Pipeline enricher that expands complex type expressions (generics, slices,
/// tuples, …) into explicit synthetic nodes and edges.
///
/// For example, a field of type `Vec<String>` produces:
/// - a generic base node `Vec<T>` in package "std"
/// - a concrete specialization node `Vec<String>` 
/// - a `Specializes` edge `Vec<String> <|-- Vec<T>`
/// - a composition edge `Vec<String> --> String`
///
/// `dyn Trait` / `impl Trait` args are resolved directly to the trait node
/// rather than producing an intermediate node.
///
/// Synthetic nodes inherit the module path of the first node that references
/// them, so they appear inside the correct package in the diagram.
pub struct TypeExpander;

impl GraphEnricher for TypeExpander {
    fn enrich(&self, graph: &mut Graph) {
        let existing = existing_node_names(graph);
        let node_module = node_module_index(graph);

        let mut seen: HashSet<String> = HashSet::new();
        let mut new_nodes: Vec<Node> = Vec::new();
        let mut new_edges: Vec<Edge> = Vec::new();
        let mut retained_edges: Vec<Edge> = Vec::new();

        for edge in graph.edges.drain(..).collect::<Vec<_>>() {
            let module_path = node_module.get(&edge.from).cloned().unwrap_or_default();

            if is_trait_wrapper(&edge.to) {
                // Trait wrappers (Box<dyn T>, Vec<dyn T>, ...) don't get a
                // synthetic node — replace the edge with a direct edge to the
                // underlying trait(s) instead.
                new_edges.extend(direct_trait_edges(&edge));
            } else {
                collect(
                    edge.to.resolve_refs(),
                    &module_path,
                    &existing,
                    &mut seen,
                    &mut new_nodes,
                    &mut new_edges,
                );
                retained_edges.push(edge);
            }
        }

        graph.edges = retained_edges;
        graph.nodes.extend(new_nodes);
        graph.edges.extend(new_edges);
    }
}

/// Names that already have a node — never emit a synthetic duplicate for them.
fn existing_node_names(graph: &Graph) -> HashSet<String> {
    graph.nodes.iter().map(|n| n.name.clone()).collect()
}

/// Maps node name → module path, used to place synthetic nodes in the same
/// package as the first struct/enum that references them.
fn node_module_index(graph: &Graph) -> HashMap<String, Vec<String>> {
    graph
        .nodes
        .iter()
        .map(|n| (n.name.clone(), n.module_path.clone()))
        .collect()
}

/// Builds direct composition/implements edges from `edge.from` to the
/// trait(s) wrapped by `edge.to` (e.g. `Box<dyn Foo>` → edge to `Foo`).
fn direct_trait_edges(edge: &Edge) -> Vec<Edge> {
    extract_traits(&edge.to)
        .unwrap_or_default()
        .into_iter()
        .map(|trait_name| Edge {
            from: edge.from.clone(),
            to: TypeExpr::Simple(trait_name),
            relation: edge.relation.clone(),
        })
        .collect()
}

/// Checks if a type is a wrapper (possibly nested) around a dyn/impl trait.
fn is_trait_wrapper(expr: &TypeExpr) -> bool {
    match expr {
        TypeExpr::DynTrait(_) | TypeExpr::ImplTrait(_) => true,
        TypeExpr::Generic { args, .. } if !args.is_empty() => {
            // Recursively check if any argument is a trait wrapper
            args.iter()
                .any(|arg| is_trait_wrapper(arg.resolve_refs()))
        }
        TypeExpr::Reference(inner) => is_trait_wrapper(inner.resolve_refs()),
        _ => false,
    }
}

/// Extracts trait names from a type that contains one or more trait objects,
/// recursively unwrapping wrappers like Box, Vec, Option, etc.
fn extract_traits(expr: &TypeExpr) -> Option<Vec<String>> {
    match expr {
        TypeExpr::DynTrait(traits) | TypeExpr::ImplTrait(traits) => Some(traits.clone()),
        TypeExpr::Generic { args, .. } if !args.is_empty() => {
            // Look through generic arguments for traits
            for arg in args {
                if let Some(traits) = extract_traits(arg.resolve_refs()) {
                    return Some(traits);
                }
            }
            None
        }
        TypeExpr::Reference(inner) => extract_traits(inner.resolve_refs()),
        _ => None,
    }
}

/// Returns the edge target name(s) for a type argument:
/// - `dyn Trait` / `impl Trait` → the individual trait names, pointing directly
///   at the existing trait nodes (no intermediate `"dyn Foo"` node).
/// - everything else → `[expr.type_name()]`
fn edge_targets(expr: &TypeExpr) -> Vec<String> {
    match expr {
        TypeExpr::DynTrait(traits) | TypeExpr::ImplTrait(traits) => traits.clone(),
        other => vec![other.type_name()],
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
    let name = expr.type_name();
    match expr {
        TypeExpr::Simple(_) => {}

        TypeExpr::DynTrait(_) | TypeExpr::ImplTrait(_) => {
            // Resolved directly to trait names by the parent via edge_targets().
            // No synthetic node needed.
        }

        TypeExpr::Generic { args, .. } if !args.is_empty() => {
            if seen.insert(name.clone()) {
                if !existing.contains(&name) {
                    // Create the concrete specialization node
                    new_nodes.push(synthetic_node(expr, module_path.to_vec()));
                    
                    // Create a generic base node and specialization edge
                    let (generic_node, specializes_edge) = create_generic_base_and_specialization(expr);
                    if !existing.contains(&generic_node.name) && !new_nodes.iter().any(|n| n.name == generic_node.name) {
                        new_nodes.push(generic_node);
                    }
                    new_edges.push(specializes_edge);
                }
                for arg in args {
                    let arg = arg.resolve_refs();
                    for target in edge_targets(arg) {
                        new_edges.push(composition(name.clone(), target));
                    }
                    collect(arg, module_path, existing, seen, new_nodes, new_edges);
                }
            }
        }

        TypeExpr::Slice(inner) | TypeExpr::Array(inner) => {
            let inner = inner.resolve_refs();
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
                    let item = item.resolve_refs();
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
        name: expr.type_name(),
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

/// Creates a generic base node and a specialization edge for a concrete generic type.
/// For example, `Vec<String>` creates:
/// - A generic base node `Vec<T>` in the "std" package
/// - A `Specializes` edge from `Vec<String>` to `Vec<T>`
fn create_generic_base_and_specialization(expr: &TypeExpr) -> (Node, Edge) {
    if let TypeExpr::Generic { base, .. } = expr {
        let concrete_name = expr.type_name();
        let generic_name = format!("{}<T>", base);
        let module_path = if is_std_name(base) {
            vec!["std".to_string()]
        } else {
            vec![]
        };

        let generic_node = Node {
            name: generic_name.clone(),
            kind: NodeKind::Synthetic {
                expr: Some(TypeExpr::Generic {
                    base: base.clone(),
                    args: vec![TypeExpr::Simple("T".to_string())],
                }),
            },
            module_path,
        };

        let specializes_edge = Edge {
            from: concrete_name,
            to: TypeExpr::Simple(generic_name),
            relation: Relation::Specializes,
        };

        (generic_node, specializes_edge)
    } else {
        unreachable!("create_generic_base_and_specialization called with non-generic type")
    }
}
