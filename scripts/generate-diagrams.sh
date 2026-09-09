#!/usr/bin/env bash
set -euo pipefail

# Regenerates the archviz PlantUML diagrams for the archviz codebase
# itself. Output is written to docs/diagrams/.
#
#   ./scripts/generate-diagrams.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
OUT_DIR="$ROOT_DIR/docs/diagrams"

mkdir -p "$OUT_DIR"

cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -- src/model     --exclude '**::tests' --exclude '**::internal' > "$OUT_DIR/model.puml"
cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -- src/parser    --exclude '**::tests' --exclude '**::internal' > "$OUT_DIR/parser.puml"
cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -- src/enricher  --exclude '**::tests' --exclude '**::internal' > "$OUT_DIR/enricher.puml"
cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -- src/filter    --exclude '**::tests' --exclude '**::internal' > "$OUT_DIR/filter.puml"
cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -- src/renderer  --exclude '**::tests' --exclude '**::internal' > "$OUT_DIR/renderer.puml"
cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -- src/project   --exclude '**::tests' --exclude '**::internal' > "$OUT_DIR/project.puml"

cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -- src \
  --exclude '**::tests' --exclude '**::internal' \
  --collapse model --collapse parser --collapse enricher \
  --collapse filter --collapse renderer --collapse project \
  > "$OUT_DIR/overview.puml"