# Refactoring Plan

A prioritized plan derived from an architectural review of archviz. Each
item lists the problem, the change to make, and where it lands. Phases are
ordered so earlier items unblock later ones.

**Legend:** `[x]` applied · `[ ]` not yet applied.

## Summary of findings

**Strengths**

- Clean pipeline (`ProjectLoader` → `Parser` → `GraphEnricher` → `Renderer`),
  each stage behind a small trait, wired in one place.
- Strategy + builder + dependency injection applied consistently.
- One-way data flow through a shared `Graph`; stages are independently
  testable.
- Parser is well layered: `GraphVisitor` (syn adapter) → `TypeExtractor`
  (syn → `TypeExpr` IR) → `GraphBuilder` (IR → graph). `TypeExpr` doesn't
  leak past the parser.
- Files and functions are small and single-purpose.

**Weaknesses**

1. Nodes are keyed by unqualified name — two `Foo` structs in different
   modules collide.
2. `PlantUmlRenderer::draw_node` opens+closes a fresh `package` block per
   node, producing duplicate wrappers instead of one grouped block per
   module.
3. `Pipeline::run` aborts on the first bad file (FR10 wants resilience).
4. Enums are half-supported: `NodeKind::Enum` + rendering exist, but
   `GraphVisitor` never visits `ItemEnum`.
5. `compute_module` doesn't strip `lib.rs` / `main.rs`, only `mod.rs`.
6. `src/error.rs` is dangling — not `mod`-declared and `thiserror` isn't
   in `Cargo.toml`.
7. O(n²) graph construction: `has_node` and `OriginResolver` linear-scan
   on every insert.
8. Name-based std classification is fragile; type-parameter heuristic
   lives only in the enricher.
9. 7 build warnings (unused imports, unused variants, unused builder
   methods, unused `SourceFile.path`).
10. No tests for parser or enricher (NFR5).
11. Repo custom-instructions doc references old file locations.

## Phase 1 — Correctness foundations

### 1.1 Wire or remove `error.rs` — [x]

- **Problem:** `src/error.rs` defines `ArchError` via `thiserror` but is
  not `mod`-declared in `main.rs`, and `thiserror` is not in
  `Cargo.toml` (compiles only because it's a transitive dep).
- **Do:** either (a) add `thiserror` to `Cargo.toml`, declare `mod error;`
  in `main.rs`, and return `Result<_, ArchError>` from stage traits so
  callers can distinguish IO from parse failures, or (b) delete
  `error.rs` entirely.
- **Recommendation:** (a).

### 1.2 Fix cross-module node name collisions — [ ]

- **Problem:** `Edge.from` / `Edge.to` are unqualified `String`s, so two
  `Foo` structs in different modules collapse and their edges mix.
- **Do:**
  - Introduce `NodeId` (fully-qualified path `a::b::Foo`, or a numeric id
    with a side table).
  - Change `Node` to have `id: NodeId` and `display_name: String`.
  - Change `Edge` to use `NodeId` for `from` / `to`.
  - Update `GraphBuilder`, `OriginResolver`, and `PlantUmlRenderer`
    accordingly.
- **Impact:** touches every module but the surface stays small.

### 1.3 Add HashSet index for node lookup — [x]

- **Problem:** `GraphBuilder::has_node` and `OriginResolver::enrich`
  linear-scan `graph.nodes` on every insert → O(n²) graph construction.
- **Do:** carry a `HashSet<NodeId>` alongside `Vec<Node>` in `Graph` (or
  in the builder) and use it for existence checks. Natural once `NodeId`
  exists (1.2).

### 1.4 Handle `lib.rs` / `main.rs` in `compute_module` — [x]

- **Problem:** `ProjectLoader::compute_module` strips only `mod.rs`, so
  `src/main.rs` → module_path `["main"]`, wrapping the whole diagram in
  a `package "main"`.
- **Do:** drop `lib.rs` and `main.rs` filenames the same way as `mod.rs`.

## Phase 2 — Feature completeness

### 2.1 Emit enum nodes from AST — [x]

- **Problem:** `NodeKind::Enum` + PlantUML `enum` rendering exist but
  `GraphVisitor` never handles `syn::ItemEnum`. FR15 is only half done.
- **Do:**
  - Add `visit_item_enum` to `GraphVisitor`.
  - Add `add_enum` to `GraphBuilder`.
  - Walk each variant's fields, treating typed variants like struct
    fields → composition edges.
  - Flip the FR15 checkbox in `docs/requirements.md`.

### 2.2 Continue on per-file parse errors (FR10) — [x]

- **Problem:** `Pipeline::run` propagates the first `parser.parse(...)?`
  or file-read error and aborts.
- **Do:**
  - Iterate files and collect `Result`s.
  - Log a warning to `stderr` for each failure (path + reason).
  - Only fail the run if zero files parsed successfully.

### 2.3 Group nodes by module in renderer — [x]

- **Problem:** `PlantUmlRenderer::draw_node` opens+closes a `package`
  block per node, producing many disjoint duplicate wrappers.
- **Do:**
  - Move package nesting out of `draw_node` into `Renderer::render`.
  - Group `graph.nodes` by `module_path`, emit one nested block per
    unique path, put all its nodes inside.
  - Update `renderer/tests.rs` and add a multi-node-per-module test.

## Phase 3 — Quality

### 3.1 Add parser + enricher tests (NFR5) — [x]

- **Do:** unit tests for
  - `TypeExtractor` on representative `syn::Type` inputs (references,
    generics, tuples, `dyn Trait`, `impl Trait`, arrays, slices).
  - `GraphBuilder` synth + specializes behavior on `TypeExpr` fixtures.
  - `OriginResolver` classification (`Local` / `Std` / `External`) and
    the type-parameter skip heuristic.
- Fixtures via inline `syn::parse_str::<syn::Type>(...)` and
  `syn::parse_file`.

### 3.2 Improve `OriginResolver` precision — [x]

- **Do:**
  - Extract the std name set into a shared `static` (or `phf`) rather
    than an inline `matches!`.
  - Apply the type-parameter heuristic in the parser too, not only in
    the enricher.
  - Consider matching on full path segments (`std::collections::HashMap`)
    once parser tracks `use` statements.

### 3.3 Clean up build warnings — [x]

- **Do:**
  - Remove unused imports in `pipeline.rs` and `renderer/tests.rs`.
  - Keep `NodeKind::Enum` / `Impl` / `TypeAlias` and builder `with_*`
    methods but annotate with `#[allow(dead_code)]` (they are part of
    the intentional extension surface).
  - Either use `SourceFile.path` in error messages (2.2 needs it
    anyway) or drop it.

### 3.4 Sync repo custom-instructions with reality — [x]

- **Problem:** the guiding doc references `TypeExpr` in
  `src/model/edge.rs` and `enricher/type_expander.rs`, but `TypeExpr`
  lives in `src/parser/type_expr.rs` and type expansion moved into
  `src/parser/graph_builder.rs`.
- **Do:** update the doc so contributors are pointed at the right files.

## Dependency graph

```
wire-or-remove-error-rs ─┐
                         │
fix-node-name-collisions ┼─▶ graph-lookup-index
                         │
                         └─▶ render-group-by-module

emit-enums ──▶ tests-for-parser-enricher
wire-or-remove-error-rs ──▶ tests-for-parser-enricher
```

Everything not shown is independent and can be picked up at any time.

## Suggested execution order

1. `wire-or-remove-error-rs`
2. `fix-node-name-collisions`
3. `graph-lookup-index`
4. `loader-crate-roots`
5. `emit-enums`
6. `resilient-parse-errors`
7. `render-group-by-module`
8. `tests-for-parser-enricher`
9. `origin-resolver-precision`
10. `cleanup-warnings`
11. `sync-custom-instructions`
