pub mod metadata;
pub mod module_index;
pub mod scope;

pub use metadata::CrateIndex;
pub use module_index::ModuleIndex;

/// Context shared with enrichment passes after all source files have been
/// loaded and (optionally) a `cargo metadata` snapshot has been taken.
///
/// Kept intentionally small: passes that need it borrow what they use, and
/// passes that don't can ignore it entirely.
pub struct ResolutionContext<'a> {
    /// Indexed external crate names, or `None` when no Cargo project could
    /// be discovered (detached mode) — passes fall back to name-based
    /// heuristics in that case.
    pub crate_index: Option<&'a CrateIndex>,
    /// Lexical index of definitions and imports across the project.
    pub module_index: &'a ModuleIndex,
}
