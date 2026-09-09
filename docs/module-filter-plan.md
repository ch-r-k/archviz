# Module filter — implementation plan

## Goal

Add a filter stage that lets the user shape the diagram by module path:

- **Include** only certain modules (allowlist).
- **Exclude** certain modules (denylist).
- **Collapse** certain modules so that only the package is drawn — the
  module's classes/traits/enums disappear, and edges that used to
  terminate inside the collapsed subtree are redirected to the collapsed
  package itself.

Filters compose: exclude wins over include; collapse then applies to
whatever nodes survive.

## Where it lives

A new `GraphEnricher` — `ModuleFilter` — running **after**
`EdgeTargetResolver` and `OriginResolver`, **before** the renderer. This
keeps the pipeline shape unchanged; the trait already takes
`&mut Graph`, and mutation (drop nodes, rewrite edges) is a natural fit.

```
loader → parser → EdgeTargetResolver → OriginResolver → ModuleFilter → renderer
```

Order matters: filtering after resolution means edges point at real
`NodeId`s, so redirection is a mechanical rewrite, not a heuristic.

## Model changes

Minimal, so the renderer keeps working with today's abstractions.

- New variant `NodeKind::Package` (or reuse: add `pub is_synthetic_package: bool`
  on `Node` — leaning toward the enum variant, since `NodeKind` is
  already the discriminator the renderer switches on).
- A collapsed module becomes a synthetic `Node { id: NodeId::bare("a::b"),
  display_name: "b", kind: Package, module_path: ["a"] }`. Its `NodeId`
  is deliberately the module path itself so incoming edges can be
  rewritten to it without collision with any real class.
- No changes to `Edge`, no changes to `Graph`. `Graph::push_node`
  already dedups; we lean on that when multiple descendants collapse to
  the same package.

## Filter spec

**Pattern syntax:** module path glob with `*` as the only wildcard.
Familiar, matches how users think about Rust modules, and easy to
implement.

- `a::b` — exact match on module `a::b` (does *not* match `a::b::c`).
- `a::b::*` — any direct child of `a::b`.
- `a::b::**` — `a::b` and any descendant.
- `**::internal` — any module ending in `internal`.

A pattern matches a **module path**, never an individual node. Filtering
individual classes is a separate feature and out of scope here.

**Precedence** (applied in this order per node):
1. If any exclude pattern matches → drop.
2. If include patterns are set and none match → drop.
3. If any collapse pattern matches → mark for collapse.

Include/exclude are optional; collapse is optional; no filter given =
today's behavior.

## Milestones

### M1 — Spec parsing and matcher

- New module `src/filter/`.
- `src/filter/pattern.rs`:
  - `pub struct ModulePattern { segments: Vec<Segment> }`
  - `enum Segment { Literal(String), Star, DoubleStar }`
  - `impl ModulePattern { pub fn parse(&str) -> Result<Self, ArchError> }`
  - `pub fn matches(&self, module_path: &[String]) -> bool`
- `src/filter/spec.rs`:
  - `pub struct FilterSpec { pub include: Vec<ModulePattern>, pub exclude: Vec<ModulePattern>, pub collapse: Vec<ModulePattern> }`
  - `impl FilterSpec { pub fn decides(&self, module_path: &[String]) -> Decision }`
  - `enum Decision { Keep, Drop, Collapse }`
- Unit tests in `src/filter/tests.rs` covering literal / `*` / `**`
  patterns and the precedence rules.

Verify: `cargo test filter::` green, no interaction with other stages
yet.

### M2 — `ModuleFilter` enricher (drop only)

Ship the include/exclude subset first — no collapse yet — to keep the
diff small.

- `src/enricher/module_filter.rs`:
  - `pub struct ModuleFilter { spec: FilterSpec }`
  - `impl GraphEnricher for ModuleFilter`:
    1. Compute the set of dropped `NodeId`s by applying
       `FilterSpec::decides` to each `Node.module_path`.
    2. Retain only non-dropped nodes.
    3. Drop edges where `from` or `to` is in the dropped set.
- Same-file tests under `src/enricher/tests.rs` (per the tests
  convention): drop-by-exclude, keep-by-include, empty include list
  means "keep everything".

Verify: fixture with modules `a::b::internal` and `a::b::public`, and
an exclude pattern `**::internal` — result contains only `public`
nodes and no dangling edges.

### M3 — Collapse

The interesting part. Split into three helpers on `ModuleFilter` to
keep the method count sane:

- `collect_collapse_roots(&self, graph) -> HashMap<NodeId, Vec<String>>`
  Maps every node that lives under a collapse root to the collapse
  root's module path. If a node is under multiple collapse roots, the
  **shortest** root wins (outermost collapse).
- `synthesize_packages(&mut graph, &roots)` — for each unique collapse
  root, `Graph::push_node(Node { kind: Package, ... })`.
- `rewrite_edges(&mut graph, &roots)`:
  1. For each edge, if `from` is in `roots`, rewrite to the collapse
     root's package `NodeId`.
  2. Same for `to`.
  3. Drop nodes that are under a collapse root and are *not*
     themselves the package node.
  4. Deduplicate edges (same from/to/kind); drop self-edges introduced
     by collapse (a class inside `a::b` that referenced another class
     inside `a::b` now becomes `a::b → a::b`, which should vanish).

**Edge cases to nail down in tests:**
- Nested collapses (`a::b::**` and `a::b::c::**`): outer wins,
  inner is redundant. Warn or silently ignore? → silently ignore, add
  a note to docs.
- Collapse root has no nodes underneath: emit the empty package
  anyway so the user sees the placeholder they asked for.
- Edge from collapsed subtree to std stub: `from` becomes the package,
  `to` stays as the stub — normal edge, no special case.
- Two different collapsed roots edge to each other: pkg-a → pkg-b, kept.

Verify: fixture with `a::b::{Foo, Bar}` where `Foo` references `Bar`
and `Bar` references `c::Baz`. Collapse `a::b` → assert one `Package`
node `a::b`, one edge `a::b → c::Baz`, no self-edges, no `Foo`/`Bar`
nodes.

### M4 — Renderer support for `NodeKind::Package`

- In `src/renderer/plantuml.rs`, teach `draw_node` to emit an empty
  `package "<display>" as "<id>" { }` block for `NodeKind::Package`
  instead of a `class`. Edges already reference by `NodeId`; PlantUML
  accepts edges to package aliases.
- `ModuleTree` (`src/renderer/module_tree.rs`) needs no change if the
  synthetic package node's `module_path` is set to its *parent* path
  (so it renders nested under the correct outer package). Verify with
  a nested-modules fixture.
- Renderer tests in `src/renderer/tests.rs`: package-kind node renders
  as `package` not `class`; incoming edge points at the alias.

### M5 — CLI wiring

- New CLI flags in `main.rs`, all repeatable:
  - `--include <pattern>`
  - `--exclude <pattern>`
  - `--collapse <pattern>`
- Build a `FilterSpec` from the flags; if any of the three is
  non-empty, `PipelineBuilder::with_module_filter(spec)` inserts
  `ModuleFilter` at the end of the enricher chain.
- If all three are empty, the filter isn't added — zero cost for
  default runs.
- Wire flag parsing via `std::env::args` for now (matching current
  minimal-deps style); note in the plan that a follow-up could adopt
  `clap` if more flags land.

Verify: `cargo run -- example/src --collapse example::sub` produces
a diagram with a package for the collapsed module and no classes
inside it.

### M6 — Config file (optional, defer if time-boxed)

Some projects will want to check in a filter config. Add a `.archviz.toml`
loader as a follow-up:

```toml
[filter]
include = ["crate::api::**"]
exclude = ["**::tests", "**::internal"]
collapse = ["crate::db"]
```

- New dep: `toml` (small, stable).
- `main.rs` looks for `.archviz.toml` in the target directory and its
  parents; CLI flags override / append to config values.

Ship this only after M1–M5 are stable.

### M7 — Docs & conventions

- `docs/architecture.md`: add `ModuleFilter` to the pipeline Mermaid
  diagram; note that it's opt-in.
- `docs/design-patterns.md`: `ModuleFilter` is another `GraphEnricher`
  strategy — cite it as reinforcement of the pattern.
- `docs/requirements.md`: three new FRs (include, exclude, collapse),
  ticked off as they land.
- New `docs/filtering.md`: end-user documentation for the pattern
  syntax and CLI flags, with worked examples using `example/`.

## Cross-cutting concerns

**Determinism.** Iterate maps/sets in sorted order when producing
output — `HashMap` iteration must not leak into the emitted diagram.
Sort synthetic packages by `NodeId`, sort edges after dedup.

**Empty results.** If filters remove every node, emit a valid but
empty diagram and print a warning to stderr. Don't panic.

**Interaction with `TypeExpander`.** `TypeExpander` synthesizes nodes
for compound types like `Vec<String>` with bare `NodeId`s and empty
`module_path`. These never match any module pattern (bare = no
module), so filters never touch them. Document this so users don't
try to filter `Vec` out via `--exclude Vec`.

**Interaction with `OriginResolver`.** Same story: std/external stubs
have empty `module_path`; they survive all module filters. If a user
wants to hide external crates, that's a different feature (filter by
`Origin`), out of scope here.

**Testing strategy.** Every milestone has a dedicated test in
`src/filter/tests.rs`, `src/enricher/tests.rs`, or `src/renderer/tests.rs`
per the file-per-stage convention. Fixture graphs are built inline in
tests — no on-disk fixture Cargo projects needed for this feature.

## Risk register

| Risk | Likelihood | Mitigation |
| --- | --- | --- |
| Pattern syntax bikeshed | Medium | Ship glob-with-`*`/`**`; document early; treat regex as a future flag. |
| Collapse produces confusing diagrams (edges from a package look weird) | Medium | Document the semantics with a diagram in `docs/filtering.md`; add a `--collapse-arrow-style` follow-up if users complain. |
| Nested collapse ambiguity | Low | Outer wins, silently; documented. |
| Renderer regression from new `NodeKind` variant | Low | Match is exhaustive; compiler forces us to handle the new variant everywhere. |
| Empty diagram surprise | Low | Stderr warning when the graph is empty post-filter. |

## Rollout order

M1 → M2 → M3 → M4 → M5 → (M7 alongside) → M6 optional.

M1 and M2 are small enough to land in one PR each. M3+M4 should land
together — collapsed nodes without renderer support produces broken
PlantUML. M5 gates the whole feature behind CLI flags so it stays
inert until users opt in.

## Definition of done

- `cargo run -- example/src --exclude '**::internal'` drops matching
  modules and their edges.
- `cargo run -- example/src --collapse example::sub` emits an empty
  `package "sub"` block, with all previously-inbound edges terminating
  on that package and no classes inside.
- `cargo test` green, including new tests in `src/filter/tests.rs`,
  `src/enricher/tests.rs`, and `src/renderer/tests.rs`.
- Docs in `docs/` updated; new `docs/filtering.md` documents pattern
  syntax with worked examples.
- Default `cargo run -- example/src` (no filter flags) produces the
  same diagram as today — zero regression when the feature isn't
  used.
