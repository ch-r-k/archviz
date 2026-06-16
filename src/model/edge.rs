#[derive(Debug, Clone)]
pub enum Relation {
    Composition,
    Implements,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub relation: Relation,
}
