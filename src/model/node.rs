#[derive(Debug, Clone)]
pub enum NodeKind {
    Struct,
    Trait,
    Enum,
    Impl { trait_name: Option<String> },
    TypeAlias,
    /// A synthetic node generated for a compound type (e.g. `Vec<String>`)
    /// that has no corresponding source-level definition.
    Synthetic,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub kind: NodeKind,
    pub module_path: Vec<String>
}
