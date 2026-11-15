#!/usr/bin/env bash
# The app's faces, as the page has them (DESIGN.md section 5): Geist,
# Geist Mono and Pretendard, all SIL Open Font License 1.1. GPUI reads
# TrueType and OpenType files, not the page's woff2 subsets, so the
# static weights the design uses (300, 400, 500, 600; 300 for Paseo's
# desktop screen title, `headers/screen-title.tsx`) are taken from the
# packages that ship them: `geist` (Vercel's own package, fetched with
# `npm pack` into a scratch directory) and `pretendard` (already in the
# page's node_modules after `npm ci` in web/). They land in
# assets/fonts/, which is not committed; the build embeds them.
#
# Usage: scripts/fonts.sh [scratch-dir]   (run from spikes/native)
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
web="$here/../../web/node_modules"
scratch="${1:-$(mktemp -d)}"
out="$here/assets/fonts"
mkdir -p "$out" "$scratch"

geist_version="1.9.2"
if [ ! -f "$scratch/package/dist/fonts/geist-sans/Geist-Regular.ttf" ]; then
  (cd "$scratch" && npm pack "geist@$geist_version" --silent >/dev/null && tar -xzf "geist-$geist_version.tgz")
fi
for weight in Light Regular Medium SemiBold; do
  cp "$scratch/package/dist/fonts/geist-sans/Geist-$weight.ttf" "$out/"
done
# The code face's regular, and its bold and italics for the terminal's
# SGR 1 and 3 (ADR 0073; GPUI synthesizes neither).
for face in Regular Bold Italic BoldItalic; do
  cp "$scratch/package/dist/fonts/geist-mono/GeistMono-$face.ttf" "$out/"
done

if [ ! -d "$web/pretendard" ]; then
  echo "run npm ci in web/ first (Pretendard comes from the page's packages)" >&2
  exit 1
fi
for weight in Light Regular Medium SemiBold; do
  cp "$web/pretendard/dist/public/static/Pretendard-$weight.otf" "$out/"
done
# The licence texts the page already ships for the same faces.
cp "$here/../../web/public/licenses/Geist-OFL.txt" "$here/../../web/public/licenses/GeistMono-OFL.txt" \
  "$here/../../web/public/licenses/Pretendard-OFL.txt" "$out/"
ls -l "$out"
