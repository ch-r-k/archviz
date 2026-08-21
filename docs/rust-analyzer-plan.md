# Rust-analyzer-backed type resolution — implementation plan

## Goal

Replace the heuristic type resolution inside the enricher stage with
rust-analyzer's semantic index, **without touching the parser, model,
renderer, or pipeline orchestration**. The `GraphEnricher` trait is the
sole extension point; everything else stays the same.

Success criteria:
- Ambiguous same-name types across modules resolve to the correct
  fully-qualified `NodeId` (no `eprintln!` warnings, no "pick first"
  fallbacks).
- `std`/`core`/`alloc`/external-crate types are classified by asking
  rust-analyzer which crate defines them, not by string tables.
- The existing heuristic enrichers still work when rust-analyzer is not
  available (non-Cargo input, or feature disabled).
- `cargo test` remains green; the new enricher is exercised by at least
  one integration-style test against a fixture Cargo project.

## Scope boundary

**In scope (enricher only):**
- New `RustAnalyzerResolver: GraphEnricher`.
- Whatever parser-side plumbing is required to hand the enricher a
  *position* (file + span) for each `Edge` and `Node`, since RA resolves
  by offset, not by identifier string.

**Out of scope:**
- Replacing `syn` in the parser.
- Rewriting the renderer.
- Changing `NodeId` semantics — RA output is mapped into the existing
  `NodeId::from_parts` / `NodeId::bare` shape.

## Milestones

Each milestone is independently shippable and independently testable.
Later milestones depend on earlier ones only where noted.

### M1 — Parser preserves source positions

Rust-analyzer resolves by `(FileId, TextRange)`, so the enricher needs a
location for every bare-name reference the parser emits.

- Add `pub struct SourceSpan { pub file: PathBuf, pub start: usize, pub end: usize }`
  in `src/model/span.rs`.
- Add `origin: Option<SourceSpan>` to `Edge`. Optional so synthetic
  edges (created by `TypeExpander`) can leave it `None`.
- Add `origin: Option<SourceSpan>` to `Node` (used later by the origin
  classifier to look up the *defining* type, not just references).
- In `GraphVisitor` / `GraphBuilder`, populate `origin` from `syn`'s
  `Spanned` trait using the visitor's current file path. The visitor
  already carries `module_path`; extend it to also carry the absolute
  file path.
- Existing constructors gain a `_with_origin` variant so tests can stay
  terse. Default `origin: None` for old call sites.

Verify: existing 30 tests still green; add one test that inspects
`edge.origin` after parsing a fixture file.

### M2 — Optional dependency wiring

The rust-analyzer crates are heavyweight and track nightly. Gate them.

- Add a `ra` Cargo feature in `Cargo.toml`. Off by default.
- Under `[dependencies]` with `optional = true`:
  - `ra_ap_hir`
  - `ra_ap_ide_db`
  - `ra_ap_load-cargo`
  - `ra_ap_project-model`
  - `ra_ap_vfs`
  - `ra_ap_paths`
  Pin exact versions (the `ra_ap_*` crates are published from nightly
  and don't follow semver rigorously).
- `[features] ra = ["dep:ra_ap_hir", ...]`.
- New module `src/enricher/rust_analyzer/` gated by `#[cfg(feature = "ra")]`.
- CI / local dev: `cargo build` (no feature) must stay 0-warning and
  independent of the ra crates.

Verify: `cargo build`, `cargo build --features ra`, `cargo test`,
`cargo test --features ra` all pass.

### M3 — Workspace loader

A thin wrapper that owns the RA analysis DB and exposes just what the
enricher needs.

- `src/enricher/rust_analyzer/workspace.rs`
- `pub struct RaWorkspace { host: AnalysisHost, vfs: Vfs, root_krate_map: HashMap<PathBuf, Crate> }`
- `impl RaWorkspace { pub fn load(cargo_toml: &Path) -> Result<Self, ArchError> }`
  - Uses `load_cargo::load_workspace_at` with default `LoadCargoConfig`
    (`load_out_dirs_from_check: false`, `with_proc_macro_server: None`,
    `prefill_caches: false`) to keep startup under ~3s for small crates.
- `pub fn file_id(&self, path: &Path) -> Option<FileId>` — VFS lookup.
- Errors: any RA load failure is wrapped in `ArchError::Workspace(String)`
  (new variant) and surfaces cleanly via `anyhow` at `main.rs`.

Verify: unit test that loads `example/` after adding a minimal
`example/Cargo.toml`, asserts the crate root is discovered.

### M4 — `RustAnalyzerResolver` enricher — edge resolution

The core replacement for `EdgeTargetResolver`.

- `src/enricher/rust_analyzer/resolver.rs`
- `pub struct RustAnalyzerResolver { ws: RaWorkspace }`.
- `impl GraphEnricher for RustAnalyzerResolver`:
  - For each `Edge` with `to.is_bare()` and `origin.is_some()`:
    1. Convert `SourceSpan` → `(FileId, TextRange)`.
    2. Build a `Semantics<'_, RootDatabase>`.
    3. Find the `ast::Path` at that range.
    4. `sema.resolve_path(&path)` → `PathResolution::Def(ModuleDef)`.
    5. Extract `ModuleDef` → `Definition` → `canonical_module_path` →
       `Vec<Name>` → join with `::`.
    6. Rewrite `edge.to = NodeId::from_parts(&module_segments, &name)`.
  - Unresolved paths stay as bare `NodeId` — `OriginResolver` (or the
    RA classifier from M5) picks them up.
- Helper split (per the "5–9 methods per struct" convention):
  - `resolve_edge`, `path_at_span`, `definition_to_node_id`,
    `krate_of_definition`.

Register in `PipelineBuilder::with_rust_analyzer(cargo_toml)` — this
method replaces the default `EdgeTargetResolver` with
`RustAnalyzerResolver`, leaves `OriginResolver` in place for M5.

Verify:
- Fixture Cargo project with two modules each defining `Foo`, and a
  third module holding `struct Bar { a: mod_a::Foo, b: Foo }`. Assert
  the two edges resolve to distinct `NodeId`s without warnings.
- Compare against `EdgeTargetResolver` on the same fixture; RA output
  must be a superset (no regressions on cases the heuristic already
  handled).

### M5 — RA-backed origin classification

Fold origin resolution into the same enricher, killing the std/primitive
tables.

- Extend `RustAnalyzerResolver::enrich` to, after edge rewriting, walk
  every `NodeId` referenced but not defined in the graph and:
  1. Find the `Definition` behind the id (via the resolved edge from
     M4, or via `find_def_at` for source Nodes).
  2. `def.krate(db).map(|k| k.display_name(db))`.
  3. Classify:
     - No krate → primitive (`i32`, `bool`, …) → stub with
       `origin: Origin::Std`.
     - Krate name `core`/`std`/`alloc`/`proc_macro` → `Origin::Std`.
     - Krate name matches the workspace root crate(s) → local (already
       in graph, do nothing).
     - Any other krate → `Origin::External(krate_name)`.
- Materialise stubs the same way `OriginResolver` does today (using
  `Graph::push_node` on a `Node` with the right `NodeKind` — RA also
  tells us struct vs trait vs enum via `ModuleDef` variants, so
  `NodeKind` becomes accurate for stubs too).
- Delete `STD_TYPES`, `PRIMITIVES`, and `is_std_name` from
  `src/model/origin.rs` **only** when the RA path is compiled in.
  Keep them under `#[cfg(not(feature = "ra"))]` so the default build
  keeps the heuristic.

Verify:
- Fixture referencing `Vec<String>`, `serde::Serialize`, and a local
  type. Assert three stubs with `Origin::Std`, `Origin::External("serde")`,
  and no stub respectively.

### M6 — Pipeline wiring & CLI flag

- `PipelineBuilder::with_rust_analyzer(cargo_toml: PathBuf) -> Self`:
  - Loads `RaWorkspace`.
  - Replaces the default enricher chain with `[RustAnalyzerResolver]`
    (no `EdgeTargetResolver`, no heuristic `OriginResolver`).
  - On workspace-load failure, logs `eprintln!` and falls back to the
    default heuristic chain — the tool still produces a diagram.
- `main.rs`: accept `--resolver ra` / `--resolver heuristic` (default
  `heuristic`). When `ra` is chosen but the feature isn't compiled in,
  exit with a clear error.
- Detect `Cargo.toml` at the given path when `--resolver ra`; if it's
  a bare src dir, ask the user to point at the crate root.

Verify: `cargo run -- example/ --resolver ra` produces the same or
better diagram than the default.

### M7 — Docs & conventions update

- `docs/architecture.md`: add the RA enricher to the Mermaid diagram
  as an alternative branch after "Parse".
- `docs/design-patterns.md`: note that `RustAnalyzerResolver` is a
  second strategy for `GraphEnricher` — reinforces the pattern rather
  than replacing it.
- `docs/requirements.md`: add FRs for "resolve ambiguous names via
  semantic analysis" and "classify crate origin via semantic analysis",
  tick them off.
- `.github/copilot-instructions.md`: add a paragraph under Conventions
  noting the `ra` feature gate and that RA-specific code lives under
  `src/enricher/rust_analyzer/`.
- New `docs/rust-analyzer-integration.md` end-user doc: how to enable
  the feature, what it changes, known limitations.

## Cross-cutting concerns

**Error handling.** All RA errors funnel through a new
`ArchError::Workspace(String)` variant and are converted to `anyhow`
at `main.rs`. The enricher never panics on unresolved paths — it
leaves them bare for downstream handling.

**Determinism.** RA is deterministic given the same input, but the
canonical path for re-exports can vary. Prefer `Definition::module(db)`
+ item name over `canonical_module_path` when the latter can point at
a re-export module; document the choice in the resolver module.

**Performance.** Load the workspace once per `Pipeline` run, not per
enricher call. Cache `Semantics` inside `RustAnalyzerResolver` for the
lifetime of `enrich`.

**Testing strategy.** Fixture Cargo projects under `tests/fixtures/`,
one per resolution scenario (same-name collision, std reference,
external crate, re-export). Each test runs the full pipeline with
`--resolver ra` and asserts on the resulting `Graph`, not on the
PlantUML text.

## Risk register

| Risk | Likelihood | Mitigation |
| --- | --- | --- |
| `ra_ap_*` API breaks on version bump | High | Pin exact versions; isolate to one module; upgrade in dedicated PRs. |
| RA load slow on large workspaces | Medium | Feature is opt-in; document expected startup cost. |
| RA requires nightly toolchain features | Low | `ra_ap_*` crates ship prebuilt against stable; verify in CI. |
| Losing "point at any src dir" use case | Certain (when RA is on) | Keep heuristic path as the default; RA is opt-in via flag. |
| Span plumbing (M1) touches every parser test | Medium | Make `origin` `Option<_>` with a `_with_origin` constructor variant; existing tests keep working unchanged. |

## Rollout order

M1 → M2 → M3 → M4 → M5 → M6 → M7. Ship each as a separate PR.
M1 and M2 can be reviewed in parallel; everything else is sequential.

## Definition of done

- `cargo test` green with and without `--features ra`.
- `cargo run -- example/ --resolver ra` produces a diagram whose
  edges all point at resolved (non-bare) `NodeId`s except for
  primitives (which are correctly stubbed as `Origin::Std`).
- Docs in `docs/` reflect the new enricher and the two-resolver
  strategy.
- The default (non-`ra`) build keeps working on a bare src directory
  with zero regressions vs. today.
