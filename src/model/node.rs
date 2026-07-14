#[derive(Debug, Clone)]
pub enum NodeKind {
    Struct,
    Trait,
    Enum,
    Impl {
        trait_name: Option<String>,
    },
    TypeAlias,
    /// A synthetic node generated for a compound type (e.g. `Vec<String>`)
    /// that has no corresponding source-level definition. `params` carries
    /// the pre-rendered stereotype content (e.g. `"String"` for `Vec<String>`,
    /// `"A, B"` for `(A, B)`, `"T"` for the generic base `Vec<T>`); `None`
    /// renders as a plain class.
    Synthetic {
        params: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub kind: NodeKind,
    pub module_path: Vec<String>,
}
