#!/bin/sh
# The Svelte half's check: compile every component with the Svelte 5
# compiler (errors fail, warnings print), then drive LoginPanel through its
# steps under jsdom. Installs svelte + jsdom into a scratch dir under
# $TMPDIR; nothing is written to the checkout. Run on the build box:
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
