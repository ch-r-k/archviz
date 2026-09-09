use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::error::ArchError;

use super::source_file::SourceFile;

pub struct ProjectLoader {
    root: PathBuf,
}

impl ProjectLoader {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn load(&self) -> Result<Vec<SourceFile>, ArchError> {
        let mut files = Vec::new();

        for entry in WalkDir::new(&self.root) {
            let entry = entry.map_err(|e| ArchError::Io(e.into()))?;

            if !entry.file_type().is_file() {
                continue;
            }

            if entry.path().extension().and_then(|s| s.to_str()) != Some("rs") {
                continue;
            }

            let path = entry.path().to_path_buf();
            let source = std::fs::read_to_string(&path)?;

            let module_path = compute_module(&self.root, &path);

            files.push(SourceFile {
                path,
                module_path,
                source,
            });
        }

        Ok(files)
    }
}

fn compute_module(root: &Path, file: &Path) -> Vec<String> {
    let rel = file.strip_prefix(root).unwrap();

    let mut parts: Vec<String> = Vec::new();
    for s in rel.components() {
        parts.push(s.as_os_str().to_string_lossy().to_string());
    }

    if let Some(last) = parts.last_mut() {
        if last == "mod.rs" || last == "lib.rs" || last == "main.rs" {
            parts.pop();
        } else if last.ends_with(".rs") {
            *last = last.trim_end_matches(".rs").to_string();
        }
    }

    parts
}
