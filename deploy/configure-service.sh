#!/usr/bin/env bash
# Install the service definition and create private API credentials once.
# Run as root on the genesis-engine guest after bootstrap-host.sh.
set -Eeuo pipefail

if [[ $(id -u) -ne 0 ]]; then
  echo "Run as root on the genesis-engine guest." >&2
  exit 1
fi

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
if [[ ! -f $script_dir/genesis-engine.service ]]; then
  echo "genesis-engine.service is missing beside this script." >&2
  exit 1
fi
if ! id genesis >/dev/null 2>&1; then
  echo "The genesis service account is missing; run bootstrap-host.sh first." >&2
  exit 1
fi

install -d -m 0750 -o root -g genesis /etc/genesis-engine
install -m 0644 "$script_dir/genesis-engine.service" /etc/systemd/system/genesis-engine.service

runtime_env=/etc/genesis-engine/runtime.env
if [[ ! -e $runtime_env ]]; then
  observer_token=$(openssl rand -hex 32)
  admin_token=$(openssl rand -hex 32)
  umask 0077
  printf 'LIFESIM_OBSERVER_TOKEN=%s\nLIFESIM_ADMIN_TOKEN=%s\n' \
    "$observer_token" "$admin_token" >"$runtime_env"
fi
chown root:genesis "$runtime_env"
chmod 0640 "$runtime_env"

systemctl daemon-reload
systemctl enable genesis-engine.service
echo "Service configured. install-release.sh will start the pinned release."
