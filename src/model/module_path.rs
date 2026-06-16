#[derive(Debug, Clone, Default)]
pub struct ModulePath(pub Vec<String>);

impl ModulePath {
    pub fn join(&self) -> String {
        self.0.join("::")
    }
}
