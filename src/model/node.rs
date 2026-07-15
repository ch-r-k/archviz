#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum NodeKind {
    Struct,
    Trait,
    Enum,
    /// `impl` block (with or without a trait). Reserved for future use — the
    /// current parser emits [`Relation::Implements`] edges rather than nodes.
    Impl {
        trait_name: Option<String>,
    },
    /// `type Foo = ...` alias. Reserved for future use — not yet emitted.
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
