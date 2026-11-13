#!/usr/bin/env bash
# The Linux tarball of a build: the release binary, its licence texts and
# a short README, as vessel-app-<label>-linux-x86_64.tar.gz, with
# SHA256SUMS beside it.
#
# Usage (from spikes/native): scripts/dist-linux.sh LABEL OUT_DIR
# The label (0.0.0.0.1, ...) is compiled in through VESSEL_BUILD_LABEL and
# shown in the window's title and the sidebar's foot.
set -euo pipefail
label=${1:?a build label, e.g. 0.0.0.0.1}
out=${2:?an output directory}
here="$(cd "$(dirname "$0")/.." && pwd)"
cd "$here"

[ -f assets/fonts/Geist-Regular.ttf ] || scripts/fonts.sh

VESSEL_BUILD_LABEL="$label" cargo build --release -p vessel-app
target="$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"

name="vessel-app-$label-linux-x86_64"
stage="$(mktemp -d)"
mkdir -p "$stage/$name/licenses" "$out"
cp "$target/release/vessel-app" "$stage/$name/"
cp ../../LICENSE.md "$stage/$name/LICENSE.md"
cp assets/fonts/*-OFL.txt "$stage/$name/licenses/"
cat > "$stage/$name/README.txt" <<README
Vessel $label (Linux, x86_64): the native app, a first build for trying.

Run:   ./vessel-app
Needs: an X11 or Wayland desktop and a Vulkan driver (Mesa's is enough).

It reads the same node book as the vessel command
(~/.config/vessel/nodes.toml). To add a machine, run
\`vessel node pair\` on it and paste the link into "Add a machine".

Environment: VESSEL_APP_THEME=light|dark pins the theme;
VESSEL_CONFIG_DIR points at another configuration directory.

Fonts embedded: Geist and Geist Mono (Vercel), Pretendard; SIL Open Font
License 1.1, texts in licenses/.
README
tar -C "$stage" -czf "$out/$name.tar.gz" "$name"
rm -rf "$stage"
(cd "$out" && sha256sum ./*.tar.gz | sed 's# \./# #' > SHA256SUMS)
ls -l "$out"
cat "$out/SHA256SUMS"
