#!/usr/bin/env bash
# Bootstrap VM 120 after an owner-approved console session. This deliberately
# does not alter Proxmox, firewall, DNS, or existing host accounts.
set -Eeuo pipefail

if [[ $(id -u) -ne 0 ]]; then
  echo "Run as root on the genesis-engine guest." >&2
  exit 1
fi

if [[ $# -ne 1 || ! -r $1 ]]; then
  echo "Usage: $0 /absolute/path/to/genesis-dev-authorized-key.pub" >&2
  exit 1
fi

public_key_file=$1
if ! ssh-keygen -lf "$public_key_file" >/dev/null 2>&1; then
  echo "The supplied public key is not valid." >&2
  exit 1
fi

apt-get update
DEBIAN_FRONTEND=noninteractive apt-get install -y \
  build-essential ca-certificates curl git pkg-config libssl-dev openssh-server

if ! id genesis >/dev/null 2>&1; then
  useradd --system --create-home --home-dir /var/lib/genesis-engine \
    --shell /usr/sbin/nologin genesis
fi
if ! id genesis-dev >/dev/null 2>&1; then
  useradd --create-home --home-dir /home/genesis-dev --shell /bin/bash genesis-dev
fi
passwd -l genesis-dev >/dev/null 2>&1 || true

install -d -m 0700 -o genesis-dev -g genesis-dev /home/genesis-dev/.ssh
install -m 0600 -o genesis-dev -g genesis-dev "$public_key_file" \
  /home/genesis-dev/.ssh/authorized_keys

install -d -m 0750 -o genesis -g genesis /var/lib/genesis-engine
install -d -m 0755 -o genesis -g genesis /opt/genesis-engine/releases
ln -sfn /opt/genesis-engine/releases /opt/genesis-engine/current

install -d -m 0755 /etc/ssh/sshd_config.d
cat >/etc/ssh/sshd_config.d/60-genesis-dev.conf <<'EOF'
Match User genesis-dev
    PubkeyAuthentication yes
    PasswordAuthentication no
    KbdInteractiveAuthentication no
    AuthenticationMethods publickey
    PermitEmptyPasswords no
    X11Forwarding no
    AllowAgentForwarding no
    AllowTcpForwarding local
EOF
sshd -t
systemctl enable --now ssh.service
systemctl restart ssh.service

echo "Bootstrap complete. Install a pinned release and runtime.env next."
