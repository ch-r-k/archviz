use syn::visit::Visit;

use crate::model::*;

pub struct Extractor {
    pub(crate) graph: Graph,
    current_struct: Option<String>,
    module_stack: Vec<String>,
}

impl Extractor {
    pub fn new() -> Self {
        Self {
            graph: Graph::default(),
            current_struct: None,
            module_stack: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for Extractor {
    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        let name = node.ident.to_string();

        self.graph.nodes.push(Node {
            name: name.clone(),
            kind: NodeKind::Struct,
            module: self.module_stack.clone(),
        });

        self.current_struct = Some(name);

        syn::visit::visit_item_struct(self, node);

        self.current_struct = None;
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        self.graph.nodes.push(Node {
            name: node.ident.to_string(),
            kind: NodeKind::Trait,
            module: self.module_stack.clone(),
        });

        syn::visit::visit_item_trait(self, node);
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if let Some((_, trait_path, _)) = &node.trait_ {
            let trait_name = trait_path.segments.last().unwrap().ident.to_string();

            if let syn::Type::Path(type_path) = &*node.self_ty {
                let struct_name = type_path.path.segments.last().unwrap().ident.to_string();

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

        if let syn::Type::Path(type_path) = &field.ty {
            if let Some(segment) = type_path.path.segments.last() {
                let target = segment.ident.to_string();

                self.graph.edges.push(Edge {
                    from: owner.clone(),
                    to: target,
                    relation: Relation::Composition,
                });
            }
        }

        syn::visit::visit_field(self, field);
    }

    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        self.module_stack.push(node.ident.to_string());

        syn::visit::visit_item_mod(self, node);

        self.module_stack.pop();
    }
}
