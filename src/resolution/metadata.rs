//! Discovers the enclosing Cargo workspace for the analyzed directory and
//! extracts the set of external crate names its code can refer to.
//!
//! `cargo metadata` is run with `--offline` so the analysis never requires
//! network access; it only needs `cargo` on `PATH` and a populated cache.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// The crate universe around the analyzed project: which first-path-segment
/// identifiers are external crate names (everything else is local, `std`,
/// `core`, `alloc`, or a primitive).
pub struct CrateIndex {
    external_crates: HashSet<String>,
}

impl CrateIndex {
    /// Builds an index from a precomputed set of external crate names.
    /// The production entry point is [`CrateIndex::discover`]; this is
    /// provided for tests.
    #[cfg(test)]
    pub fn new(external_crates: HashSet<String>) -> CrateIndex {
        CrateIndex { external_crates }
    }

    /// Builds a [`CrateIndex`] by locating the nearest `Cargo.toml` above
    /// `root` and running `cargo metadata --offline`. Returns `None` when no
    /// manifest is found or metadata cannot be produced, so callers can fall
    /// back to name-based resolution.
    pub fn discover(root: &Path) -> Option<CrateIndex> {
        let manifest = find_manifest(root)?;

        let metadata = match cargo_metadata::MetadataCommand::new()
            .manifest_path(&manifest)
            .other_options(["--offline".to_string()])
            .exec()
        {
            Ok(metadata) => metadata,
            Err(err) => {
                eprintln!(
                    "archviz: cargo metadata failed ({}); \
                     falling back to name-based type classification",
                    err
                );
                return None;
            }
        };

        let mut external_crates: HashSet<String> = HashSet::new();

        // Every package pulled from a registry/git source is external to the
        // analyzed code base.
        for package in &metadata.packages {
            if package.source.is_some() {
                external_crates.insert(package.name.to_string());
            }
        }

        // Dependency renames change the identifier used in source
        // (`use foo as bar` via Cargo.toml `rename`), so register them too,
        // but only when the renamed dependency is an external package.
        for package in &metadata.packages {
            for dep in &package.dependencies {
                if let Some(rename) = &dep.rename {
                    if external_crates.contains(&dep.name) {
                        external_crates.insert(rename.clone());
                    }
                }
            }
        }

        Some(CrateIndex { external_crates })
    }

    pub fn is_external(&self, crate_name: &str) -> bool {
        self.external_crates.contains(crate_name)
    }
}

/// Walks up from `root` looking for the nearest `Cargo.toml`.
fn find_manifest(root: &Path) -> Option<PathBuf> {
    let mut dir: Option<&Path> = if root.is_dir() {
        Some(root)
    } else {
        root.parent()
    };

    while let Some(current) = dir {
        let candidate = current.join("Cargo.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        dir = current.parent();
    }

    None
}
