# Architecture

archviz is organized as a **pipeline** of independent stages. Each stage is
defined by a public trait in its own module, with one concrete
implementation provided today. The `Pipeline` struct (`src/pipeline.rs`)
owns one instance of each stage and drives them in sequence; it is the only
piece of code that knows about *all* the modules and wires them together.

`Pipeline::run()` is the single orchestrator: it directly calls every
module below, in order, on every run. No stage calls another stage
directly — `Pipeline` is the only piece of code that knows about all of
them.

```mermaid
flowchart TD
    Main["main.rs"] -->|"Pipeline::builder(root).build().run()"| P["Pipeline\n(src/pipeline.rs)"]

    P -->|"1 . ProjectLoader::new(root).load()"| PL["ProjectLoader\n(src/project)"]
    PL -->|"Vec&lt;SourceFile&gt;"| P

    P -->|"2 . for each file: parser.parse(file)"| PA["Parser (AstParser)\n(src/parser)"]
    PA -->|"ParsedModule"| P

    P -->|"3 . GraphVisitor::new(&mut graph, ...).visit_module(...)"| GV["GraphVisitor\n(src/parser/visitor.rs)"]
    GV -->|"populates"| G["Graph\n(src/model, nodes + edges)"]

    P -->|"4 . for each enricher: enricher.enrich(&mut graph)"| EN["GraphEnricher(s) (TypeExpander, OriginResolver)\n(src/enricher)"]
    EN -->|"mutates"| G

    P -->|"5 . renderer.render(&graph)"| R["Renderer (PlantUmlRenderer)\n(src/renderer)"]
    R -->|"String"| P

    P -->|"println!"| Out["PlantUML output"]
```

Each numbered step is a call `Pipeline::run()` makes itself, in this exact
order:

1. Construct a `ProjectLoader` for the root path and call `.load()` to get
   all `SourceFile`s.
2. For each file, call `self.parser.parse(file)` to get a `ParsedModule`.
3. Construct a `GraphVisitor` over the shared `Graph` and call
   `.visit_module(&parsed)`, which walks the AST and pushes `Node`s/`Edge`s.
4. After *all* files are parsed, call `enricher.enrich(&mut graph)` for
   every enricher in `self.enrichers`, in order.
5. Call `self.renderer.render(&graph)` to produce the final output string.

## Stages

### 1. `ProjectLoader` (`src/project/`)

Walks the target directory (via `walkdir`) and collects every `*.rs` file
into a `SourceFile { path, module_path, source }`. The `module_path` is
derived from the file's path relative to the project root (e.g.
`src/foo/bar.rs` → `["foo", "bar"]`; a `mod.rs` file drops its own trailing
segment), so downstream stages can know which "package" a type belongs to
without re-deriving it from the filesystem.

### 2. `Parser` (`src/parser/`)

The `Parser` trait has one method, `parse(SourceFile) -> ParsedModule`,
implemented by `AstParser` using the [`syn`](https://docs.rs/syn) crate to
turn source text into an AST plus the file's `module_path`.

`GraphVisitor` (`src/parser/visitor.rs`) then walks each `ParsedModule`'s
AST using `syn::visit::Visit` and mutates a shared `Graph`:

- Each `struct` becomes a `Node` (`NodeKind::Struct`).
- Each `trait` becomes a `Node` (`NodeKind::Trait`).
- Each field of a struct becomes a `Composition` edge from the struct to
  the field's type.
- Each `impl Trait for Struct` becomes an `Implements` edge.

Field types are recursively decomposed into a `TypeExpr` tree (see
`extract_type_expr` in `visitor.rs`), unwrapping references, generics,
arrays/slices, tuples, and `dyn`/`impl Trait` bounds so that even complex
field types can be represented as relationships in the graph.

### 3. `GraphEnricher` (`src/enricher/`)

Runs *after* all files have been parsed, so it can see the complete graph.
The `GraphEnricher` trait has one method, `enrich(&self, graph: &mut
Graph)`. Two implementations ship by default and run in this order:

`TypeExpander` (`src/enricher/type_expander.rs`) walks every edge's
`TypeExpr` and:

- Synthesizes new `Node`s (`NodeKind::Synthetic`) for compound types that
  have no explicit source definition — e.g. `Vec<String>` becomes its own
  diagram node with a `Composition` edge to `String`.
- Resolves `dyn Trait` / `impl Trait` bounds directly to the underlying
  trait node(s) instead of creating an intermediate node.

`OriginResolver` (`src/enricher/origin_resolver.rs`) then iterates every
node's outgoing edges and classifies each referenced base type as
`Local` (already a node in the graph), `Std` (matches a known stdlib /
primitive name), or `External`. Types with no matching node yet are
materialized as synthetic nodes placed under the `std` or `external`
package so they appear in the diagram, cleanly separated from
project-local types. The classification is name-based (see
`src/enricher/origin.rs`); a later revision could delegate to
rust-analyzer for precise resolution.

`Pipeline` supports **multiple** enrichers (`Vec<Box<dyn GraphEnricher>>`),
run in order, so additional enrichment passes (e.g. computing metrics,
filtering, or adding annotations) can be added without touching existing
ones.

### 4. `Renderer` (`src/renderer/`)

`Renderer` is a supertrait of `DrawNode` + `DrawEdge`. Its `render(&self,
graph: &Graph) -> String` method has a default implementation that emits
`preamble()`, then every node via `draw_node`, then every edge via
`draw_edge`, then `postamble()`. Implementors only need to supply
`draw_node`/`draw_edge` (and optionally override `preamble`/`postamble`).

`PlantUmlRenderer` (`src/renderer/plantuml.rs`) is the only implementation
today; it nests `package "..." { ... }` blocks per `module_path` segment
around each node and prints PlantUML `class`/`interface`/`enum`
declarations and relationship arrows.

## Data flow

Data flows strictly **one way** through the pipeline via the shared
`Graph` (`src/model/graph.rs`, just `Vec<Node>` + `Vec<Edge>`):

```
files ─▶ Parser ─▶ Graph (built incrementally per file)
                     │
                     ▼
              GraphEnricher(s) (mutate Graph in place)
                     │
                     ▼
                 Renderer (Graph → String)
```

There is no back-communication between stages — a later stage never
reaches back into an earlier one. This keeps each stage independently
testable and replaceable.

## Wiring: `Pipeline` and `PipelineBuilder`

`main.rs` builds and runs the pipeline in two lines:

```rust
let output = Pipeline::builder(root).build().run()?;
```

`PipelineBuilder::new` supplies the default implementations (`AstParser`,
`TypeExpander`, `PlantUmlRenderer`). Callers can override any stage before
calling `.build()`:

```rust
Pipeline::builder(root)
    .with_parser(MyParser)
    .with_enricher(MyExtraEnricher)
    .with_renderer(MyRenderer)
    .build()
    .run()?;
```

`Pipeline::run` performs the actual orchestration described above: load
files, parse + visit each one into the `Graph`, run all enrichers, then
render. See `design-patterns.md` for the design patterns behind this
wiring (trait objects, builder pattern, strategy pattern, visitor
pattern).
