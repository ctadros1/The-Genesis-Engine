#!/usr/bin/env bash
# Build and install one immutable release checkout on the guest. Run as root.
set -Eeuo pipefail

if [[ $(id -u) -ne 0 ]]; then
  echo "Run as root on the genesis-engine guest." >&2
  exit 1
fi
if [[ $# -ne 2 ]]; then
  echo "Usage: $0 <git-url> <commit>" >&2
  exit 1
fi

git_url=$1
commit=$2
release_dir="/opt/genesis-engine/releases/$commit"
node_version="v25.6.0"
node_archive="node-${node_version}-linux-x64.tar.xz"
node_sha256="f61908298ba1c8e1802ac00283cee678e0eb4035e1c74f094b06b1620423fcf2"

if [[ ! $commit =~ ^[0-9a-f]{7,64}$ ]]; then
  echo "Commit must be a hexadecimal Git commit ID." >&2
  exit 1
fi
if [[ -e $release_dir ]]; then
  echo "Release already exists: $release_dir" >&2
  exit 1
fi

install -d -m 0755 /opt/genesis-engine/releases /opt/genesis-engine/toolchains
if [[ ! -x /opt/genesis-engine/toolchains/node-${node_version}-linux-x64/bin/node ]]; then
  archive_path="/tmp/$node_archive"
  curl --fail --location --proto '=https' --tlsv1.2 \
    "https://nodejs.org/dist/$node_version/$node_archive" -o "$archive_path"
  echo "$node_sha256  $archive_path" | sha256sum --check --status
  tar -C /opt/genesis-engine/toolchains -xf "$archive_path"
  rm -f "$archive_path"
fi
ln -sfn "/opt/genesis-engine/toolchains/node-${node_version}-linux-x64/bin/node" /usr/local/bin/node
ln -sfn "/opt/genesis-engine/toolchains/node-${node_version}-linux-x64/bin/npm" /usr/local/bin/npm
ln -sfn "/opt/genesis-engine/toolchains/node-${node_version}-linux-x64/bin/npx" /usr/local/bin/npx

if ! command -v rustup >/dev/null 2>&1; then
  curl --fail --location --proto '=https' --tlsv1.2 https://sh.rustup.rs -o /tmp/rustup-init.sh
  sh /tmp/rustup-init.sh -y --profile minimal --default-toolchain 1.97.1
  rm -f /tmp/rustup-init.sh
fi
export PATH="$HOME/.cargo/bin:/opt/genesis-engine/toolchains/node-${node_version}-linux-x64/bin:$PATH"

tmp_dir=$(mktemp -d /opt/genesis-engine/releases/.staging.XXXXXX)
trap 'rm -rf "$tmp_dir"' EXIT
git_ssh_command="ssh -i /home/genesis-dev/.ssh/github_deploy -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new"
GIT_SSH_COMMAND="$git_ssh_command" git clone --no-checkout "$git_url" "$tmp_dir"
git -C "$tmp_dir" checkout --detach "$commit"
actual_commit=$(git -C "$tmp_dir" rev-parse HEAD)
if [[ $actual_commit != "$commit"* ]]; then
  echo "Checked out commit does not match requested prefix." >&2
  exit 1
fi

pushd "$tmp_dir" >/dev/null
cargo build --locked --release -p sim-server -p sim-cli
pushd apps/observer >/dev/null
npm ci
npm run build
popd >/dev/null
install -d bin observer
install -m 0755 target/release/lifesim-server bin/lifesim-server
install -m 0755 target/release/lifesim bin/lifesim
cp -a apps/observer/dist/. observer/
popd >/dev/null

chown -R genesis:genesis "$tmp_dir"
chmod 0755 "$tmp_dir" "$tmp_dir/observer"
mv "$tmp_dir" "$release_dir"
trap - EXIT
ln -sfn "$release_dir" /opt/genesis-engine/current
systemctl daemon-reload
systemctl restart genesis-engine.service
echo "Installed and started $actual_commit"
