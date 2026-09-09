//! Command-line argument parsing.
//!
//! Kept intentionally dependency-free (no `clap`) to match the rest of
//! the crate's minimal-deps style. If the flag set grows past a handful,
//! consider swapping this out for a real parser — everything downstream
//! only depends on the produced [`Cli`] struct.
//!
//! The CLI **does not** know about `ModulePattern` / `FilterSpec` /
//! `ModuleFilter`. Filter flags are collected as raw strings on
//! [`CliFilter`] and handed to the pipeline via
//! [`crate::pipeline::PipelineBuilder::with_filter_options`], which
//! owns the parsing.

use anyhow::{Result, anyhow};

/// Parsed command-line invocation.
pub struct Cli {
    pub root: String,
    pub filter: CliFilter,
}

/// Raw, unparsed filter flags exactly as the user typed them. This is
/// the CLI ↔ pipeline boundary: no filter internals leak into CLI code.
#[derive(Debug, Default, Clone)]
pub struct CliFilter {
    pub includes: Vec<String>,
    pub excludes: Vec<String>,
    pub collapses: Vec<String>,
    pub collapse_depth: Option<usize>,
}

impl CliFilter {
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.includes.is_empty()
            && self.excludes.is_empty()
            && self.collapses.is_empty()
            && self.collapse_depth.is_none()
    }
}

/// Parses `std::env::args()` into a [`Cli`]. Recognizes:
///
/// * `<path>` — the target directory (exactly one required positional).
/// * `--include <pattern>` — append to the include list.
/// * `--exclude <pattern>` — append to the exclude list.
/// * `--collapse <pattern>` — append to the collapse list.
/// * `--collapse-depth <N>` — collapse every module deeper than `N`.
/// * `--include=<pattern>` (and the `=` form for the other flags).
///
/// Any of the three filter flags may be repeated;
/// `--collapse-depth` may be given at most once.
pub fn parse() -> Result<Cli> {
    parse_from(std::env::args().skip(1))
}

fn parse_from<I: IntoIterator<Item = String>>(args: I) -> Result<Cli> {
    let mut args = args.into_iter();
    let mut positional: Option<String> = None;
    let mut filter = CliFilter::default();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--include" | "--exclude" | "--collapse" => {
                let value = args
                    .next()
                    .ok_or_else(|| anyhow!("`{}` requires a pattern argument", arg))?;
                push_pattern(&mut filter, &arg, value);
            }
            "--collapse-depth" => {
                let value = args
                    .next()
                    .ok_or_else(|| anyhow!("`--collapse-depth` requires a numeric argument"))?;
                set_collapse_depth(&mut filter, &value)?;
            }
            other if other.starts_with("--include=") => {
                push_pattern(&mut filter, "--include", other["--include=".len()..].to_string());
            }
            other if other.starts_with("--exclude=") => {
                push_pattern(&mut filter, "--exclude", other["--exclude=".len()..].to_string());
            }
            other if other.starts_with("--collapse-depth=") => {
                set_collapse_depth(&mut filter, &other["--collapse-depth=".len()..])?;
            }
            other if other.starts_with("--collapse=") => {
                push_pattern(&mut filter, "--collapse", other["--collapse=".len()..].to_string());
            }
            other if other.starts_with("--") => {
                return Err(anyhow!("unknown flag `{}`", other));
            }
            _ => {
                if positional.is_some() {
                    return Err(anyhow!(
                        "unexpected extra argument `{}` (only one path is supported)",
                        arg
                    ));
                }
                positional = Some(arg);
            }
        }
    }

    let root = positional.ok_or_else(|| {
        anyhow!(
            "usage: archviz <path> \
             [--include <pat>] [--exclude <pat>] \
             [--collapse <pat>] [--collapse-depth <N>]"
        )
    })?;

    Ok(Cli { root, filter })
}

fn set_collapse_depth(filter: &mut CliFilter, value: &str) -> Result<()> {
    if filter.collapse_depth.is_some() {
        return Err(anyhow!("`--collapse-depth` may be given at most once"));
    }
    let n: usize = value
        .parse()
        .map_err(|e| anyhow!("`--collapse-depth` expects a non-negative integer: {}", e))?;
    filter.collapse_depth = Some(n);
    Ok(())
}

fn push_pattern(filter: &mut CliFilter, flag: &str, value: String) {
    match flag {
        "--include" => filter.includes.push(value),
        "--exclude" => filter.excludes.push(value),
        "--collapse" => filter.collapses.push(value),
        _ => unreachable!("unhandled flag `{}`", flag),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args(args: &[&str]) -> Result<Cli> {
        parse_from(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn parses_bare_path() {
        let cli = parse_args(&["example/src"]).unwrap();
        assert_eq!(cli.root, "example/src");
        assert!(cli.filter.is_empty());
    }

    #[test]
    fn parses_repeated_filter_flags() {
        let cli = parse_args(&[
            "example/src",
            "--exclude",
            "**::internal",
            "--exclude=**::tests",
            "--collapse",
            "a::b",
        ])
        .unwrap();
        assert_eq!(cli.root, "example/src");
        assert_eq!(cli.filter.excludes, vec!["**::internal", "**::tests"]);
        assert_eq!(cli.filter.collapses, vec!["a::b"]);
    }

    #[test]
    fn errors_on_missing_pattern_value() {
        assert!(parse_args(&["example/src", "--include"]).is_err());
    }

    #[test]
    fn errors_on_missing_path() {
        assert!(parse_args(&["--exclude", "a"]).is_err());
    }

    #[test]
    fn errors_on_unknown_flag() {
        assert!(parse_args(&["example/src", "--nope"]).is_err());
    }

    #[test]
    fn errors_on_extra_positional() {
        assert!(parse_args(&["a", "b"]).is_err());
    }

    #[test]
    fn cli_does_not_validate_patterns() {
        // Pattern validation now happens in the pipeline; CLI just
        // collects strings.
        let cli = parse_args(&["p", "--include", "not*a*valid*pattern"]).unwrap();
        assert_eq!(cli.filter.includes, vec!["not*a*valid*pattern"]);
    }
}

#[cfg(test)]
mod depth_tests {
    use super::*;

    fn parse_args(args: &[&str]) -> Result<Cli> {
        parse_from(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn parses_collapse_depth() {
        let cli = parse_args(&["p", "--collapse-depth", "2"]).unwrap();
        assert_eq!(cli.filter.collapse_depth, Some(2));
    }

    #[test]
    fn parses_collapse_depth_equals_form() {
        let cli = parse_args(&["p", "--collapse-depth=3"]).unwrap();
        assert_eq!(cli.filter.collapse_depth, Some(3));
    }

    #[test]
    fn rejects_repeated_collapse_depth() {
        assert!(parse_args(&["p", "--collapse-depth", "1", "--collapse-depth", "2"]).is_err());
    }

    #[test]
    fn rejects_non_numeric_collapse_depth() {
        assert!(parse_args(&["p", "--collapse-depth", "abc"]).is_err());
    }
}
