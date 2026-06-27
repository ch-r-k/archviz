#[derive(Debug, Clone)]
pub enum Relation {
    Composition,
    Implements,
}

#[derive(Debug, Clone)]
pub enum TypeExpr {
    Simple(String),
    Generic {
        base: String,
        args: Vec<TypeExpr>,
    },
    Reference(Box<TypeExpr>),
    Slice(Box<TypeExpr>),
    Array(Box<TypeExpr>),
    Tuple(Vec<TypeExpr>),

    DynTrait(Vec<String>),
    ImplTrait(Vec<String>),

    Unknown,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub from: String,
    pub to: TypeExpr,
    pub relation: Relation,
}
