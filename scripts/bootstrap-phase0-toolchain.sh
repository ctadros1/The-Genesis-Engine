#!/bin/sh
set -eu

phase0_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
phase0_tmp=$(mktemp -d)
trap 'rm -rf "$phase0_tmp"' EXIT HUP INT TERM

export RUSTUP_HOME="$phase0_repo/.phase0-tools/rustup"
export CARGO_HOME="$phase0_repo/.phase0-tools/cargo"
export PATH="$CARGO_HOME/bin:$PATH"

if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 --silent --show-error --fail \
    https://sh.rustup.rs -o "$phase0_tmp/rustup-init.sh"
  sh "$phase0_tmp/rustup-init.sh" \
    -y --no-modify-path --profile minimal --default-toolchain 1.97.1
fi

rustup toolchain install 1.97.1 --profile minimal --component clippy --component rustfmt
rustc --version
cargo --version

