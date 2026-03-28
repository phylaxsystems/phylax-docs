#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd -- "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
required_node="$(tr -d '[:space:]' < "$repo_root/.node-version")"
current_major="$(node -p 'process.versions.node.split(".")[0]' 2>/dev/null || echo "")"

run_mint() {
  cd "$repo_root"
  if [[ -x "./node_modules/.bin/mint" ]]; then
    exec ./node_modules/.bin/mint "$@"
  fi

  if command -v mint >/dev/null 2>&1; then
    exec mint "$@"
  fi

  echo "Mint CLI not found. Run 'pnpm install' or install mint globally." >&2
  exit 1
}

if [[ "$current_major" == "$required_node" ]]; then
  run_mint "$@"
fi

if [[ -s "${NVM_DIR:-$HOME/.nvm}/nvm.sh" ]]; then
  unset npm_config_prefix
  # shellcheck source=/dev/null
  . "${NVM_DIR:-$HOME/.nvm}/nvm.sh"
  nvm use "$required_node" >/dev/null
  run_mint "$@"
fi

cat <<EOF >&2
This repo requires Node $required_node.x to run Mintlify commands.

Current Node: $(node -v 2>/dev/null || echo "not found")

If you use nvm:
  source "\$HOME/.nvm/nvm.sh"
  nvm use $required_node
  pnpm dev

Supported shortcuts after this change:
  pnpm dev
  npm run dev
  npm start
EOF

exit 1
