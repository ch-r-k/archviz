#[derive(Debug, Clone)]
pub enum Relation {
    Composition,
    Implements,
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
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub from: String,
    pub to: TypeExpr,
    pub relation: Relation,
}
