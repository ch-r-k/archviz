pub mod metadata;

pub use metadata::CrateIndex;

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
}
