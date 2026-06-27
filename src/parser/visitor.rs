use syn::visit::Visit;

use crate::model::{Edge, Graph, Node, NodeKind, Relation, TypeExpr};
use crate::parser::ast_parser::ParsedModule;

pub struct GraphVisitor<'a> {
    pub graph: &'a mut Graph,
    pub module_path: &'a Vec<String>,
    current_struct: Option<String>,
}

impl<'a> GraphVisitor<'a> {
    pub fn new(graph: &'a mut Graph, module_path: &'a Vec<String>) -> Self {
        Self {
            graph,
            module_path,
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
            module_path: self.module_path.clone(),
        });

        self.current_struct = Some(name);

        syn::visit::visit_item_struct(self, node);

        self.current_struct = None;
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        self.graph.nodes.push(Node {
            name: node.ident.to_string(),
            kind: NodeKind::Trait,
            module_path: self.module_path.clone(),
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
                    to: TypeExpr::Simple(trait_name),
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

        if let Some(type_expr) = extract_type_expr(&field.ty) {
            self.graph.edges.push(Edge {
                from: owner.clone(),
                to: type_expr,
                relation: Relation::Composition,
            });
        }

        syn::visit::visit_field(self, field);
    }
}

/// Recursively unwraps wrapper types (`Box`, `Vec`, `Option`, `Arc`, `Rc`,
/// `RefCell`, `Mutex`) to find the meaningful inner type. Also handles
/// `dyn Trait` objects by returning the first trait bound name.
fn extract_type_expr(ty: &syn::Type) -> Option<TypeExpr> {
match ty {
    syn::Type::Path(tp) => {
        let seg = tp.path.segments.last()?;
        let base = seg.ident.to_string();

        match &seg.arguments {
            syn::PathArguments::None => {
                Some(TypeExpr::Simple(base))
            }

            syn::PathArguments::AngleBracketed(args) => {
                let mut inner = vec![];

                for arg in &args.args {
                    if let syn::GenericArgument::Type(inner_ty) = arg {
                        if let Some(t) = extract_type_expr(inner_ty) {
                            inner.push(t);
                        }
                    }
                }

                Some(TypeExpr::Generic { base, args: inner })
            }

            _ => Some(TypeExpr::Simple(base)),
        }
    }

    syn::Type::Array(type_array) => {
        extract_type_expr(&type_array.elem)
            .map(|inner| TypeExpr::Array(Box::new(inner)))
    }

    syn::Type::Slice(type_slice) => {
        extract_type_expr(&type_slice.elem)
            .map(|inner| TypeExpr::Slice(Box::new(inner)))
    }

    syn::Type::Reference(type_reference) => {
        extract_type_expr(&type_reference.elem)
            .map(|inner| TypeExpr::Reference(Box::new(inner)))
    }

    syn::Type::Tuple(type_tuple) => {
        let elems = type_tuple
            .elems
            .iter()
            .filter_map(extract_type_expr)
            .collect();

        Some(TypeExpr::Tuple(elems))
    }

    syn::Type::ImplTrait(type_impl_trait) => {
        let traits = type_impl_trait
            .bounds
            .iter()
            .filter_map(|b| {
                if let syn::TypeParamBound::Trait(tr) = b {
                    tr.path.segments
                        .last()
                        .map(|s| s.ident.to_string())
                } else {
                    None
                }
            })
            .collect();

        Some(TypeExpr::ImplTrait(traits))
    }

    syn::Type::TraitObject(type_trait_object) => {
        let traits = type_trait_object
            .bounds
            .iter()
            .filter_map(|b| {
                if let syn::TypeParamBound::Trait(tr) = b {
                    tr.path.segments
                        .last()
                        .map(|s| s.ident.to_string())
                } else {
                    None
                }
            })
            .collect();

        Some(TypeExpr::DynTrait(traits))
    }

    syn::Type::Group(type_group) => {
        // just unwrap parentheses grouping
        extract_type_expr(&type_group.elem)
    }

    _ => None,
}
}
