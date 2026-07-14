//! Pure `syn::Type` → [`TypeExpr`] conversion. Decoupled from the
//! graph-building side ([`crate::parser::graph_builder`]) so each layer
//! has one job.

use crate::parser::type_expr::TypeExpr;

pub struct TypeExtractor;

impl TypeExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Turns a `syn::Type` into a [`TypeExpr`], recursively descending
    /// through references, generic arguments, arrays/slices, tuples, and
    /// `dyn`/`impl Trait` bounds. Returns `None` for `syn::Type` variants
    /// archviz doesn't model (function pointers, macros, etc.).
    pub fn extract(&self, ty: &syn::Type) -> Option<TypeExpr> {
        match ty {
            syn::Type::Path(tp) => self.extract_path(tp),

            syn::Type::Array(a) => self.extract(&a.elem).map(|i| TypeExpr::Array(Box::new(i))),
            syn::Type::Slice(s) => self.extract(&s.elem).map(|i| TypeExpr::Slice(Box::new(i))),
            syn::Type::Reference(r) => {
                self.extract(&r.elem).map(|i| TypeExpr::Reference(Box::new(i)))
            }

            syn::Type::Tuple(t) => Some(TypeExpr::Tuple(self.extract_many(&t.elems))),

            syn::Type::ImplTrait(i) => Some(TypeExpr::ImplTrait(trait_bounds(&i.bounds))),
            syn::Type::TraitObject(t) => Some(TypeExpr::DynTrait(trait_bounds(&t.bounds))),

            syn::Type::Group(g) => self.extract(&g.elem),

            _ => None,
        }
    }

    fn extract_path(&self, tp: &syn::TypePath) -> Option<TypeExpr> {
        let seg = tp.path.segments.last()?;
        let base = seg.ident.to_string();

        match &seg.arguments {
            syn::PathArguments::None => Some(TypeExpr::Simple(base)),

            syn::PathArguments::AngleBracketed(args) => {
                let mut inner: Vec<TypeExpr> = Vec::new();
                for arg in &args.args {
                    if let syn::GenericArgument::Type(ty) = arg {
                        if let Some(t) = self.extract(ty) {
                            inner.push(t);
                        }
                    }
                }
                Some(TypeExpr::Generic { base, args: inner })
            }

            _ => Some(TypeExpr::Simple(base)),
        }
    }

    fn extract_many(
        &self,
        types: &syn::punctuated::Punctuated<syn::Type, syn::Token![,]>,
    ) -> Vec<TypeExpr> {
        let mut out: Vec<TypeExpr> = Vec::new();
        for t in types {
            if let Some(e) = self.extract(t) {
                out.push(e);
            }
        }
        out
    }
}

impl Default for TypeExtractor {
    fn default() -> Self {
        Self::new()
    }
}

fn trait_bounds(
    bounds: &syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>,
) -> Vec<String> {
    let mut traits: Vec<String> = Vec::new();
    for b in bounds {
        if let syn::TypeParamBound::Trait(tr) = b {
            if let Some(s) = tr.path.segments.last() {
                traits.push(s.ident.to_string());
            }
        }
    }
    traits
}
