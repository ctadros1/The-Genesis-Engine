#!/bin/sh
set -eu

phase0_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
export RUSTUP_HOME="$phase0_repo/.phase0-tools/rustup"
export CARGO_HOME="$phase0_repo/.phase0-tools/cargo"
export PATH="$CARGO_HOME/bin:$PATH"

if ! command -v cargo >/dev/null 2>&1; then
  echo "Phase 0 Rust toolchain is missing; run scripts/bootstrap-phase0-toolchain.sh first." >&2
  exit 1
fi
