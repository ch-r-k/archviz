//! Thin `syn::Visit` adapter: walks the AST, converts field types via
//! [`extract_type_expr`], and forwards everything to [`GraphBuilder`].

use syn::visit::Visit;

use crate::model::Graph;
use crate::parser::ast_parser::ParsedModule;
use crate::parser::graph_builder::GraphBuilder;
use crate::parser::type_extractor::TypeExtractor;

pub struct GraphVisitor<'a> {
    builder: GraphBuilder<'a>,
    extractor: TypeExtractor,
    current_struct: Option<String>,
}

impl<'a> GraphVisitor<'a> {
    pub fn new(graph: &'a mut Graph, module_path: &'a Vec<String>) -> Self {
        Self {
            builder: GraphBuilder::new(graph, module_path),
            extractor: TypeExtractor::new(),
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
        self.builder.add_struct(&name);

        self.current_struct = Some(name);
        syn::visit::visit_item_struct(self, node);
        self.current_struct = None;
    }

    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        let name = node.ident.to_string();
        self.builder.add_enum(&name);

        // Treat each variant's payload fields as composition edges from
        // the enum, mirroring how struct fields are handled.
        self.current_struct = Some(name);
        for variant in &node.variants {
            for field in &variant.fields {
                if let Some(type_expr) = self.extractor.extract(&field.ty) {
                    let owner = self.current_struct.clone().unwrap();
                    self.builder.add_field_type(&owner, &type_expr);
                }
            }
        }
        self.current_struct = None;
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        self.builder.add_trait(&node.ident.to_string());
        syn::visit::visit_item_trait(self, node);
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if let Some((_, trait_path, _)) = &node.trait_ {
            let trait_name = trait_path.segments.last().unwrap().ident.to_string();

            if let syn::Type::Path(tp) = &*node.self_ty {
                let struct_name = tp.path.segments.last().unwrap().ident.to_string();
                self.builder.add_implements(&struct_name, &trait_name);
            }
        }

        syn::visit::visit_item_impl(self, node);
    }

    fn visit_field(&mut self, field: &'ast syn::Field) {
        let Some(owner) = self.current_struct.clone() else {
            return;
        };

        if let Some(type_expr) = self.extractor.extract(&field.ty) {
            self.builder.add_field_type(&owner, &type_expr);
        }

        syn::visit::visit_field(self, field);
    }
}
