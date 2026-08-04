#!/bin/sh
set -eu

phase0_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
phase0_id=${1:-phase0-local-$(date -u +%Y%m%dT%H%M%SZ)}

cd "$phase0_repo"
scripts/run-phase0-benchmarks.sh "$phase0_id"
scripts/run-renderer-benchmarks.sh "$phase0_id"
printf 'complete Phase 0 raw record: %s\n' "$phase0_repo/benchmarks/raw/$phase0_id"
