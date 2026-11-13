#!/bin/sh
# Adds Vessel to this user's application menu: copies the icons into
# ~/.local/share/icons/hicolor and writes ~/.local/share/applications/
# vessel-app.desktop pointing at the vessel-app beside this script (so
# run it again after moving the folder). Touches nothing else; remove the
# two with `./install-desktop-entry.sh --remove`.
set -eu
here="$(cd "$(dirname "$0")" && pwd)"
data="${XDG_DATA_HOME:-$HOME/.local/share}"
entry="$data/applications/vessel-app.desktop"
if [ "${1:-}" = "--remove" ]; then
    rm -f "$entry"
    find "$data/icons/hicolor" -path '*/apps/vessel-app.png' -delete 2>/dev/null || true
    echo "removed $entry and the icons"
    exit 0
fi
for dir in "$here"/icons/hicolor/*/apps; do
    size="$(basename "$(dirname "$dir")")"
    mkdir -p "$data/icons/hicolor/$size/apps"
    cp "$dir/vessel-app.png" "$data/icons/hicolor/$size/apps/vessel-app.png"
done
mkdir -p "$data/applications"
sed "s|{EXEC}|$here/vessel-app|" "$here/vessel-app.desktop.in" >"$entry"
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$data/applications" || true
echo "added $entry"
