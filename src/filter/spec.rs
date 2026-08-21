//! Composite filter over module paths.
//!
//! Precedence per node (applied in order):
//! 1. If any `exclude` pattern matches → [`Decision::Drop`].
//! 2. Otherwise, if `include` is non-empty and none match → [`Decision::Drop`].
//! 3. Otherwise, if any `collapse` pattern matches → [`Decision::Collapse`].
//! 4. Otherwise → [`Decision::Keep`].

use crate::filter::pattern::ModulePattern;

/// Outcome of applying a [`FilterSpec`] to a single module path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Keep,
    Drop,
    /// Node lives under a `--collapse` pattern; the caller redirects its
    /// edges to a synthetic package node for the matching module.
    Collapse,
}

#[derive(Debug, Clone, Default)]
pub struct FilterSpec {
    pub include: Vec<ModulePattern>,
    pub exclude: Vec<ModulePattern>,
    pub collapse: Vec<ModulePattern>,
    /// Maximum module depth to render. `Some(N)` means: any node whose
    /// `module_path.len() > N` is collapsed into its ancestor at
    /// depth `N`. `None` disables depth-based collapse.
    pub collapse_depth: Option<usize>,
}

impl FilterSpec {
    pub fn is_empty(&self) -> bool {
        self.include.is_empty()
            && self.exclude.is_empty()
            && self.collapse.is_empty()
            && self.collapse_depth.is_none()
    }

    /// Include/exclude decision only. Collapse is handled by
    /// [`crate::enricher::module_filter::ModuleFilter`] because it
    /// needs to identify the collapse *root* per node (not just
    /// yes/no), and applies subtree semantics that `ModulePattern`
    /// alone doesn't express.
    pub fn decides(&self, module_path: &[String]) -> Decision {
        if any_match(&self.exclude, module_path) {
            return Decision::Drop;
        }
        if !self.include.is_empty() && !any_match(&self.include, module_path) {
            return Decision::Drop;
        }
        Decision::Keep
    }
}

fn any_match(patterns: &[ModulePattern], module_path: &[String]) -> bool {
    for p in patterns {
        if p.matches(module_path) {
            return true;
        }
    }
    false
}
