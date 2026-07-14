//! Internal parser representation of a Rust type. Consumed by
//! [`GraphVisitor`] to expand fields into graph nodes and edges; the rest of
//! the pipeline sees only stringly-typed [`crate::model::Edge`]s.

#[derive(Debug, Clone)]
pub enum TypeExpr {
    Simple(String),
    Generic { base: String, args: Vec<TypeExpr> },
    Reference(Box<TypeExpr>),
    Slice(Box<TypeExpr>),
    Array(Box<TypeExpr>),
    Tuple(Vec<TypeExpr>),

    DynTrait(Vec<String>),
    ImplTrait(Vec<String>),

    Unknown,
}

impl TypeExpr {
    /// Strips top-level `&`/`&mut` references to reach the underlying type.
    pub fn resolve_refs(&self) -> &TypeExpr {
        match self {
            TypeExpr::Reference(inner) => inner.resolve_refs(),
            other => other,
        }
    }

    /// Returns the display name used as a diagram node/edge identifier
    /// (e.g. `Vec<String>`, `[u8]`, `(A, B)`, `dyn Trait`).
    pub fn type_name(&self) -> String {
        match self {
            TypeExpr::Simple(name) => name.clone(),
            TypeExpr::Generic { base, args } => {
                if args.is_empty() {
                    base.clone()
                } else {
                    format!("{}<{}>", base, join_type_names(args))
                }
            }
            TypeExpr::Reference(inner) => inner.type_name(),
            TypeExpr::Slice(inner) => format!("[{}]", inner.resolve_refs().type_name()),
            TypeExpr::Array(inner) => format!("[{}; N]", inner.resolve_refs().type_name()),
            TypeExpr::Tuple(items) => format!("({})", join_type_names(items)),
            TypeExpr::DynTrait(traits) => format!("dyn {}", traits.join(" + ")),
            TypeExpr::ImplTrait(traits) => format!("impl {}", traits.join(" + ")),
            TypeExpr::Unknown => "_".to_string(),
        }
    }

    /// Direct sub-expressions of a compound type, with top-level references
    /// stripped. Returns empty for leaf variants.
    pub fn children(&self) -> Vec<&TypeExpr> {
        match self {
            TypeExpr::Generic { args, .. } => args.iter().map(|a| a.resolve_refs()).collect(),
            TypeExpr::Slice(inner) | TypeExpr::Array(inner) => vec![inner.resolve_refs()],
            TypeExpr::Tuple(items) => items.iter().map(|i| i.resolve_refs()).collect(),
            TypeExpr::Reference(inner) => vec![inner.resolve_refs()],
            _ => Vec::new(),
        }
    }

    /// Trait names if this expression (possibly wrapped in `Box<...>`,
    /// `Vec<...>`, references, etc.) resolves to a `dyn Trait` / `impl Trait`.
    pub fn trait_object_names(&self) -> Option<Vec<String>> {
        match self {
            TypeExpr::DynTrait(traits) | TypeExpr::ImplTrait(traits) => Some(traits.clone()),
            TypeExpr::Reference(inner) => inner.trait_object_names(),
            TypeExpr::Generic { args, .. } => {
                for arg in args {
                    if let Some(t) = arg.resolve_refs().trait_object_names() {
                        return Some(t);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// True when the expression needs its own synthetic node (compound
    /// generics, slices, arrays, tuples).
    pub fn is_compound(&self) -> bool {
        match self {
            TypeExpr::Generic { args, .. } => !args.is_empty(),
            TypeExpr::Slice(_) | TypeExpr::Array(_) | TypeExpr::Tuple(_) => true,
            _ => false,
        }
    }

    /// Comma-joined child type names as they should appear in a PlantUML
    /// stereotype (`<String>`, `<A, B>`, `<u8>`, …).
    pub fn stereotype_params(&self) -> Option<String> {
        match self {
            TypeExpr::Generic { args, .. } if !args.is_empty() => Some(join_type_names(args)),
            TypeExpr::Slice(inner) | TypeExpr::Array(inner) => {
                Some(inner.resolve_refs().type_name())
            }
            TypeExpr::Tuple(items) => Some(join_type_names(items)),
            _ => None,
        }
    }
}

fn join_type_names(items: &[TypeExpr]) -> String {
    let mut parts: Vec<String> = Vec::with_capacity(items.len());
    for i in items {
        parts.push(i.resolve_refs().type_name());
    }
    parts.join(", ")
}
