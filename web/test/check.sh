#!/bin/sh
# The Svelte half's check: compile every component with the Svelte 5
# compiler (errors fail, warnings print), then drive LoginPanel through its
# steps under jsdom, then build a minimal host app (web/test/host/) with
# vite 6 + @sveltejs/vite-plugin-svelte 5 against the packed package — it
# fails on any vite-plugin-svelte WARNING (e.g. a `svelte` field without an
# exports `svelte` condition, #14). Installs everything into a scratch dir
# under $TMPDIR; nothing is written to the checkout. Run on the build box:
#   sc-build web/test/check.sh
set -eu
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
work=$(mktemp -d "${TMPDIR:-/tmp}/stormview-web.XXXXXX")
trap 'rm -rf "$work"' EXIT

cd "$work"
npm init -y >/dev/null
npm pkg set type=module
npm install --silent --no-audit --no-fund svelte@5 jsdom >/dev/null
cp "$here"/*.mjs .
node --conditions=browser check.mjs "$root/web/components"

# --- a host build, the way a storm web UI pulls stormview -------------------
mkdir "$work/host" "$work/pack"
cp "$here"/host/* "$work/host/"
cd "$work/pack"
tgz=$(npm pack --silent "$root")
cd "$work/host"
npm init -y >/dev/null
npm pkg set type=module
npm install --silent --no-audit --no-fund \
  "$work/pack/$tgz" svelte@5 vite@6 @sveltejs/vite-plugin-svelte@5 >/dev/null
out=$(npx vite build 2>&1) || { echo "$out"; echo "FAIL  host build"; exit 1; }
echo "$out" | sed 's/^/  /'
if echo "$out" | grep -q 'vite-plugin-svelte.*WARNING'; then
  echo "FAIL  host build: vite-plugin-svelte warned"
  exit 1
fi
echo "pass  host build (vite 6 + vite-plugin-svelte 5), no plugin warnings"
