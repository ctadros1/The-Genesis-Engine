#!/usr/bin/env bash
# Enable private-LAN Observer hosting through Caddy and mDNS. Run as root on
# the genesis-engine guest after configure-service.sh and install-release.sh.
set -Eeuo pipefail

if [[ $(id -u) -ne 0 ]]; then
  echo "Run as root on the genesis-engine guest." >&2
  exit 1
fi

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
fqdn=${1:-genesisengine.local}
if [[ $fqdn != genesisengine.local ]]; then
  echo "This private hosting configuration is pinned to genesisengine.local." >&2
  exit 1
fi
if [[ ! -d /opt/genesis-engine/current/observer ]]; then
  echo "No installed Observer release found." >&2
  exit 1
fi

hostnamectl set-hostname genesisengine
apt-get update
DEBIAN_FRONTEND=noninteractive apt-get install -y avahi-daemon caddy

install -m 0644 "$script_dir/Caddyfile" /etc/caddy/Caddyfile
# The release staging directory is private to the service account; the
# browser-facing static assets need traversal permission for Caddy only.
chmod 0755 "$(readlink -f /opt/genesis-engine/current)" \
  "$(readlink -f /opt/genesis-engine/current)/observer"

caddy validate --config /etc/caddy/Caddyfile --adapter caddyfile
systemctl enable --now avahi-daemon.service caddy.service
systemctl restart caddy.service

echo "Hosting enabled at https://${fqdn}. Trust Caddy's local root CA on clients."
