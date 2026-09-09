//! Module-path glob patterns used by [`crate::filter::spec::FilterSpec`].
//!
//! Syntax:
//! - `a::b` — exact match on module `a::b` (does *not* match `a::b::c`).
//! - `a::b::*` — any direct child of `a::b`.
//! - `a::b::**` — `a::b` itself and any descendant module.
//! - `**::internal` — any module ending in a segment called `internal`.
//!
//! `*` matches exactly one path segment. `**` matches zero or more
//! segments. No other wildcards are supported.

use crate::error::ArchError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    Literal(String),
    Star,
    DoubleStar,
}

#[derive(Debug, Clone)]
pub struct ModulePattern {
    segments: Vec<Segment>,
}

impl ModulePattern {
    /// Parses `"a::b::*"` into a [`ModulePattern`]. Empty strings and
    /// segments mixing `*` with other characters (e.g. `foo*`) are
    /// rejected.
    pub fn parse(pattern: &str) -> Result<Self, ArchError> {
        if pattern.is_empty() {
            return Err(ArchError::InvalidPattern(
                "empty module pattern".to_string(),
            ));
        }
        let mut segments: Vec<Segment> = Vec::new();
        for raw in pattern.split("::") {
            if raw.is_empty() {
                return Err(ArchError::InvalidPattern(format!(
                    "empty segment in pattern `{pattern}`"
                )));
            }
            if raw == "*" {
                segments.push(Segment::Star);
            } else if raw == "**" {
                segments.push(Segment::DoubleStar);
            } else if raw.contains('*') {
                return Err(ArchError::InvalidPattern(format!(
                    "partial wildcards not supported in `{raw}` \
                     (use `*` or `**` on their own)"
                )));
            } else {
                segments.push(Segment::Literal(raw.to_string()));
            }
        }
        Ok(Self { segments })
    }

    /// True when `module_path` matches this pattern in full.
    pub fn matches(&self, module_path: &[String]) -> bool {
        match_segments(&self.segments, module_path)
    }

    /// Returns the module path this pattern names, if it is purely
    /// literal (no `*` / `**` segments). Used to synthesize an empty
    /// placeholder package for a collapse pattern that matches no real
    /// node in the graph.
    pub fn literal_path(&self) -> Option<Vec<String>> {
        let mut out: Vec<String> = Vec::with_capacity(self.segments.len());
        for seg in &self.segments {
            match seg {
                Segment::Literal(s) => out.push(s.clone()),
                _ => return None,
            }
        }
        Some(out)
    }
}

/// Two-pointer glob-style matcher: literals and `*` consume exactly one
/// segment, `**` consumes zero or more segments and backtracks if the
/// tail fails.
fn match_segments(pattern: &[Segment], path: &[String]) -> bool {
    let mut pi = 0usize;
    let mut si = 0usize;
    let mut star_pi: Option<usize> = None;
    let mut star_si: usize = 0;

    while si < path.len() {
        let mut matched = false;
        if pi < pattern.len() {
            match &pattern[pi] {
                Segment::Literal(name) => {
                    if &path[si] == name {
                        pi += 1;
                        si += 1;
                        matched = true;
                    }
                }
                Segment::Star => {
                    pi += 1;
                    si += 1;
                    matched = true;
                }
                Segment::DoubleStar => {
                    // Try matching zero segments first; remember the
                    // `**` position so we can backtrack and consume one
                    // more path segment if the tail fails.
                    star_pi = Some(pi);
                    star_si = si;
                    pi += 1;
                    matched = true;
                }
            }
        }
        if matched {
            continue;
        }
        if let Some(sp) = star_pi {
            pi = sp + 1;
            star_si += 1;
            si = star_si;
            continue;
        }
        return false;
    }

    // Trailing `**` may still match the remainder (zero segments).
    while pi < pattern.len() && pattern[pi] == Segment::DoubleStar {
        pi += 1;
    }
    pi == pattern.len()
}
