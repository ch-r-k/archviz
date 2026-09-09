pub mod module_filter;
pub mod pattern;
pub mod spec;
pub mod traits;
mod tests;

pub use module_filter::ModuleFilter;
pub use pattern::ModulePattern;
pub use spec::FilterSpec;
pub use traits::Filter;

use crate::error::ArchError;

/// Plain-data configuration passed from the CLI (or any front-end)
/// to the pipeline. The pipeline turns this into a concrete
/// [`ModuleFilter`] internally — callers don't need to know about
/// [`ModulePattern`] or [`FilterSpec`].
#[derive(Debug, Default, Clone)]
pub struct FilterOptions {
    pub includes: Vec<String>,
    pub excludes: Vec<String>,
    pub collapses: Vec<String>,
    pub collapse_depth: Option<usize>,
}

impl FilterOptions {
    pub fn is_empty(&self) -> bool {
        self.includes.is_empty()
            && self.excludes.is_empty()
            && self.collapses.is_empty()
            && self.collapse_depth.is_none()
    }

    /// Parse the raw pattern strings and build a `ModuleFilter`.
    pub fn build(self) -> Result<ModuleFilter, ArchError> {
        let mut include = Vec::with_capacity(self.includes.len());
        for pat in self.includes {
            include.push(ModulePattern::parse(&pat)?);
        }
        let mut exclude = Vec::with_capacity(self.excludes.len());
        for pat in self.excludes {
            exclude.push(ModulePattern::parse(&pat)?);
        }
        let mut collapse = Vec::with_capacity(self.collapses.len());
        for pat in self.collapses {
            collapse.push(ModulePattern::parse(&pat)?);
        }
        let spec = FilterSpec {
            include,
            exclude,
            collapse,
            collapse_depth: self.collapse_depth,
        };
        Ok(ModuleFilter::new(spec))
    }
}
