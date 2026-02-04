#!/usr/bin/env bash
set -euo pipefail
shopt -s nullglob

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
PYTHON_DIR="$ROOT_DIR/python"
DIST_DIR="$ROOT_DIR/dist"
STAGE_DIR="$DIST_DIR/wheel2deb"
OUTPUT_DIR="$STAGE_DIR/output"

require_cmd() {
  if ! command -v "$1" &>/dev/null; then
    echo "$1 not found. Install it and try again." >&2
    exit 1
  fi
}

require_cmd maturin
require_cmd wheel2deb

mkdir -p "$STAGE_DIR"

# Build wheel into a clean staging directory to avoid mixing with other artifacts.
rm -f "$STAGE_DIR"/*.whl
(
  cd "$PYTHON_DIR"
  maturin build --release --locked --out "$STAGE_DIR"
)

wheels=("$STAGE_DIR"/*.whl)
if [ "${#wheels[@]}" -eq 0 ]; then
  echo "No wheels found in $STAGE_DIR" >&2
  exit 1
fi

(
  cd "$STAGE_DIR"
  wheel2deb
)

echo
echo "Built .deb packages in $OUTPUT_DIR"
