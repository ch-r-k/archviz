use std::path::PathBuf;

pub struct SourceFile {
    pub path: PathBuf,
    pub module_path: Vec<String>,
    pub source: String,
}
