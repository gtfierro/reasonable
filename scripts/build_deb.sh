#!/usr/bin/env bash
set -euo pipefail

if ! command -v cargo &>/dev/null; then
  echo "cargo not found. Install Rust toolchain first." >&2
  exit 1
fi

if ! command -v cargo-deb &>/dev/null; then
  echo "cargo-deb not found. Install with: cargo install cargo-deb" >&2
  exit 1
fi

# Build the Debian package for the CLI crate
cargo deb -p reasonable-cli "$@"

echo
echo "Built .deb packages in target/debian/"
