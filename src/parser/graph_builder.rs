//! Turns a stream of high-level "the source contains …" observations
//! (`add_struct`, `add_trait`, `add_implements`, `add_field_type`) into
//! nodes and edges in the shared [`Graph`], including the on-the-fly
//! expansion of compound and trait-wrapper types into synthetic nodes.
//!
//! Knows nothing about `syn` — the AST-facing side lives in
//! [`crate::parser::visitor`].

use crate::model::origin::is_std_name;
use crate::model::{Edge, Graph, Node, NodeKind, Relation};
use crate::parser::type_expr::TypeExpr;

pub struct GraphBuilder<'a> {
    graph: &'a mut Graph,
    module_path: &'a Vec<String>,
}

impl<'a> GraphBuilder<'a> {
    pub fn new(graph: &'a mut Graph, module_path: &'a Vec<String>) -> Self {
        Self { graph, module_path }
    }

    pub fn add_struct(&mut self, name: &str) {
        self.graph.nodes.push(Node {
            name: name.to_string(),
            kind: NodeKind::Struct,
            module_path: self.module_path.clone(),
        });
    }

    pub fn add_trait(&mut self, name: &str) {
        self.graph.nodes.push(Node {
            name: name.to_string(),
            kind: NodeKind::Trait,
            module_path: self.module_path.clone(),
        });
    }

    pub fn add_implements(&mut self, struct_name: &str, trait_name: &str) {
        self.push_edge(struct_name, trait_name, Relation::Implements);
    }

    /// Records a struct field of type `expr` as a composition edge from
    /// `owner`. Compound and trait-wrapper types are expanded into
    /// synthetic nodes + composition edges on the fly.
    pub fn add_field_type(&mut self, owner: &str, expr: &TypeExpr) {
        self.record_type(owner, Relation::Composition, expr);
    }

    fn record_type(&mut self, from: &str, relation: Relation, expr: &TypeExpr) {
        let expr = expr.resolve_refs();

        if let Some(traits) = expr.trait_object_names() {
            for trait_name in traits {
                self.push_edge(from, &trait_name, relation.clone());
            }
            return;
        }

        let target = expr.type_name();
        self.push_edge(from, &target, relation);

        if expr.is_compound() {
            self.synthesize(expr);
        }
    }

    /// Materializes a synthetic node for a compound type (idempotent per
    /// name) and recursively records composition edges to its children.
    /// For concrete generics (e.g. `Vec<String>`), also emits the generic
    /// base node (`Vec<T>`) and a `Specializes` edge.
    fn synthesize(&mut self, expr: &TypeExpr) {
        let name = expr.type_name();
        if self.has_node(&name) {
            return;
        }

        self.graph.nodes.push(Node {
            name: name.clone(),
            kind: NodeKind::Synthetic {
                params: expr.stereotype_params(),
            },
            module_path: self.module_path.clone(),
        });

        if let TypeExpr::Generic { base, args } = expr {
            if !args.is_empty() && !is_placeholder_base(args) {
                let base_name = format!("{}<T>", base);
                self.ensure_generic_base(base, &base_name);
                self.push_edge(&name, &base_name, Relation::Specializes);
            }
        }

        for child in expr.children() {
            match child {
                TypeExpr::DynTrait(traits) | TypeExpr::ImplTrait(traits) => {
                    for t in traits {
                        self.push_edge(&name, t, Relation::Composition);
                    }
                }
                other => {
                    self.push_edge(&name, &other.type_name(), Relation::Composition);
                    if other.is_compound() {
                        self.synthesize(other);
                    }
                }
            }
        }
    }

    fn ensure_generic_base(&mut self, base: &str, base_name: &str) {
        if self.has_node(base_name) {
            return;
        }
        let module_path = if is_std_name(base) {
            vec!["std".to_string()]
        } else {
            Vec::new()
        };
        self.graph.nodes.push(Node {
            name: base_name.to_string(),
            kind: NodeKind::Synthetic {
                params: Some("T".to_string()),
            },
            module_path,
        });
    }

    fn has_node(&self, name: &str) -> bool {
        self.graph.nodes.iter().any(|n| n.name == name)
    }

    fn push_edge(&mut self, from: &str, to: &str, relation: Relation) {
        self.graph.edges.push(Edge {
            from: from.to_string(),
            to: to.to_string(),
            relation,
        });
    }
}

fn is_placeholder_base(args: &[TypeExpr]) -> bool {
    args.len() == 1 && matches!(&args[0], TypeExpr::Simple(s) if s == "T")
}
