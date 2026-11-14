#!/usr/bin/env bash
# Build every artifact of the native app for one label, on Linux.
#
#   spikes/release/build.sh LABEL [TARGET...] [--publish]
#
# TARGETs: windows-x64, linux-x64 (both when none is named).
# --publish also writes the channel's signed manifest (<dist>/<channel>.json),
# which tells every installed copy that LABEL is the newest build: only
# when the coordinator gives the word.
#
# Writes "$VESSEL_DIST_DIR/LABEL/" as one set: everything is built into a
# hidden directory beside it (keeping the other targets' files of the
# label, if any), checked with `sha256sum -c`, and only then renamed into
# place, so the directory the owner downloads from never holds a
# half-written set. The last step checks the sums again where they landed.
# Contents:
#   vessel-LABEL-windows-x64.zip        Vessel.exe, LICENSE.md, README.txt,
#                                       licenses/ (the embedded fonts' OFL,
#                                       the agents' marks' licences)
#   vessel-LABEL-windows-x64-setup.exe  per-user installer (NSIS)
#   vessel-LABEL-windows-x64.exe        the executable alone: the update
#   vessel-LABEL-linux-x64.tar.gz       vessel-app, LICENSE.md, README.txt,
#                                       licenses/
#   vessel-LABEL-linux-x64              the executable alone: the update
#   SHA256SUMS (and SHA256SUMS.minisig when a signing key is given)
#
# Environment (no host names or paths live in this file):
#   VESSEL_DIST_DIR        required: the dist root (the feed's base)
#   VESSEL_TOOLS_DIR       required for Windows: cargo-xwin (bin/), its
#                          cache (xwin-cache/), fxc (fxc/), wine's prefix
#                          (wineprefix/); tools/setup-windows-cross.sh fills it
#   VESSEL_UPDATE_FEED_FILE  a file holding the feed's https:// base (one
#                          line), compiled into the app; the address carries
#                          a token, so it is read from a file and never
#                          printed by this script (VESSEL_UPDATE_FEED, the
#                          address itself, is accepted too)
#   VESSEL_UPDATE_KEY      the release key's public base64 line, compiled in
#   VESSEL_UPDATE_SECRET   the release key's secret file: signs SHA256SUMS
#                          and, with --publish, the manifest
#   VESSEL_UPDATE_CHANNEL  the channel (preview)
#   VESSEL_APP_WORKSPACE, VESSEL_APP_PACKAGE, VESSEL_APP_BIN
#                          what to build (spikes/native, vessel-app,
#                          vessel-app); the release workspace's
#                          vessel-release-hello / vessel-hello tries the
#                          pipeline without the app
#   VESSEL_SYMBOLS_DIR     where each label's Windows .pdb and Linux
#                          .debug are kept (vessel-app-symbols beside the
#                          dist root)
#   JOBS                   cargo's -j (6: /data is one spinning disk)
#
# Why one script for every target: the owner gets a new build after every
# round of feedback, and each must carry the same label, the same update
# settings and the same checks on every system.
set -euo pipefail

usage() {
    sed -n '2,10p' "$0" >&2
    exit 2
}

[ $# -ge 1 ] || usage
label="$1"
shift
[[ "$label" =~ ^[0-9]+(\.[0-9]+){0,7}$ ]] || { echo "label must be dot-separated numbers: $label" >&2; exit 2; }

targets=()
publish=0
for arg in "$@"; do
    case "$arg" in
        windows-x64 | linux-x64) targets+=("$arg") ;;
        --publish) publish=1 ;;
        *) usage ;;
    esac
done
[ ${#targets[@]} -gt 0 ] || targets=(windows-x64 linux-x64)

here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
: "${VESSEL_DIST_DIR:?VESSEL_DIST_DIR names the dist root}"
app_ws="${VESSEL_APP_WORKSPACE:-$repo/spikes/native}"
app_pkg="${VESSEL_APP_PACKAGE:-vessel-app}"
app_bin="${VESSEL_APP_BIN:-vessel-app}"
channel="${VESSEL_UPDATE_CHANNEL:-preview}"
jobs="${JOBS:-6}"
final="$VESSEL_DIST_DIR/$label"
partial="$VESSEL_DIST_DIR/.$label.partial.$$"
out="$partial"
mkdir -p "$out"
if [ -d "$final" ]; then
    # Hard links: no copy of tens of MiB; every file this run writes is
    # removed first (fresh) so the published one is never written through.
    cp -al "$final/." "$out/"
fi
rm -f "$out/SHA256SUMS" "$out/SHA256SUMS.minisig"

# The update settings go into the app together or not at all.
if [ -n "${VESSEL_UPDATE_FEED_FILE:-}" ]; then
    VESSEL_UPDATE_FEED="$(head -n 1 "$VESSEL_UPDATE_FEED_FILE" | tr -d '[:space:]')"
    [ -n "$VESSEL_UPDATE_FEED" ] || { echo "VESSEL_UPDATE_FEED_FILE is empty" >&2; exit 2; }
fi
if [ -n "${VESSEL_UPDATE_FEED:-}" ] || [ -n "${VESSEL_UPDATE_KEY:-}" ]; then
    : "${VESSEL_UPDATE_FEED:?VESSEL_UPDATE_KEY is set, VESSEL_UPDATE_FEED is not}"
    : "${VESSEL_UPDATE_KEY:?VESSEL_UPDATE_FEED is set, VESSEL_UPDATE_KEY is not}"
    export VESSEL_UPDATE_FEED VESSEL_UPDATE_KEY
    export VESSEL_UPDATE_CHANNEL="$channel"
    # Never the address: it carries the feed's token.
    echo "updates: channel $channel, from the feed compiled in"
else
    echo "updates: none compiled in (VESSEL_UPDATE_FEED and VESSEL_UPDATE_KEY unset)"
fi
export VESSEL_BUILD_LABEL="$label"

stage="$(mktemp -d)"
# The partial set only: after the rename it no longer exists, and the
# published directory is never what is removed.
trap 'rm -rf "$stage" "$partial"' EXIT

# The app embeds its faces; they are fetched, not committed.
# Checked by the newest face the kit embeds (Light, 300, added 2026-11-13),
# so a tree fetched before it fetches again.
if [ -x "$app_ws/scripts/fonts.sh" ] && [ ! -f "$app_ws/assets/fonts/Pretendard-Light.otf" ]; then
    (cd "$app_ws" && scripts/fonts.sh "$stage/fonts")
fi

target_dir="$(cd "$app_ws" && cargo metadata --format-version 1 --no-deps |
    python3 -I -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')"

# What the README says about updates must be true of this build: the
# settings compiled in, and the app linking the updater that reads them.
links_updater=0
(cd "$app_ws" && cargo metadata --format-version 1 --locked 2>/dev/null) |
    grep -q '"name":"vessel-app-updater"' && links_updater=1
if [ -n "${VESSEL_UPDATE_FEED:-}" ] && [ "$links_updater" = 1 ]; then
    updates_ko="이 빌드부터 스스로 업데이트합니다. 시작 직후와 한 시간마다 새 빌드를 확인해 뒤에서 받아 두고, 다음에 Vessel을 열면 새 빌드로 시작합니다(받아 두면 화면 아래에 알려 줍니다). 0.0.0.0.2 이하를 쓰던 경우 이번 한 번만 이 빌드를 직접 설치하세요. 자동 확인을 끄려면 노드 목록과 같은 폴더의 app-updates.json 에서 automatic 값을 false로 바꿉니다(한 번만 끄려면 VESSEL_APP_NO_UPDATE=1)."
    updates_en="From this build on, Vessel updates itself: it checks shortly after start and hourly, downloads a new build in the background, and opens it the next time Vessel starts (the window's foot says when one is ready). Coming from 0.0.0.0.2 or earlier, install this build by hand once. To turn the automatic checks off, set automatic to false in app-updates.json beside the node book (VESSEL_APP_NO_UPDATE=1 turns updates off for one run)."
else
    updates_ko="이 빌드는 스스로 업데이트하지 않습니다. 새 빌드는 새로 받아 같은 방법으로 설치하면 덮어씁니다."
    updates_en="This build does not update itself. Install a new build the same way; it replaces this one."
fi

# The licences of what the app embeds that is not Vessel's, one text per
# file in licenses/:
# - The SIL Open Font License asks that each copy of the fonts carry the
#   licence; the app embeds Geist, Geist Mono and Pretendard (fonts.sh
#   puts their texts beside the fonts).
# - The agents' marks (docs/design/agent-marks/README.md): every text
#   beside the marks is carried, so a mark that comes with a licence brings
#   it without a change here. 0.0.0.0.10-0.0.0.0.14 embedded Gemini marks
#   from Google repositories under Apache-2.0 (section 4(a): recipients of
#   the object form get the licence), hence google-Apache-2.0.txt. The
#   desktop track is moving Gemini to Google's own file from Google's CDN,
#   which is not an Apache-2.0 work; once no embedded mark needs that text
#   it can leave the folder. The folder may then hold no text at all, so
#   the copy takes what is there and never fails on an empty match. (The
#   restricted marks keep only path data and ship no text.)
licenses() {
    mkdir -p "$1/licenses"
    cp "$app_ws"/assets/fonts/*-OFL.txt "$1/licenses/"
    local text
    for text in "$repo"/docs/design/agent-marks/*.txt; do
        [ -e "$text" ] || continue
        cp "$text" "$1/licenses/"
    done
}

# Every icon, from the owner's mark (docs/design/brand/, whose README says
# how it is used), into this run's stage; nothing generated is committed.
# resvg rasterises (tools/make-icons.py says why not ImageMagick's).
icons="$stage/icons"
PATH="${VESSEL_TOOLS_DIR:+$VESSEL_TOOLS_DIR/bin:}$PATH" \
    python3 -I "$here/tools/make-icons.py" "$repo/docs/design/brand/vessel-mark.svg" "$icons"

readme() {
    sed -e "s/{LABEL}/$label/g" -e "s/{UPDATES_KO}/$updates_ko/" -e "s/{UPDATES_EN}/$updates_en/" "$1"
}

# Every DLL a Windows 10 (1703 or later) or 11 ships in System32, as the
# app imports them today. A new import fails the build until someone has
# checked it is there on a clean Windows or bundled it.
windows_dlls='advapi32.dll api-ms-win-core-synch-l1-2-0.dll api-ms-win-core-winrt-error-l1-1-0.dll
api-ms-win-core-winrt-l1-1-0.dll api-ms-win-core-winrt-string-l1-1-0.dll api-ms-win-shcore-scaling-l1-1-1.dll
bcrypt.dll bcryptprimitives.dll combase.dll comctl32.dll crypt32.dll d3d11.dll d3dcompiler_47.dll dcomp.dll
dwmapi.dll dwrite.dll dxgi.dll gdi32.dll gdiplus.dll icuuc.dll imm32.dll kernel32.dll ncrypt.dll ntdll.dll ole32.dll
oleaut32.dll secur32.dll shell32.dll shlwapi.dll uiautomationcore.dll user32.dll userenv.dll uxtheme.dll
version.dll winmm.dll ws2_32.dll'

check_imports() {
    local exe="$1" dll unknown=0 headers
    # Read once into a variable: `grep -q` on a pipe would close it early
    # and, under pipefail, fail the check of a good executable.
    headers="$(x86_64-w64-mingw32-objdump -p "$exe")"
    for dll in $(sed -n 's/^\s*DLL Name: //p' <<<"$headers" | tr 'A-Z' 'a-z' | sort -u); do
        if ! grep -qw -- "$dll" <<<"$windows_dlls"; then
            echo "imports $dll, which is not on the list of DLLs Windows ships" >&2
            unknown=1
        fi
    done
    [ "$unknown" = 0 ] || exit 1
    # The icon and the version block (build_windows links them) beside
    # GPUI's manifest: an exe without them shows a blank icon and its
    # binary's name in Task Manager.
    local resources kind
    resources="$(llvm-readobj --coff-resources "$exe")"
    for kind in 'ICON (ID 3)' 'GROUP_ICON (ID 14)' 'VERSIONINFO (ID 16)' 'MANIFEST (ID 24)'; do
        grep -qF "Type: $kind" <<<"$resources" || { echo "$exe has no $kind resource" >&2; exit 1; }
    done
    grep -q 'Subsystem.*(Windows GUI)' <<<"$headers" ||
        { echo "$exe is not a Windows GUI executable (a console would open behind it)" >&2; exit 1; }
}

build_windows() {
    : "${VESSEL_TOOLS_DIR:?VESSEL_TOOLS_DIR names the Windows build tools}"
    local tools="$VESSEL_TOOLS_DIR" triple=x86_64-pc-windows-msvc
    # cargo-xwin: Microsoft's CRT and SDK libraries, linked by lld-link.
    # The CRT is linked statically, so the app needs no Visual C++
    # redistributable; the GUI subsystem is set at link time, so no console
    # opens behind the window whatever the app's source says.
    export XWIN_CACHE_DIR="$tools/xwin-cache"
    export GPUI_FXC_PATH="$here/tools/fxc-wine" VESSEL_FXC_DIR="$tools/fxc" WINEPREFIX="$tools/wineprefix"
    # The icon and the version block, compiled to a .res the linker takes
    # as an input. Its path is fixed (in the target directory) so the flags,
    # and with them every crate's fingerprint, stay the same build to build;
    # a new label relinks the app anyway (the label is compiled into it).
    local res_dir="$target_dir/vessel-release-resources"
    mkdir -p "$res_dir"
    cp "$icons/vessel.ico" "$res_dir/vessel.ico"
    local version
    version="$(python3 -I -c 'import sys; p = ([0, 0, 0, 0] + [int(x) for x in sys.argv[1].split(".")])[-4:]; print(",".join(str(min(x, 65535)) for x in p))' "$label")"
    sed -e "s/{LABEL}/$label/g" -e "s/{VERSION}/$version/g" "$here/windows/app.rc.in" >"$res_dir/app.rc"
    # Relative names: llvm-rc reads an argument starting with `/` as a switch.
    (cd "$res_dir" && llvm-rc /nologo /fo app.res app.rc)
    export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS="-C target-feature=+crt-static -C link-arg=/IGNORE:4099 -C link-arg=/SUBSYSTEM:WINDOWS -C link-arg=/ENTRY:mainCRTStartup -C link-arg=$res_dir/app.res"
    (cd "$app_ws" && "$tools/bin/cargo-xwin" xwin build --release --locked --target "$triple" -p "$app_pkg" -j "$jobs")
    local exe="$target_dir/$triple/release/$app_bin.exe"
    check_imports "$exe"

    rm -f "$out"/vessel-"$label"-windows-x64*
    local dir="$stage/windows/Vessel"
    mkdir -p "$dir"
    cp "$exe" "$dir/Vessel.exe"
    cp "$repo/LICENSE.md" "$dir/LICENSE.md"
    licenses "$dir"
    readme "$here/windows/README.txt" | sed 's/$/\r/' >"$dir/README.txt"
    (cd "$stage/windows" && TZ=UTC zip -q -X -r "$out/vessel-$label-windows-x64.zip" Vessel)
    cp "$exe" "$out/vessel-$label-windows-x64.exe"
    makensis -V2 -DLABEL="$label" -DSTAGE="$dir" -DICONS="$icons" \
        -DOUTFILE="$out/vessel-$label-windows-x64-setup.exe" "$here/windows/installer.nsi"
    echo "windows-x64: $(du -h "$exe" | cut -f1) executable"
    keep_symbols "$exe"
}

# The Windows build's .pdb, kept per label in the symbols directory with
# the exe's SHA-256 beside it, never in the dist folder: it is about 20 MB
# and only for naming a panic's frames (vessel-app-log writes them as
# `exe+0x<rva>`; `llvm-symbolizer --obj=<exe> --relative-address 0x<rva>`
# with this .pdb beside the exe). Taken at build time because the next
# build overwrites the one in the target directory.
keep_symbols() {
    local exe="$1"
    local pdb="${exe%/*}/${app_bin//-/_}.pdb"
    local dir="${VESSEL_SYMBOLS_DIR:-$(dirname "$VESSEL_DIST_DIR")/vessel-app-symbols}/$label"
    [ -f "$pdb" ] || { echo "no $pdb beside the exe" >&2; exit 1; }
    mkdir -p "$dir"
    cp "$pdb" "$dir/"
    (cd "$dir" && sha256sum "${pdb##*/}" >SHA256SUMS)
    printf '%s  %s\n' "$(sha256sum <"$exe" | cut -d' ' -f1)" "vessel-$label-windows-x64.exe" >>"$dir/SHA256SUMS"
    echo "windows-x64: ${pdb##*/} kept in the symbols directory under $label"
}

# The Linux build's debug information (the line tables of the native
# profile's `debug = "line-tables-only"`), split out of the executable and
# kept per label in the symbols directory, as the Windows .pdb is: what is
# shipped stays the size it was without them (its symbols stay, as
# before), and `--add-gnu-debuglink` names the kept file, so a debugger or
# `addr2line -e vessel-app.debug` gives a panic's frames their file and
# line.
split_debug() {
    local built="$1" shipped="$2"
    local dir="${VESSEL_SYMBOLS_DIR:-$(dirname "$VESSEL_DIST_DIR")/vessel-app-symbols}/$label"
    local debug="$dir/$app_bin.debug"
    mkdir -p "$dir"
    objcopy --only-keep-debug "$built" "$debug"
    objcopy --strip-debug --add-gnu-debuglink="$debug" "$built" "$shipped"
    (cd "$dir" && sha256sum "$app_bin.debug" >SHA256SUMS-linux)
    printf '%s  %s\n' "$(sha256sum <"$shipped" | cut -d' ' -f1)" "vessel-$label-linux-x64" >>"$dir/SHA256SUMS-linux"
    echo "linux-x64: $(du -h "$built" | cut -f1) built, $(du -h "$shipped" | cut -f1) shipped, $app_bin.debug kept in the symbols directory under $label"
}

build_linux() {
    (cd "$app_ws" && cargo build --release --locked -p "$app_pkg" -j "$jobs")
    local name="vessel-$label-linux-x64"
    rm -f "$out/$name" "$out/$name.tar.gz"
    local dir="$stage/linux/$name"
    mkdir -p "$dir"
    local bin="$stage/linux/$app_bin"
    split_debug "$target_dir/release/$app_bin" "$bin"
    # Not `vessel`: that is the command line's name.
    cp "$bin" "$dir/vessel-app"
    cp "$repo/LICENSE.md" "$dir/LICENSE.md"
    licenses "$dir"
    readme "$here/linux/README.txt" >"$dir/README.txt"
    # The menu entry and its icons (the freedesktop sizes), and the script
    # that installs them for the user who unpacked the tarball.
    local size
    for size in 16 24 32 48 64 128 256 512; do
        mkdir -p "$dir/icons/hicolor/${size}x${size}/apps"
        cp "$icons/tile-$size.png" "$dir/icons/hicolor/${size}x${size}/apps/vessel-app.png"
    done
    cp "$here/linux/vessel-app.desktop.in" "$here/linux/install-desktop-entry.sh" "$dir/"
    tar --sort=name --owner=0 --group=0 --numeric-owner --mtime='@0' -C "$stage/linux" -czf "$out/$name.tar.gz" "$name"
    cp "$bin" "$out/$name"
    chmod 0755 "$out/$name"
    echo "linux-x64: $(du -h "$bin" | cut -f1) executable"
}

for target in "${targets[@]}"; do
    case "$target" in
        windows-x64) build_windows ;;
        linux-x64) build_linux ;;
    esac
done

feed_tool() {
    (cd "$here" && cargo run --quiet --locked -p vessel-app-feed -- "$@")
}

if [ -n "${VESSEL_UPDATE_SECRET:-}" ]; then
    # The label names the set in the signature's comment: the files sit in
    # a hidden staging directory now, whose name must not be signed.
    feed_tool sums --key "$VESSEL_UPDATE_SECRET" --label "$label" "$out"
else
    (cd "$out" && find . -maxdepth 1 -type f ! -name 'SHA256SUMS*' ! -name '.*' -printf '%f\n' | sort |
        xargs sha256sum >SHA256SUMS)
    echo "SHA256SUMS unsigned (VESSEL_UPDATE_SECRET unset)"
fi
(cd "$out" && sha256sum --quiet -c SHA256SUMS)

# The set is complete and checked: put it in place. Between the two
# renames the label's directory is briefly absent, never partial.
if [ -d "$final" ]; then
    old="$VESSEL_DIST_DIR/.$label.old.$$"
    mv "$final" "$old"
    mv "$out" "$final"
    rm -rf "$old"
else
    mv "$out" "$final"
fi
out="$final"

if [ "$publish" = 1 ]; then
    : "${VESSEL_UPDATE_SECRET:?--publish signs the manifest: set VESSEL_UPDATE_SECRET}"
    : "${VESSEL_UPDATE_KEY:?--publish checks the manifest with VESSEL_UPDATE_KEY}"
    artifacts=()
    [ -f "$out/vessel-$label-windows-x64.exe" ] &&
        artifacts+=(--artifact "windows-x86_64:executable:$label/vessel-$label-windows-x64.exe")
    [ -f "$out/vessel-$label-linux-x64" ] &&
        artifacts+=(--artifact "linux-x86_64:executable:$label/vessel-$label-linux-x64")
    notes=()
    [ -f "$out/NOTES.txt" ] && notes=(--notes "$out/NOTES.txt")
    feed_tool publish --key "$VESSEL_UPDATE_SECRET" --feed "$VESSEL_DIST_DIR" --channel "$channel" \
        --label "$label" "${notes[@]}" "${artifacts[@]}"
    feed_tool check --feed "$VESSEL_DIST_DIR" --public "$VESSEL_UPDATE_KEY" --channel "$channel"
fi

ls -l "$out"
# Last: the sums, checked where the owner downloads from, and their
# signature with the release key's public half (the .pub beside the secret
# unless VESSEL_UPDATE_KEY names it).
(cd "$final" && sha256sum -c SHA256SUMS)
if [ -n "${VESSEL_UPDATE_SECRET:-}" ]; then
    feed_tool check-sums --public "${VESSEL_UPDATE_KEY:-${VESSEL_UPDATE_SECRET%.key}.pub}" "$final"
fi
