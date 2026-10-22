#!/usr/bin/env bash
# The release, as one flat directory (EPIC-20 § Release, 6a).
#
#   scripts/dist.sh <out-dir> [--tag vX.Y.Z]
#
# Writes: bowery.js bowery.css bowery_wasm.js bowery_wasm_bg.wasm
#         LICENSE-MIT LICENSE-APACHE README.md package.json MANIFEST.sha256
#
# The same directory is the npm package and the tarball's contents, so one
# build is published three ways. Every check runs before anything is
# written, and a failed run leaves no out-dir behind (pkarena0-web's
# scripts/embed-bundle.mjs checks, without its bundler: bowery has one JS
# file and needs none).
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
out="${1:-}"
tag=""
if [ "${2:-}" = "--tag" ]; then tag="${3:-}"; fi
die() { echo "dist: $*" >&2; exit 1; }

[ -n "$out" ] || die "usage: scripts/dist.sh <out-dir> [--tag vX.Y.Z]"
[ ! -e "$out" ] || die "$out exists; remove it first"

# ── Checks, all before any write ────────────────────────────────────────────
crate=$(sed -n 's/^version = "\(.*\)"$/\1/p' "$root/Cargo.toml" | head -1)
npm=$(sed -n 's/^  "version": "\(.*\)",$/\1/p' "$root/npm/package.json" | head -1)
[ -n "$crate" ] || die "no version in Cargo.toml"
[ "$crate" = "$npm" ] || die "Cargo.toml is $crate but npm/package.json is $npm"
if [ -n "$tag" ] && [ "$tag" != "v$crate" ]; then
  die "tag $tag is not the crate version $crate; bump both or tag v$crate"
fi
for file in pkg/bowery_wasm.js pkg/bowery_wasm_bg.wasm; do
  [ -f "$root/$file" ] || die "$file is missing; run make pkg first"
done
gate=$(sed -n 's/^WASM_GATE := \([0-9]*\)$/\1/p' "$root/Makefile")
bytes=$(wc -c < "$root/pkg/bowery_wasm_bg.wasm" | tr -d ' ')
[ "$bytes" -le "$gate" ] || die "bowery_wasm_bg.wasm is $bytes bytes, over the $gate gate"
# Comments stripped first: the stylesheet's header says it has no url().
if perl -0pe 's{/\*.*?\*/}{}gs' "$root/js/bowery.css" | grep -Eq '@import|url\('; then
  die "bowery.css references something outside itself"
fi

# ── Write, then the manifest over what was written ──────────────────────────
mkdir -p "$out"
cp "$root/js/bowery.js" "$root/js/bowery.css" "$root/pkg/bowery_wasm.js" "$root/pkg/bowery_wasm_bg.wasm" \
  "$root/LICENSE-MIT" "$root/LICENSE-APACHE" "$root/README.md" "$root/npm/package.json" "$out/"
(
  cd "$out"
  if command -v sha256sum > /dev/null; then sum="sha256sum"; else sum="shasum -a 256"; fi
  $sum LICENSE-APACHE LICENSE-MIT README.md bowery.css bowery.js bowery_wasm.js bowery_wasm_bg.wasm package.json > MANIFEST.sha256
)
echo "dist: v$crate -> $out (wasm $bytes bytes)"
