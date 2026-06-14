#[derive(Debug, PartialEq, Eq)]
pub enum NodeKind {
    Struct,
    Trait,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Node {
    pub(crate) name: String,
    pub(crate) kind: NodeKind,
    pub(crate) module: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Relation {
    Implements,
    Composition,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Edge {
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) relation: Relation,
}

#[derive(Default, PartialEq, Eq)]
pub struct Graph {
    pub(crate) nodes: Vec<Node>,
    pub(crate) edges: Vec<Edge>,
}
