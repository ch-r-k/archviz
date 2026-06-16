use syn::visit::Visit;

use crate::model::{Edge, Graph, ModulePath, Node, NodeKind, Relation};
use crate::parser::ast_parser::ParsedModule;

pub struct GraphVisitor<'a> {
    pub graph: &'a mut Graph,
    pub module: &'a ModulePath,
    current_struct: Option<String>,
}

impl<'a> GraphVisitor<'a> {
    pub fn new(graph: &'a mut Graph, module: &'a ModulePath) -> Self {
        Self {
            graph,
            module,
            current_struct: None,
        }
    }

    pub fn visit_module(mut self, module: &ParsedModule) {
        self.visit_file(&module.ast);
    }
}

impl<'ast> Visit<'ast> for GraphVisitor<'_> {
    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        let name = node.ident.to_string();

        self.graph.nodes.push(Node {
            name: name.clone(),
            kind: NodeKind::Struct,
            module: self.module.clone(),
        });

        self.current_struct = Some(name);

        syn::visit::visit_item_struct(self, node);

        self.current_struct = None;
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        self.graph.nodes.push(Node {
            name: node.ident.to_string(),
            kind: NodeKind::Trait,
            module: self.module.clone(),
        });

        syn::visit::visit_item_trait(self, node);
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if let Some((_, trait_path, _)) = &node.trait_ {
            let trait_name = trait_path.segments.last().unwrap().ident.to_string();

            if let syn::Type::Path(tp) = &*node.self_ty {
                let struct_name = tp.path.segments.last().unwrap().ident.to_string();

                self.graph.edges.push(Edge {
                    from: struct_name,
                    to: trait_name,
                    relation: Relation::Implements,
                });
            }
        }

        syn::visit::visit_item_impl(self, node);
    }

    fn visit_field(&mut self, field: &'ast syn::Field) {
        let Some(owner) = &self.current_struct else {
            return;
        };

        if let syn::Type::Path(tp) = &field.ty {
            if let Some(seg) = tp.path.segments.last() {
                self.graph.edges.push(Edge {
                    from: owner.clone(),
                    to: seg.ident.to_string(),
                    relation: Relation::Composition,
                });
            }
        }

        syn::visit::visit_field(self, field);
    }
}
