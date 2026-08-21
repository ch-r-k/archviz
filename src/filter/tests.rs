#![cfg(test)]

use crate::filter::pattern::ModulePattern;
use crate::filter::spec::{Decision, FilterSpec};

fn path(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

fn pat(s: &str) -> ModulePattern {
    ModulePattern::parse(s).expect("valid pattern")
}

#[test]
fn literal_pattern_exact_match_only() {
    let p = pat("a::b");
    assert!(p.matches(&path(&["a", "b"])));
    assert!(!p.matches(&path(&["a", "b", "c"])));
    assert!(!p.matches(&path(&["a"])));
    assert!(!p.matches(&path(&["x", "a", "b"])));
}

#[test]
fn single_star_matches_one_segment() {
    let p = pat("a::*");
    assert!(p.matches(&path(&["a", "b"])));
    assert!(p.matches(&path(&["a", "z"])));
    assert!(!p.matches(&path(&["a"])));
    assert!(!p.matches(&path(&["a", "b", "c"])));
}

#[test]
fn double_star_matches_self_and_descendants() {
    let p = pat("a::b::**");
    assert!(p.matches(&path(&["a", "b"])));
    assert!(p.matches(&path(&["a", "b", "c"])));
    assert!(p.matches(&path(&["a", "b", "c", "d"])));
    assert!(!p.matches(&path(&["a"])));
    assert!(!p.matches(&path(&["a", "c"])));
}

#[test]
fn leading_double_star_matches_any_prefix() {
    let p = pat("**::internal");
    assert!(p.matches(&path(&["internal"])));
    assert!(p.matches(&path(&["a", "internal"])));
    assert!(p.matches(&path(&["a", "b", "internal"])));
    assert!(!p.matches(&path(&["internal", "x"])));
}

#[test]
fn double_star_alone_matches_everything() {
    let p = pat("**");
    assert!(p.matches(&[]));
    assert!(p.matches(&path(&["a"])));
    assert!(p.matches(&path(&["a", "b", "c"])));
}

#[test]
fn parse_rejects_empty_and_partial_wildcards() {
    assert!(ModulePattern::parse("").is_err());
    assert!(ModulePattern::parse("a::").is_err());
    assert!(ModulePattern::parse("foo*").is_err());
    assert!(ModulePattern::parse("a::foo*::b").is_err());
}

#[test]
fn spec_exclude_wins_over_include() {
    let spec = FilterSpec {
        include: vec![pat("a::**")],
        exclude: vec![pat("a::internal::**")],
        collapse: vec![],
    };
    assert_eq!(spec.decides(&path(&["a", "public"])), Decision::Keep);
    assert_eq!(spec.decides(&path(&["a", "internal"])), Decision::Drop);
    assert_eq!(spec.decides(&path(&["a", "internal", "x"])), Decision::Drop);
    assert_eq!(spec.decides(&path(&["b"])), Decision::Drop);
}

#[test]
fn spec_empty_include_keeps_everything_by_default() {
    let spec = FilterSpec::default();
    assert_eq!(spec.decides(&path(&["a", "b"])), Decision::Keep);
    assert_eq!(spec.decides(&[]), Decision::Keep);
}

#[test]
fn spec_collapse_only_when_kept() {
    let spec = FilterSpec {
        include: vec![],
        exclude: vec![pat("a::internal::**")],
        collapse: vec![pat("a::db")],
    };
    assert_eq!(spec.decides(&path(&["a", "db"])), Decision::Collapse);
    assert_eq!(spec.decides(&path(&["a", "internal"])), Decision::Drop);
    assert_eq!(spec.decides(&path(&["a", "public"])), Decision::Keep);
}
