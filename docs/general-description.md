# General Description

**archviz** is a command-line application that generates UML class diagrams
from the source code of a Rust project.

Given the path to a Rust project (or any directory containing `*.rs` files),
archviz:

1. Walks the directory tree and reads every Rust source file.
2. Parses each file's abstract syntax tree (AST) to discover structs,
   traits, enums, and the relationships between them (fields, generic type
   parameters, trait implementations, etc.).
3. Enriches the resulting model with additional, synthesized information —
   for example, giving compound types such as `Vec<String>` or `Box<dyn
   Trait>` their own diagram nodes and relationships, even though they have
   no explicit `struct`/`trait` definition in the source.
4. Renders the final model as UML diagram markup — currently
   [PlantUML](https://plantuml.com/) class-diagram syntax — which can be fed
   into a PlantUML renderer (CLI, IDE plugin, or web service) to produce an
   actual diagram image.

## Usage

```sh
cargo run -- <path-to-rust-project>
# e.g.
cargo run -- example/src
```

The tool prints the generated PlantUML source to standard output. Redirect
it to a `.puml` file and render it with any PlantUML-compatible tool to get
a visual diagram:

```sh
cargo run -- example/src > diagram.puml
plantuml diagram.puml   # produces diagram.png
```

## Why

Understanding the structure of an unfamiliar (or evolving) Rust codebase —
what types exist, how they relate to each other through composition and
trait implementation, and how they are organized into modules — is easier
with a visual diagram than by reading source files one by one. archviz
automates the tedious part of extracting that structure so it can be kept
up to date as the code changes, rather than maintained by hand.

## Scope

archviz focuses on the *static structure* of the code — types, fields, and
trait relationships — not on runtime behavior, control flow, or sequence
diagrams. Its output format is decoupled from the analysis: today it emits
PlantUML, but the pipeline is designed so that other diagram/renderer
back ends could be added without changing how the code is parsed or
analyzed (see `architecture.md` and `design-patterns.md`).
