#[derive(Debug, Clone)]
pub enum Relation {
    Composition,
    Implements,
    Specializes,  // Concrete generic type inherits from generic base (e.g., Vec<String> specializes Vec<T>)
}

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
    pub fn base_name(&self) -> &str {
        match self {
            TypeExpr::Simple(name) => name,
            TypeExpr::Generic { base, .. } => base,
            TypeExpr::Reference(inner) => inner.base_name(),
            TypeExpr::Slice(inner) => inner.base_name(),
            TypeExpr::Array(inner) => inner.base_name(),
            TypeExpr::DynTrait(traits) | TypeExpr::ImplTrait(traits) => &traits[0],
            TypeExpr::Tuple(_) => "tuple",
            TypeExpr::Unknown => "_",
        }
    }

    /// Strips top-level `&`/`&mut` references to reach the underlying type.
    ///
    /// The single source of truth for this traversal — previously duplicated
    /// in `enricher/type_expander.rs` and `renderer/plantuml.rs`.
    pub fn resolve_refs(&self) -> &TypeExpr {
        match self {
            TypeExpr::Reference(inner) => inner.resolve_refs(),
            other => other,
        }
    }

    /// Returns the display name used as a diagram node/edge identifier
    /// (e.g. `Vec<String>`, `[u8]`, `(A, B)`, `dyn Trait`).
    ///
    /// The single source of truth for this traversal — previously duplicated
    /// in `enricher/type_expander.rs` and `renderer/plantuml.rs`.
    pub fn type_name(&self) -> String {
        match self {
            TypeExpr::Simple(name) => name.clone(),
            TypeExpr::Generic { base, args } => {
                if args.is_empty() {
                    base.clone()
                } else {
                    let rendered = args
                        .iter()
                        .map(|a| a.resolve_refs().type_name())
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{}<{}>", base, rendered)
                }
            }
            TypeExpr::Reference(inner) => inner.type_name(),
            TypeExpr::Slice(inner) => format!("[{}]", inner.resolve_refs().type_name()),
            TypeExpr::Array(inner) => format!("[{}; N]", inner.resolve_refs().type_name()),
            TypeExpr::Tuple(items) => {
                let rendered = items
                    .iter()
                    .map(|i| i.resolve_refs().type_name())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({})", rendered)
            }
            TypeExpr::DynTrait(traits) => format!("dyn {}", traits.join(" + ")),
            TypeExpr::ImplTrait(traits) => format!("impl {}", traits.join(" + ")),
            TypeExpr::Unknown => "_".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub from: String,
    pub to: TypeExpr,
    pub relation: Relation,
}
