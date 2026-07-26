#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

(
  cd "$project_dir/frontend"
  pnpm run build
)

dist_dir="$project_dir/dist"
client_dir="$dist_dir/client"
server_dir="$dist_dir/server"

rm -rf "$dist_dir"
mkdir -p "$client_dir" "$server_dir"
cp -R "$project_dir/frontend/build"/. "$client_dir"/
cp "$project_dir/.openai/sites-worker.js" "$server_dir/index.js"

test -f "$server_dir/index.js"
test -f "$client_dir/index.html"
