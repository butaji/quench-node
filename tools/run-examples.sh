#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
binary="$root/target/debug/quench-node"

if [[ ! -x "$binary" ]]; then
  cargo build --quiet --manifest-path "$root/Cargo.toml" -p quench-node
fi

run_example() {
  local file="$1"
  echo "example=$file"
  "$binary" "$root/examples/$file"
}

run_example cli-summary.cjs
run_example crypto-file-summary.cjs
run_example http-loopback.cjs
run_example hono-json.cjs
run_example stream-summary.cjs
