# Filtering

archviz can shape the generated diagram with three module-path filters,
all opt-in and all repeatable:

| Flag | Effect |
| --- | --- |
| `--include <pattern>` | Keep only nodes whose module path matches at least one include pattern. Omitting the flag means "include everything". |
| `--exclude <pattern>` | Drop nodes whose module path matches. Edges incident to a dropped node are dropped too. |
| `--collapse <pattern>` | Replace matching modules (and every descendant module) with a single empty `package "…" { }` block; redirect edges that used to terminate inside them to the package. |
| `--collapse-depth <N>` | Collapse every module deeper than `N` levels into its ancestor at depth `N`. `N = 0` flattens the entire project into a single root package. Combine with `--collapse` freely — the shorter (outermost) collapse root wins. |

Precedence, applied per node: `exclude` wins over `include`, and
collapse (whether from `--collapse` or `--collapse-depth`) applies only
to nodes that survived both.

**Subtree semantics for collapse.** Unlike include/exclude, a
`--collapse <pattern>` pattern matches the module itself **and** every
descendant module. Both `--collapse enricher` and
`--collapse 'enricher::**'` collapse the same subtree; the shorter form
is preferred. This is by design — collapse is almost always used to
hide a subsystem.

## Pattern syntax

Patterns match a **module path** — the `::`-joined list of directory /
file segments archviz derives from each source file (e.g. `foo::bar`).
They never match a class or trait by name; class filtering is a separate
feature.

| Pattern | Matches |
| --- | --- |
| `a::b` | Exactly the module `a::b`. *Not* `a::b::c`. |
| `a::b::*` | Any direct child of `a::b`. |
| `a::b::**` | `a::b` itself and any descendant. |
| `**::internal` | Any module ending in a segment called `internal`. |
| `**` | Every module (rarely useful on its own). |

- `*` matches exactly one path segment.
- `**` matches zero or more segments.
- No other wildcards. Partial wildcards inside a segment (`foo*`,
  `*bar`) are rejected.

Synthetic nodes (`Vec<String>`, `Box<dyn Trait>`) and std/external
stubs live at the empty module path, so no module pattern ever matches
them. Filtering them requires a different (as-yet-unbuilt) feature.

## Worked examples

Using the sample project in `example/src` (modules `domain`,
`repository`, `service`).

### Exclude an internal module

```sh
cargo run -- example/src --exclude 'repository'
```

The `repository` module and its edges disappear; `domain::User` and
`service::UserService` remain.

### Include only the domain layer

```sh
cargo run -- example/src --include 'domain::**'
```

Only `domain::User` (plus its std stubs) is drawn — everything else is
dropped, including edges that would have crossed into `repository` /
`service`.

### Collapse a subsystem

```sh
cargo run -- example/src --collapse 'repository'
```

The `repository::UserRepository` and `repository::PostgresUserRepository`
classes disappear, replaced by a single empty
`package "repository" { }`. Edges from `service::UserService` into the
old subtree are redirected to that package; internal edges between the
collapsed classes vanish.

### Cap the visible depth

```sh
cargo run -- example/src --collapse-depth 1
```

Any module deeper than one level collapses into its top-level ancestor —
useful as a first pass on an unfamiliar codebase to see just the
top-level architecture before drilling in. Set `--collapse-depth 2` to
keep two nesting levels, and so on.

### Combining

Flags stack, and each is repeatable:

```sh
cargo run -- example/src \
  --include 'crate::**' \
  --exclude '**::tests' \
  --exclude '**::internal' \
  --collapse 'crate::db'
```

## Semantics notes

- **Nested collapses.** If `a::b::**` and `a::b::c::**` both match, the
  **outer** wins; the inner pattern is silently ignored.
- **Empty collapse root.** A fully-literal collapse pattern (e.g.
  `crate::db`) that matches no source-level node still produces an
  empty placeholder package, so you see what you asked for.
- **Self-edges.** Edges that become `X → X` after redirection are
  dropped; duplicates are deduplicated.
- **Empty diagram.** If filters drop every node, archviz prints a
  warning to stderr and emits a valid but empty PlantUML document.

## Default behavior

When none of the three flags is passed, no filter enricher is added to
the pipeline and the output matches archviz's pre-filter behavior
exactly — zero regression.
