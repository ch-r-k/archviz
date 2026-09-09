# archviz

A Rust CLI that parses a Rust project's source tree and generates a PlantUML
class diagram of its structs, traits, enums, and their relationships.

## Build, run, test

```sh
cargo build
cargo run -- <path-to-rust-project>      # e.g. cargo run -- example/src
cargo test                               # run all tests
cargo test draw_struct_node_with_packages  # run a single test by name
```

There is no separate lint/CI config; `cargo build` warnings should still be
treated as signal (see "known rough edges" below).

The `example/` directory is a small sample Rust project (not part of the
`archviz` crate/workspace) used as manual test input for the CLI — run
`cargo run -- example/src` to sanity-check output after changes to the
parser/enricher/renderer.

## Architecture & documentation

`Pipeline` (`src/pipeline.rs`) wires together four swappable stages —
`ProjectLoader` (`src/project/`), `Parser` (`src/parser/`), `GraphEnricher`
(`src/enricher/`), and `Renderer` (`src/renderer/`) — each backed by a
trait so implementations can be swapped via `PipelineBuilder::with_*`.
Data flows one-way through them via the shared `Graph`
(`src/model/graph.rs`: `Vec<Node>` + `Vec<Edge>`); there's no back
communication between stages.

Full details, diagrams, and rationale live in `docs/` and should be kept in
sync when the pipeline, its stages, or requirements change:

- `docs/general-description.md` — what the project is and how to run it.
- `docs/architecture.md` — how `Pipeline` calls each stage in order, with
  a Mermaid flow diagram (what each stage does: `ProjectLoader` computes
  `module_path` from each file's path, `AstParser`/`GraphVisitor` build the
  `Graph` from the AST, `TypeExpander` synthesizes nodes for compound
  types like `Vec<String>`, `PlantUmlRenderer` nests `package` blocks per
  module segment).
- `docs/design-patterns.md` — design patterns used (strategy, builder,
  dependency injection, visitor, template method, interface segregation),
  each explained generally plus how archviz applies it, with diagrams.
- `docs/requirements.md` — functional/non-functional requirements with
  `[x]`/`[ ]` checkboxes tracking implementation status (e.g. enums have
  `NodeKind`/renderer support but the visitor doesn't emit them yet);
  update the checkbox and note when a requirement's status changes.

## Conventions

- Each pipeline stage lives in its own module with a public trait
  (`Parser`, `GraphEnricher`, `Renderer`) plus one concrete implementation;
  when adding a new variant, implement the trait rather than special-casing
  inside `Pipeline`.
- Each stage's public trait(s) live in a dedicated `traits.rs` file
  inside the stage (`src/<stage>/traits.rs`) and are re-exported from
  the stage's `mod.rs` with `pub use traits::…`. Do not define
  `pub trait` items inline in `mod.rs`. When a stage has multiple
  related traits (e.g. `DrawNode` + `DrawEdge` + `Renderer`) they may
  share one `traits.rs`.
- All tests for a stage live in a single `src/<stage>/tests.rs` file,
  gated at the top with `#![cfg(test)]` and registered via `mod tests;`
  in the stage's `mod.rs`. Do not scatter `#[cfg(test)] mod tests { …
  }` blocks inline in the implementation files.
- `TypeExpr` recursion (`type_name`, `resolve_refs`, `children`,
  `trait_object_names`, `is_compound`, `stereotype_params`) lives as
  inherent methods on `TypeExpr` itself (`src/parser/type_expr.rs`) —
  this is the single source of truth for naming/resolving a `TypeExpr`.
  Both `parser/graph_builder.rs` (which now owns compound-type
  synthesis) and `renderer/plantuml.rs` call these methods rather than
  each maintaining their own copy; add new `TypeExpr` traversal logic
  there, not as free functions in a consumer module.
- Prefer several small, single-purpose functions over one large function
  (e.g. `GraphBuilder::record_type` delegates to `synthesize` and
  `ensure_generic_base` rather than inlining all of that logic). As a
  rule of thumb, keep the number of public methods/fields on a
  struct/trait and the number of responsibilities in a function within
  about 5–9 — the range people can hold in mind at once — splitting
  further when it grows beyond that.
- `src/error.rs` defines `ArchError` via `thiserror` and is wired into
  `main.rs`; stage traits (`Parser::parse`, `ProjectLoader::load`) return
  `Result<_, ArchError>` so callers can distinguish IO from parse
  failures. `Pipeline::run` maps these into `anyhow::Result` at the
  boundary and logs per-file warnings to stderr instead of aborting on
  the first bad file.
- Graph identity is carried by `NodeId` (`src/model/node.rs`), a
  newtype over `String`. Source-level nodes get a fully-qualified id
  (`"a::b::Foo"`) from `NodeId::from_parts(module_path, name)`, so two
  same-named items in different modules stay distinct; synthetic nodes
  and unresolved edge targets use `NodeId::bare(name)`. The parser
  emits edge targets as bare ids; the `EdgeTargetResolver` enricher
  (`src/enricher/edge_target_resolver.rs`) runs first in the default
  chain and rewrites them to FQ ids (preferring the owner's own module
  when a display name is ambiguous), so `OriginResolver` only sees
  genuinely unresolved targets. The renderer uses PlantUML
  `class "<display>" as "<id>"` aliases so edges can reference the FQ
  id.
