//! Command-line argument parsing.
//!
//! Kept intentionally dependency-free (no `clap`) to match the rest of
//! the crate's minimal-deps style. If the flag set grows past a handful,
//! consider swapping this out for a real parser — everything downstream
//! only depends on the produced [`Cli`] struct.

use anyhow::{Result, anyhow};

use crate::filter::{FilterSpec, ModulePattern};

/// Parsed command-line invocation.
pub struct Cli {
    pub root: String,
    pub filter: FilterSpec,
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
    let mut spec = FilterSpec::default();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--include" | "--exclude" | "--collapse" => {
                let value = args
                    .next()
                    .ok_or_else(|| anyhow!("`{}` requires a pattern argument", arg))?;
                push_pattern(&mut spec, &arg, &value)?;
            }
            "--collapse-depth" => {
                let value = args
                    .next()
                    .ok_or_else(|| anyhow!("`--collapse-depth` requires a numeric argument"))?;
                set_collapse_depth(&mut spec, &value)?;
            }
            other if other.starts_with("--include=") => {
                push_pattern(&mut spec, "--include", &other["--include=".len()..])?;
            }
            other if other.starts_with("--exclude=") => {
                push_pattern(&mut spec, "--exclude", &other["--exclude=".len()..])?;
            }
            other if other.starts_with("--collapse-depth=") => {
                set_collapse_depth(&mut spec, &other["--collapse-depth=".len()..])?;
            }
            other if other.starts_with("--collapse=") => {
                push_pattern(&mut spec, "--collapse", &other["--collapse=".len()..])?;
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

    Ok(Cli { root, filter: spec })
}

fn set_collapse_depth(spec: &mut FilterSpec, value: &str) -> Result<()> {
    if spec.collapse_depth.is_some() {
        return Err(anyhow!("`--collapse-depth` may be given at most once"));
    }
    let n: usize = value
        .parse()
        .map_err(|e| anyhow!("`--collapse-depth` expects a non-negative integer: {}", e))?;
    spec.collapse_depth = Some(n);
    Ok(())
}

fn push_pattern(spec: &mut FilterSpec, flag: &str, value: &str) -> Result<()> {
    let pat = ModulePattern::parse(value)?;
    match flag {
        "--include" => spec.include.push(pat),
        "--exclude" => spec.exclude.push(pat),
        "--collapse" => spec.collapse.push(pat),
        _ => unreachable!("unhandled flag `{}`", flag),
    }
    Ok(())
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
        assert_eq!(cli.filter.exclude.len(), 2);
        assert_eq!(cli.filter.collapse.len(), 1);
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
