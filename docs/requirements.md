# Requirements

This document lists the functional and non-functional requirements for
archviz, derived from its current implementation and intended purpose.

**Legend:** `[x]` implemented and verified in the current codebase ·
`[ ]` not yet implemented, or only partially implemented (see the note on
that item for what's missing).

## Functional requirements

### Input

- [x] FR1: The tool shall accept a single command-line argument specifying
  the path to a directory containing Rust source files (e.g. a crate's
  `src/` directory).
- [x] FR2: The tool shall recursively discover all files with a `.rs`
  extension under the given path, including files in nested
  subdirectories.
- [x] FR3: The tool shall derive each discovered file's module path from
  its location relative to the given root, following Rust's module-path
  conventions (e.g. `src/foo/bar.rs` → module `foo::bar`; a `mod.rs` file's
  own segment is dropped from its module path).

### Parsing and analysis

- [x] FR4: The tool shall parse each discovered file as Rust source code
  and extract its abstract syntax tree.
- [x] FR5: The tool shall identify all `struct` definitions and represent
  each as a diagram node.
- [x] FR6: The tool shall identify all `trait` definitions and represent
  each as a diagram node.
- [x] FR7: The tool shall identify all fields of a struct and represent
  each field's type as a relationship (composition) from the struct to
  that type.
- [x] FR8: The tool shall identify all `impl Trait for Struct` blocks and
  represent them as an "implements" relationship from the struct to the
  trait.
- [x] FR9: The tool shall decompose complex field types — references,
  generics, arrays, slices, tuples, `dyn Trait`, and `impl Trait` — into
  their constituent parts so that relationships to the underlying types
  are captured, not just the outermost type.
- [x] FR10: The tool shall continue processing remaining files if parsing
  an individual file succeeds, and shall surface a clear error if a file
  cannot be parsed or read.
  > Implemented: `Pipeline::run` collects per-file results, logs a warning
  > to stderr for each failed file (path + reason), and only aborts the
  > whole run if zero files parse successfully.

### Enrichment

- [x] FR11: The tool shall, after analyzing all files, synthesize
  additional diagram nodes for compound types that have no explicit
  source-level definition (e.g. `Vec<String>`, `Option<T>`, `Box<dyn
  Trait>`), so that such relationships are still visualized.
- [x] FR12: The tool shall resolve `dyn Trait` / `impl Trait` bounds to
  the underlying trait node(s) rather than creating a redundant
  intermediate node.
- [x] FR13: The enrichment step shall support being extended with
  additional, independent enrichment passes without modifying existing
  ones.
  > Reinforced by the opt-in `ModuleFilter` enricher (see FR20–FR22
  > below), which was added purely by implementing `GraphEnricher` and
  > appending to the chain via `PipelineBuilder::with_module_filter`.
- [x] FR20: The tool shall support a repeatable `--exclude <pattern>`
  flag that drops all nodes whose module path matches the pattern
  along with any edges incident to them.
- [x] FR21: The tool shall support a repeatable `--include <pattern>`
  flag that, when set, keeps only nodes whose module path matches at
  least one include pattern (exclude always wins).
- [x] FR22: The tool shall support a repeatable `--collapse <pattern>`
  flag that hides the classes/traits/enums under matching modules and
  replaces the subtree with a single empty `package` node, with
  previously-inbound edges redirected to that package.

### Rendering / output

- [x] FR14: The tool shall render the analyzed and enriched model as
  diagram markup that can be consumed by an external UML rendering tool.
- [x] FR15: The default output format shall be PlantUML class-diagram
  syntax, representing structs as `class`, traits as `interface`, and
  enums as `enum`.
  > Implemented: `GraphVisitor` now visits `syn::ItemEnum`, `GraphBuilder`
  > has `add_enum`, and enum variants' typed fields are recorded as
  > composition edges from the enum node.
- [x] FR16: The rendered output shall group nodes into nested packages
  reflecting their module path, so the diagram reflects the project's
  module structure.
- [x] FR17: The rendered output shall include all discovered composition
  and "implements" relationships as diagram edges/arrows.
- [x] FR18: The tool shall print the rendered diagram markup to standard
  output, so it can be redirected to a file or piped into another tool.
- [x] FR19: The rendering step shall be replaceable with an alternative
  output format (e.g. Mermaid, Graphviz) without requiring changes to the
  parsing or enrichment steps.

## Non-functional requirements

- [x] NFR1: The tool shall be usable as a single self-contained
  command-line binary (`cargo run -- <path>` / a compiled executable),
  requiring no network access or external services to run.
- [x] NFR2: Each pipeline stage (loading, parsing, enrichment, rendering)
  shall be independently testable and swappable, so that new languages,
  analyses, or output formats can be added with minimal changes to
  existing code (open/closed principle).
- [ ] NFR3: The tool shall process a typical small-to-medium Rust project
  (tens to low hundreds of files) in a few seconds or less on commodity
  hardware.
  > Not verified: no benchmark or timing test exists yet to confirm this
  > target is met.
- [x] NFR4: The tool's behavior shall be deterministic for a given input
  directory — running it twice on unchanged source shall produce
  identical output.
- [x] NFR5: The codebase shall have automated tests covering the parsing,
  enrichment, and rendering stages, runnable via `cargo test`.
  > Implemented: unit tests exist for the renderer
  > (`src/renderer/tests.rs`), the parser's `TypeExtractor`
  > (`src/parser/tests.rs`), the enricher's `OriginResolver` and
  > `ModuleFilter` (`src/enricher/tests.rs`), the module-path
  > `FilterSpec` (`src/filter/tests.rs`), and CLI parsing
  > (`src/cli.rs`). Black-box **system tests** in `tests/system.rs`
  > invoke the compiled binary against the bundled `example/src` and
  > against archviz's own `src/` tree, asserting on the emitted
  > PlantUML and writing every rendered diagram to
  > `target/systemtest-output/` for developer inspection.
- [x] NFR6: Each struct/trait shall expose a small, human-trackable number
  of members (fields/methods) — roughly 5–9 (Miller's "seven, plus or
  minus two") — with logic beyond that split into smaller, well-named
  helper functions/types rather than one large one.
  > Applied during cleanup: type-expression expansion (compound-type
  > synthesis, trait-wrapper unwrapping, generic-base specialization)
  > was pulled out of the enricher stage and folded into
  > `GraphVisitor::record_type` / `synthesize` in the parser, so the
  > graph model exposes only stringly-typed edges and `TypeExpr` stays
  > a parser-internal detail. The enricher stage is now just
  > `OriginResolver`, which classifies each edge target as
  > local/std/external.

## Out of scope (current version)

- Analyzing behavior, control flow, or runtime execution (e.g. sequence
  diagrams) — only static structural relationships are modeled.
- Emitting diagrams for enums' variants, type aliases, or free functions
  in full detail (partial/planned support only — see
  `architecture.md`).
- Rendering diagrams directly as images; archviz emits diagram *source*
  (e.g. PlantUML text) and relies on an external renderer to produce
  the final image.
- Analyzing languages other than Rust.
