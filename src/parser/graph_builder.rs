//! Turns a stream of high-level "the source contains …" observations
//! (`add_struct`, `add_trait`, `add_enum`, `add_implements`, `add_field_type`)
//! into nodes and edges in the shared [`Graph`], including the on-the-fly
//! expansion of compound and trait-wrapper types into synthetic nodes.
//!
//! Knows nothing about `syn` — the AST-facing side lives in
//! [`crate::parser::visitor`].
//!
//! **Naming:** source-level nodes (`add_struct` / `add_trait` /
//! `add_enum`) get a fully-qualified [`NodeId`] so two `Foo`s in
//! different modules stay distinct. Synthetic nodes for compound types
//! and edge targets use a bare `NodeId` — a later resolution pass on the
//! whole graph promotes them to FQ ids when unambiguous.

use crate::model::node::NodeId;
use crate::model::origin::{is_std_name, looks_like_type_param};
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
        self.add_typed_node(name, NodeKind::Struct);
    }

    pub fn add_trait(&mut self, name: &str) {
        self.add_typed_node(name, NodeKind::Trait);
    }

    pub fn add_enum(&mut self, name: &str) {
        self.add_typed_node(name, NodeKind::Enum);
    }

    fn add_typed_node(&mut self, name: &str, kind: NodeKind) {
        self.graph.add_node(Node {
            id: NodeId::from_parts(self.module_path, name),
            display_name: name.to_string(),
            kind,
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
        if looks_like_type_param(&target) {
            return;
        }
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
        let id = NodeId::bare(&name);
        if self.graph.has_node(&id) {
            return;
        }

        self.graph.push_node(Node {
            id: id.clone(),
            display_name: name.clone(),
            kind: NodeKind::Synthetic {
                params: expr.stereotype_params(),
            },
            // Compound types are placed in the module where the
            // referencing (owner) type lives, so e.g. `Vec<Node>` used
            // by `Graph` renders inside the `graph` package alongside
            // its owner. Compound types are shared/deduplicated by id,
            // so the first module to reference a given compound type
            // wins.
            module_path: self.module_path.clone(),
        });

        if let TypeExpr::Generic { base, args } = expr {
            if !args.is_empty() && !is_placeholder_base(args) {
                let base_name = format!("{}<T>", base);
                self.ensure_generic_base(base, &base_name);
                self.push_synthetic_edge(&id, &base_name, Relation::Specializes);
            }
        }

        for child in expr.children() {
            match child {
                TypeExpr::DynTrait(traits) | TypeExpr::ImplTrait(traits) => {
                    for t in traits {
                        self.push_synthetic_edge(&id, t, Relation::Composition);
                    }
                }
                other => {
                    let child_name = other.type_name();
                    if looks_like_type_param(&child_name) {
                        continue;
                    }
                    self.push_synthetic_edge(&id, &child_name, Relation::Composition);
                    if other.is_compound() {
                        self.synthesize(other);
                    }
                }
            }
        }
    }

    fn ensure_generic_base(&mut self, base: &str, base_name: &str) {
        let id = NodeId::bare(base_name);
        if self.graph.has_node(&id) {
            return;
        }
        let module_path = if is_std_name(base) {
            vec!["std".to_string()]
        } else {
            Vec::new()
        };
        self.graph.push_node(Node {
            id,
            display_name: base_name.to_string(),
            kind: NodeKind::Synthetic {
                params: Some("T".to_string()),
            },
            module_path,
        });
    }

    /// Emits an edge from a source-level owner (fully-qualified by the
    /// current module path) to a target referenced by its bare name — the
    /// bare target will be resolved to a real [`NodeId`] later by
    /// [`crate::pipeline::resolve_edge_targets`].
    fn push_edge(&mut self, from: &str, to: &str, relation: Relation) {
        self.graph.edges.push(Edge {
            from: NodeId::from_parts(self.module_path, from),
            to: NodeId::bare(to),
            relation,
        });
    }

    /// Emits an edge whose source is already a resolved (synthetic)
    /// [`NodeId`]; the target stays bare until resolution.
    fn push_synthetic_edge(&mut self, from: &NodeId, to: &str, relation: Relation) {
        self.graph.edges.push(Edge {
            from: from.clone(),
            to: NodeId::bare(to),
            relation,
        });
    }
}

fn is_placeholder_base(args: &[TypeExpr]) -> bool {
    args.len() == 1 && matches!(&args[0], TypeExpr::Simple(s) if s == "T")
}
