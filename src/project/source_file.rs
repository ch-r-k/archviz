use crate::model::ModulePath;
use std::path::PathBuf;

pub struct SourceFile {
    pub path: PathBuf,
    pub module: ModulePath,
    pub source: String,
}
