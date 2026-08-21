//! System-level (black-box) tests: build & run the actual `archviz`
//! binary against real Rust source trees and assert on the emitted
//! PlantUML.
//!
//! Cargo automatically builds the binary before running these and sets
//! `CARGO_BIN_EXE_archviz` to its path, so no extra dependencies are
//! needed and no test tries to shell out through `cargo run` (which
//! would recurse and pollute output).
//!
//! Each test also writes the diagram it produced to
//! `target/systemtest-output/<name>.puml` so a developer can eyeball or
//! render them after `cargo test`.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Path to the compiled `archviz` binary, provided by Cargo for
/// integration tests.
fn archviz_bin() -> &'static str {
    env!("CARGO_BIN_EXE_archviz")
}

/// Repository root — the crate manifest's directory.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Runs archviz with the given arguments (paths are joined against the
/// repo root when relative). Fails the test on non-zero exit.
fn run_archviz(args: &[&str]) -> String {
    let output = Command::new(archviz_bin())
        .args(args)
        .current_dir(repo_root())
        .output()
        .expect("failed to spawn archviz binary");

    assert!(
        output.status.success(),
        "archviz exited with {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    String::from_utf8(output.stdout).expect("archviz produced non-UTF8 output")
}

/// Writes `contents` to `target/systemtest-output/<name>.puml` so it can
/// be inspected or rendered after the test run.
fn write_artifact(name: &str, contents: &str) {
    let dir = repo_root().join("target").join("systemtest-output");
    std::fs::create_dir_all(&dir).expect("create artifact dir");
    let path = dir.join(format!("{name}.puml"));
    std::fs::write(&path, contents).expect("write artifact");
    eprintln!("archviz systemtest wrote {}", path.display());
}

fn assert_plantuml_envelope(out: &str) {
    assert!(
        out.starts_with("@startuml"),
        "output missing @startuml header:\n{out}"
    );
    assert!(
        out.trim_end().ends_with("@enduml"),
        "output missing @enduml footer:\n{out}"
    );
}

// ---------------------------------------------------------------------
// example/src — the bundled sample project
// ---------------------------------------------------------------------

#[test]
fn analyzes_example_project_baseline() {
    let out = run_archviz(&["example/src"]);
    write_artifact("example_baseline", &out);

    assert_plantuml_envelope(&out);
    for expected in [
        "domain__User",
        "repository__UserRepository",
        "repository__PostgresUserRepository",
        "service__UserService",
    ] {
        assert!(
            out.contains(expected),
            "expected `{expected}` in baseline output:\n{out}"
        );
    }
    assert!(out.contains("package \"std\" {"), "missing std package:\n{out}");
}

#[test]
fn example_project_is_deterministic() {
    let a = run_archviz(&["example/src"]);
    let b = run_archviz(&["example/src"]);
    assert_eq!(a, b, "two runs on the same input produced different output");
}

#[test]
fn exclude_flag_drops_matching_module_on_example() {
    let out = run_archviz(&["example/src", "--exclude", "repository"]);
    write_artifact("example_exclude_repository", &out);

    assert_plantuml_envelope(&out);
    assert!(
        !out.contains("repository__UserRepository"),
        "excluded node still present:\n{out}"
    );
    assert!(
        !out.contains("repository__PostgresUserRepository"),
        "excluded node still present:\n{out}"
    );
    assert!(out.contains("domain__User"));
    assert!(out.contains("service__UserService"));
}

#[test]
fn collapse_flag_emits_package_placeholder_on_example() {
    let out = run_archviz(&["example/src", "--collapse", "repository"]);
    write_artifact("example_collapse_repository", &out);

    assert_plantuml_envelope(&out);
    assert!(
        out.contains("package repository {") || out.contains("package \"repository\""),
        "expected a package block for `repository`:\n{out}"
    );
    assert!(
        !out.contains("repository__UserRepository"),
        "collapsed class still emitted:\n{out}"
    );
    assert!(
        !out.contains("repository__PostgresUserRepository"),
        "collapsed class still emitted:\n{out}"
    );
}

// ---------------------------------------------------------------------
// archviz's own source tree — dogfooding
// ---------------------------------------------------------------------

#[test]
fn analyzes_own_source_tree() {
    let out = run_archviz(&["src"]);
    write_artifact("archviz_self", &out);

    assert_plantuml_envelope(&out);

    // Every pipeline stage should show up somewhere in the diagram.
    // Match on the sanitized alias form because IDs contain `::`.
    let must_have = [
        "pipeline__Pipeline",
        "pipeline__PipelineBuilder",
        "parser__ast_parser__AstParser",
        "enricher__origin_resolver__OriginResolver",
        "enricher__module_filter__ModuleFilter",
        "renderer__plantuml__PlantUmlRenderer",
        "filter__spec__FilterSpec",
    ];
    for expected in must_have {
        assert!(
            out.contains(expected),
            "expected `{expected}` in self-analysis output"
        );
    }
}

#[test]
fn self_analysis_with_collapse_hides_renderer_internals() {
    let out = run_archviz(&["src", "--collapse", "renderer::**"]);
    write_artifact("archviz_self_collapse_renderer", &out);

    assert_plantuml_envelope(&out);
    assert!(
        !out.contains("renderer__plantuml__PlantUmlRenderer"),
        "collapsed renderer class leaked:\n{out}"
    );
    assert!(out.contains("pipeline__Pipeline"));
    assert!(out.contains("enricher__origin_resolver__OriginResolver"));
}

// ---------------------------------------------------------------------
// CLI error surface
// ---------------------------------------------------------------------

#[test]
fn unknown_flag_exits_nonzero() {
    let output = Command::new(archviz_bin())
        .args(["example/src", "--nope"])
        .current_dir(repo_root())
        .output()
        .expect("spawn");
    assert!(
        !output.status.success(),
        "expected failure for unknown flag, got success"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown flag"),
        "expected `unknown flag` in stderr, got:\n{stderr}"
    );
}

#[test]
fn missing_path_exits_nonzero() {
    let output = Command::new(archviz_bin())
        .current_dir(repo_root())
        .output()
        .expect("spawn");
    assert!(!output.status.success());
}

#[test]
fn artifact_directory_exists() {
    let dir = repo_root().join("target").join("systemtest-output");
    std::fs::create_dir_all(&dir).expect("create artifact dir");
    assert!(Path::new(&dir).exists());
}

// ---------------------------------------------------------------------
// Nested-collapse regression and --collapse-depth
// ---------------------------------------------------------------------

#[test]
fn literal_collapse_covers_descendants_on_self() {
    // Before the subtree-semantics fix, `--collapse enricher` matched
    // only the (nonexistent) exact module `enricher` and left every
    // `enricher::*` submodule untouched. Regression guard.
    let out = run_archviz(&["src", "--collapse", "enricher"]);
    write_artifact("archviz_self_collapse_enricher_literal", &out);

    assert_plantuml_envelope(&out);
    assert!(
        !out.contains("enricher__module_filter__ModuleFilter"),
        "enricher::module_filter class leaked past literal collapse:\n{out}"
    );
    assert!(
        !out.contains("enricher__origin_resolver__OriginResolver"),
        "enricher::origin_resolver class leaked past literal collapse:\n{out}"
    );
    // Should have exactly one `enricher` package block (the collapsed
    // synthetic one), not two (placeholder + surviving submodules).
    let matches = out.matches("package enricher").count()
        + out.matches("package \"enricher\"").count();
    assert_eq!(matches, 1, "expected one enricher package block, got {matches}:\n{out}");
}

#[test]
fn collapse_depth_one_flattens_top_level_on_self() {
    let out = run_archviz(&["src", "--collapse-depth", "1"]);
    write_artifact("archviz_self_collapse_depth_1", &out);

    assert_plantuml_envelope(&out);
    // Every archviz submodule class is gone; only top-level packages
    // remain (plus any depth-1 classes and std/external stubs).
    for gone in [
        "enricher__module_filter__ModuleFilter",
        "renderer__plantuml__PlantUmlRenderer",
        "parser__ast_parser__AstParser",
        "filter__spec__FilterSpec",
    ] {
        assert!(
            !out.contains(gone),
            "expected `{gone}` collapsed at depth 1, still present:\n{out}"
        );
    }
}

#[test]
fn collapse_depth_two_keeps_second_level_on_self() {
    let out = run_archviz(&["src", "--collapse-depth", "2"]);
    write_artifact("archviz_self_collapse_depth_2", &out);

    assert_plantuml_envelope(&out);
    // At depth 2 the enricher submodule classes are still there (they
    // live at module_path.len() == 2).
    assert!(out.contains("enricher__module_filter__ModuleFilter"));
    assert!(out.contains("renderer__plantuml__PlantUmlRenderer"));
}

#[test]
fn literal_include_covers_descendants_on_self() {
    // Regression: `--include renderer` used to return an empty diagram
    // because include was strict-exact. It should now keep the whole
    // `renderer::*` subtree plus referenced std stubs.
    let out = run_archviz(&["src", "--include", "renderer"]);
    write_artifact("archviz_self_include_renderer", &out);

    assert_plantuml_envelope(&out);
    assert!(
        out.contains("renderer__plantuml__PlantUmlRenderer"),
        "expected renderer classes to survive `--include renderer`:\n{out}"
    );
    assert!(
        out.contains("renderer__traits__Renderer"),
        "expected renderer traits to survive `--include renderer`:\n{out}"
    );
    // Non-renderer classes must be gone.
    assert!(
        !out.contains("parser__ast_parser__AstParser"),
        "non-included module leaked into output:\n{out}"
    );
    assert!(
        !out.contains("enricher__module_filter__ModuleFilter"),
        "non-included module leaked into output:\n{out}"
    );
    // Std stubs actually referenced by kept edges must survive.
    assert!(
        out.contains("class String"),
        "referenced std stub was dropped:\n{out}"
    );
}

#[test]
fn depth1_view_of_self_has_no_bogus_model_parser_edge() {
    // Regression: `model --> parser` used to appear in the
    // `--collapse-depth 1` architecture view purely because a shared
    // compound type (`Option<String>`) was first synthesized in a
    // parser file. Compound types now live under `std`, so no such
    // false dependency edge should appear.
    let out = run_archviz(&["src", "--collapse-depth", "1"]);
    write_artifact("archviz_self_collapse_depth_1_regression", &out);

    assert_plantuml_envelope(&out);
    assert!(
        !out.contains("model --> parser"),
        "spurious `model --> parser` edge in depth-1 view:\n{out}"
    );
    // `parser --> model` is legitimate (GraphBuilder holds &mut Graph)
    // and should still be there — this asserts the fix didn't
    // accidentally hide real dependencies.
    assert!(
        out.contains("parser --> model"),
        "legitimate `parser --> model` edge missing:\n{out}"
    );
}
