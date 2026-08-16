#!/bin/sh
set -eu

# Select the toolchain every script in this directory builds with.
#
# Two hosts, one rule: **the compiler version is pinned and verified, wherever
# it comes from.** The repo-local toolchain under `.phase0-tools` is preferred
# and is what a developer laptop has after
# `scripts/bootstrap-phase0-toolchain.sh`. `genesis-engine` (the VM that
# `docs/28-server-development-workflow.md` designates as the authoritative
# build and test host) has a system rustup instead, and before this fallback
# existed all nineteen scripts that source this file failed there with
# "run scripts/bootstrap-phase0-toolchain.sh first" - advice that would have
# had a second full toolchain downloaded into the repo to sit beside a working
# one.
#
# The version check is not decoration. Fixtures `0x1e3158a26afd3b39` and
# `0xff9dfcff5dffbf42` and every determinism claim in `scripts/verify-*.sh`
# are statements about a build; a fallback that silently accepted whatever
# `cargo` happened to be on `PATH` would let a different compiler produce a
# checksum that is then reported as a reproduction. So the fallback fails
# closed on a version mismatch rather than warning.

phase0_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
phase0_pinned=$(
  sed -n 's/^channel[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' \
    "$phase0_repo/rust-toolchain.toml"
)
if [ -z "${phase0_pinned:-}" ]; then
  echo "cannot read the pinned channel from rust-toolchain.toml" >&2
  exit 1
fi

if [ -x "$phase0_repo/.phase0-tools/cargo/bin/cargo" ]; then
  export RUSTUP_HOME="$phase0_repo/.phase0-tools/rustup"
  export CARGO_HOME="$phase0_repo/.phase0-tools/cargo"
  export PATH="$CARGO_HOME/bin:$PATH"
elif ! command -v cargo >/dev/null 2>&1; then
  echo "No Rust toolchain found. Either run scripts/bootstrap-phase0-toolchain.sh" >&2
  echo "for a repo-local one, or put a rustup-managed cargo $phase0_pinned on PATH." >&2
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "Phase 0 Rust toolchain is missing; run scripts/bootstrap-phase0-toolchain.sh first." >&2
  exit 1
fi

# `cargo --version` prints "cargo <semver> (<hash> <date>)". Compared against
# the pin rather than against a range: rust-toolchain.toml is honoured
# automatically by a rustup shim, so a mismatch here means the cargo on PATH
# is not a rustup shim, and that is exactly the case worth refusing.
phase0_found=$(cargo --version | awk '{print $2}')
if [ "$phase0_found" != "$phase0_pinned" ]; then
  echo "cargo $phase0_found is on PATH but rust-toolchain.toml pins $phase0_pinned." >&2
  echo "Refusing: every fixture checksum and determinism script here is a claim" >&2
  echo "about a specific build. Install $phase0_pinned via rustup, or run" >&2
  echo "scripts/bootstrap-phase0-toolchain.sh for a repo-local toolchain." >&2
  exit 1
fi
