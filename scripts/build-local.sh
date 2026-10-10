#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd -- "$project_dir"

if ! command -v cargo >/dev/null 2>&1; then
    echo "Cargo is required. Install Rust stable from https://rustup.rs/." >&2
    exit 1
fi

cargo build --workspace --release --locked
