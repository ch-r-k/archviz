use super::ModulePath;

#[derive(Debug, Clone)]
pub enum NodeKind {
    Struct,
    Trait,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub kind: NodeKind,
    pub module: ModulePath,
}
