#!/usr/bin/env bash
# Provision the restricted SSH developer account with the pinned Rust toolchain.
# Run as root on the genesis-engine guest after bootstrap-host.sh.
set -Eeuo pipefail

if [[ $(id -u) -ne 0 ]]; then
  echo "Run as root on the genesis-engine guest." >&2
  exit 1
fi
if ! id genesis-dev >/dev/null 2>&1; then
  echo "The genesis-dev account is missing; run bootstrap-host.sh first." >&2
  exit 1
fi

if [[ ! -x /home/genesis-dev/.cargo/bin/rustup ]]; then
  curl --fail --location --proto '=https' --tlsv1.2 https://sh.rustup.rs -o /tmp/genesis-rustup-init.sh
  chown genesis-dev:genesis-dev /tmp/genesis-rustup-init.sh
  runuser -u genesis-dev -- env HOME=/home/genesis-dev sh /tmp/genesis-rustup-init.sh \
    -y --profile minimal --default-toolchain 1.97.1
  rm -f /tmp/genesis-rustup-init.sh
fi

install -d -m 0755 -o genesis-dev -g genesis-dev /home/genesis-dev/src
echo "Development account configured. Clone the repository under /home/genesis-dev/src."
