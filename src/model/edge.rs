#[derive(Debug, Clone)]
pub enum Relation {
    Composition,
    Implements,
    /// Concrete generic type inherits from its generic base
    /// (e.g. `Vec<String>` specializes `Vec<T>`).
    Specializes,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub relation: Relation,
}
