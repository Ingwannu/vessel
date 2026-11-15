#!/usr/bin/env bash
# Build every artifact of the native app for one label, on Linux.
#
#   spikes/release/build.sh LABEL [TARGET...] --compile-only DIR [--commit REV]
#   spikes/release/build.sh LABEL [TARGET...] --package-from DIR [--publish]
#   spikes/release/build.sh LABEL [TARGET...] [--publish]
#
# TARGETs: windows-x64, linux-x64 (both when none is named).
# A release is two steps, on two machines (ADR 0099):
#   --compile-only DIR  compiles, on a machine that holds no secret, and
#                       writes DIR (which must not exist): the executables
#                       with the feed address's slot unwritten, their
#                       symbols, the icons, the licences, the feed tool,
#                       compile.txt and SHA256SUMS. It refuses to run with
#                       the feed's address or the signing key in its
#                       environment. --commit REV: the source's commit, when
#                       the tree carries no git metadata (checked against
#                       HEAD when it does).
#   --package-from DIR  on the machine that holds them, and running no
#                       compiler: checks DIR against its sums and this
#                       checkout's commit, writes the feed's address into
#                       copies of the executables (tools/patch-feed.py),
#                       proves on the Linux build that the app reads the
#                       slot (tools/feed-slot-proof.sh), and makes the zip,
#                       the installer, the tarball, the gzip copies, the
#                       sums and their signature.
# Without either, both run here, one after the other (the same two steps).
# --publish also writes the channel's signed manifest (<dist>/<channel>.json),
# which tells every installed copy that LABEL is the newest build: only
# when the coordinator gives the word.
#
# The package step writes "$VESSEL_DIST_DIR/LABEL/" as one set: everything
# is made in a hidden directory beside it (keeping the other targets' files
# of the label, if any), checked with `sha256sum -c`, and only then renamed
# into place, so the directory the owner downloads from never holds a
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
#   <each file the updater downloads>.gz   the same file, gzip -9 -n: the
#                                       copy an app from 2026-11-15 on
#                                       downloads (the manifest's
#                                       `compressed`; older apps ignore it)
#   SHA256SUMS (and SHA256SUMS.minisig when a signing key is given)
# With VESSEL_PACKAGE_NODE=1 (ADR 0094), the command line and the node go
# in a bin folder beside the app in the zip, the installer and the tarball
# (bin\vessel.exe and bin\vesseld.exe on Windows, bin/vessel and
# bin/vesseld on Linux), and each also alone, as the update's companions:
#   vessel-LABEL-windows-x64-vessel.exe, vessel-LABEL-windows-x64-vesseld.exe
#   vessel-LABEL-linux-x64-vessel, vessel-LABEL-linux-x64-vesseld
#
# Environment (no host names or paths live in this file):
#   compile step
#   VESSEL_TOOLS_DIR       required for Windows: cargo-xwin (bin/), its
#                          cache (xwin-cache/), fxc (fxc/), wine's prefix
#                          (wineprefix/); resvg in bin/ for the icons
#   VESSEL_UPDATE_KEY_FILE the release key's public file, compiled in
#                          (update-key.pub beside this script when unset;
#                          `none` builds an app that never updates)
#   VESSEL_UPDATE_CHANNEL  the channel (preview), compiled in
#   VESSEL_APP_WORKSPACE, VESSEL_APP_PACKAGE, VESSEL_APP_BIN
#                          what to build (spikes/native, vessel-app,
#                          vessel-app); the release workspace's
#                          vessel-release-hello / vessel-hello tries the
#                          pipeline without the app
#   VESSEL_PACKAGE_NODE    1: ship the command line and the node with the
#                          app (ADR 0094); off until the app's local-node
#                          controller is in a build (the coordinator's
#                          word, 2026-11-15). The package step takes it
#                          from compile.txt.
#   JOBS                   cargo's -j (6: /data is one spinning disk)
#   package step
#   VESSEL_DIST_DIR        required: the dist root (the feed's base)
#   VESSEL_UPDATE_FEED_FILE  required when a release key is compiled in: a
#                          file whose first line is the feed's https://
#                          base. The address carries a token: it is read
#                          by tools/patch-feed.py only, never printed, and
#                          never reaches a compiler (the updater's build.rs
#                          refuses VESSEL_UPDATE_FEED, which once compiled
#                          it in)
#   VESSEL_UPDATE_SECRET   the release key's secret file: signs SHA256SUMS
#                          and, with --publish, the manifest
#   VESSEL_SYMBOLS_DIR     where each label's Windows .pdb and Linux
#                          .debug are kept (vessel-app-symbols beside the
#                          dist root)
#   VESSEL_FEED_TOOL       the feed tool to sign and publish with (the one
#                          the compile step built, DIR/tools, when unset)
#
# Why one script for every target: the owner gets a new build after every
# round of feedback, and each must carry the same label, the same update
# settings and the same checks on every system. Why two steps: ADR 0099.
set -euo pipefail

usage() {
    sed -n '2,8p' "$0" >&2
    exit 2
}

[ $# -ge 1 ] || usage
label="$1"
shift
[[ "$label" =~ ^[0-9]+(\.[0-9]+){0,7}$ ]] || { echo "label must be dot-separated numbers: $label" >&2; exit 2; }

targets=()
publish=0
compile_to=""
package_from=""
declared_commit=""
while [ $# -gt 0 ]; do
    case "$1" in
        windows-x64 | linux-x64) targets+=("$1") ;;
        --publish) publish=1 ;;
        --compile-only) [ $# -ge 2 ] || usage; compile_to="$2"; shift ;;
        --package-from) [ $# -ge 2 ] || usage; package_from="$2"; shift ;;
        --commit) [ $# -ge 2 ] || usage; declared_commit="$2"; shift ;;
        *) usage ;;
    esac
    shift
done
[ ${#targets[@]} -gt 0 ] || targets=(windows-x64 linux-x64)
[ -z "$compile_to" ] || [ -z "$package_from" ] || { echo "--compile-only and --package-from are two steps: one per run" >&2; exit 2; }
[ -z "$compile_to" ] || [ "$publish" = 0 ] || { echo "--compile-only publishes nothing" >&2; exit 2; }
[ -z "$declared_commit" ] || [ -n "$compile_to" ] || { echo "--commit goes with --compile-only" >&2; exit 2; }

here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
app_bin="${VESSEL_APP_BIN:-vessel-app}"

# The address is never compiled in any more (ADR 0099): refuse the
# variable that once carried it, whatever the step, before anything runs.
[ -z "${VESSEL_UPDATE_FEED:-}" ] ||
    { echo "VESSEL_UPDATE_FEED is no longer read (ADR 0099): give VESSEL_UPDATE_FEED_FILE to the package step" >&2; exit 2; }

stage="$(mktemp -d)"
partial=""
# The scratch directory and a partial set only: after its rename the set no
# longer exists under the partial name, and nothing published is removed.
cleanup() {
    rm -rf "$stage"
    [ -z "$partial" ] || rm -rf "$partial"
}
trap cleanup EXIT

# --------------------------------------------------------------------------
# Step 1: compile (no secret here)
# --------------------------------------------------------------------------

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
# Gathered by the compile step, which has the fonts it embedded.
licenses() {
    mkdir -p "$1"
    cp "$app_ws"/assets/fonts/*-OFL.txt "$1/"
    local text
    for text in "$repo"/docs/design/agent-marks/*.txt; do
        [ -e "$text" ] || continue
        cp "$text" "$1/"
    done
}

# The Linux build's debug information (the line tables of the native
# profile's `debug = "line-tables-only"`), split out of the executable, as
# the Windows .pdb is: what is shipped stays the size it was without them
# (its symbols stay, as before), and `--add-gnu-debuglink` names the kept
# file, so a debugger or `addr2line -e vessel-app.debug` gives a panic's
# frames their file and line. The package step keeps it per label in the
# symbols directory.
split_debug() {
    local built="$1" dir="$2"
    objcopy --only-keep-debug "$built" "$dir/$app_bin.debug"
    objcopy --strip-debug --add-gnu-debuglink="$dir/$app_bin.debug" "$built" "$dir/$app_bin"
    echo "linux-x64: $(du -h "$built" | cut -f1) built, $(du -h "$dir/$app_bin" | cut -f1) shipped"
}

# The command line and the node (ADR 0094), from the repository's own
# workspace, with the page the node serves built into it as the preview
# node is built (VESSEL_WEB_REQUIRE_BUNDLE: a node without the page fails
# the build instead of serving nothing). The release profile is that
# workspace's (fat LTO, symbols kept so a node's panic names its frames).
web_built=0
web_bundle() {
    [ "$web_built" = 1 ] && return
    (cd "$repo/web" && npm ci --silent && npm run build --silent)
    web_built=1
}

# The repository's toolchain and the app's are each a rust-toolchain.toml
# beside their workspace; the Windows target is per toolchain and per user,
# and adding it does nothing when it is there.
windows_target() {
    local channel
    channel="$(sed -n 's/^channel *= *"\(.*\)"/\1/p' "$1/rust-toolchain.toml")"
    rustup target add --toolchain "$channel" x86_64-pc-windows-msvc >/dev/null
}

compile_windows() {
    : "${VESSEL_TOOLS_DIR:?VESSEL_TOOLS_DIR names the Windows build tools}"
    local tools="$VESSEL_TOOLS_DIR" triple=x86_64-pc-windows-msvc dir="$1/windows-x64"
    mkdir -p "$dir"
    windows_target "$app_ws"
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
    (
        export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS="-C target-feature=+crt-static -C link-arg=/IGNORE:4099 -C link-arg=/SUBSYSTEM:WINDOWS -C link-arg=/ENTRY:mainCRTStartup -C link-arg=$res_dir/app.res"
        cd "$app_ws" && "$tools/bin/cargo-xwin" xwin build --release --locked --target "$triple" -p "$app_pkg" -j "$jobs"
    )
    local built="$target_dir/$triple/release"
    cp "$built/$app_bin.exe" "$dir/$app_bin.exe"
    # The .pdb (about 20 MB) names a panic's frames: vessel-app-log writes
    # them as `exe+0x<rva>`; `llvm-symbolizer --obj=<exe> --relative-address
    # 0x<rva>` with this .pdb beside the exe. Taken now because the next
    # build overwrites the one in the target directory.
    cp "$built/${app_bin//-/_}.pdb" "$dir/"
    if [ "$package_node" = 1 ]; then
        # Console programs, so neither the app's GUI subsystem nor its
        # resources (the flags are the app's alone; here only the static CRT).
        web_bundle
        windows_target "$repo"
        (
            cd "$repo"
            export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS="-C target-feature=+crt-static -C link-arg=/IGNORE:4099"
            VESSEL_WEB_REQUIRE_BUNDLE=1 "$tools/bin/cargo-xwin" xwin build --release --locked --target "$triple" \
                -p vessel-cli -p vessel-node --bins -j "$jobs"
        )
        mkdir -p "$dir/bin"
        local program
        for program in vessel vesseld; do
            cp "$root_target/$triple/release/$program.exe" "$dir/bin/$program.exe"
        done
    fi
    echo "windows-x64: $(du -h "$dir/$app_bin.exe" | cut -f1) executable"
}

compile_linux() {
    local dir="$1/linux-x64"
    mkdir -p "$dir"
    (cd "$app_ws" && cargo build --release --locked -p "$app_pkg" -j "$jobs")
    split_debug "$target_dir/release/$app_bin" "$dir"
    if [ "$package_node" = 1 ]; then
        web_bundle
        (cd "$repo" && VESSEL_WEB_REQUIRE_BUNDLE=1 cargo build --release --locked -p vessel-cli -p vessel-node --bins -j "$jobs")
        mkdir -p "$dir/bin"
        local program
        for program in vessel vesseld; do
            install -m 0755 "$root_target/release/$program" "$dir/bin/$program"
        done
    fi
}

# DIR as the package step reads it; written beside DIR and renamed into
# place when complete, so DIR exists only whole.
compile_step() {
    local final="$1"
    [ ! -e "$final" ] || { echo "$final exists; the compile step makes a new directory" >&2; exit 2; }
    # The secrets never reach this step: refused, not merely unused.
    local name
    for name in VESSEL_UPDATE_FEED_FILE VESSEL_UPDATE_SECRET; do
        [ -z "${!name:-}" ] || { echo "$name is set; the compile step holds no secret (ADR 0099)" >&2; exit 2; }
    done
    mkdir -p "$(dirname "$final")"
    partial="$(dirname "$final")/.$(basename "$final").partial.$$"
    local out="$partial"
    mkdir "$out"

    app_ws="${VESSEL_APP_WORKSPACE:-$repo/spikes/native}"
    app_pkg="${VESSEL_APP_PACKAGE:-vessel-app}"
    jobs="${JOBS:-6}"
    package_node="${VESSEL_PACKAGE_NODE:-0}"
    [[ "$package_node" =~ ^[01]$ ]] || { echo "VESSEL_PACKAGE_NODE is 0 or 1" >&2; exit 2; }
    local channel="${VESSEL_UPDATE_CHANNEL:-preview}"

    # The source: HEAD when the tree is a checkout (and clean), else what
    # --commit declares (a tree exported without its git metadata).
    local commit=""
    if git -C "$repo" rev-parse --verify -q HEAD >/dev/null 2>&1; then
        commit="$(git -C "$repo" rev-parse HEAD)"
        [ -z "$declared_commit" ] || [ "$(git -C "$repo" rev-parse "$declared_commit^{commit}")" = "$commit" ] ||
            { echo "--commit $declared_commit is not this checkout's HEAD" >&2; exit 2; }
        git -C "$repo" diff --quiet HEAD -- || { echo "the checkout has changes to tracked files; a label builds a commit" >&2; exit 2; }
    else
        [ -n "$declared_commit" ] || { echo "no git metadata here: name the source with --commit REV" >&2; exit 2; }
        [[ "$declared_commit" =~ ^[0-9a-f]{40}$ ]] || { echo "--commit takes the full 40-digit commit" >&2; exit 2; }
        commit="$declared_commit"
    fi

    # The release key's public half, compiled in (it is public; ADR 0099
    # says why it is committed beside this script).
    local key_file="${VESSEL_UPDATE_KEY_FILE:-$here/update-key.pub}" key=""
    if [ "$key_file" != none ]; then
        key="$(tail -n 1 "$key_file" | tr -d '[:space:]')"
        [ -n "$key" ] || { echo "$key_file holds no key" >&2; exit 2; }
        export VESSEL_UPDATE_KEY="$key" VESSEL_UPDATE_CHANNEL="$channel"
        echo "updates: channel $channel; the feed's address is written in by the package step"
    else
        unset VESSEL_UPDATE_KEY VESSEL_UPDATE_CHANNEL
        echo "updates: none (no release key compiled in)"
    fi
    export VESSEL_BUILD_LABEL="$label"

    # The app embeds its faces; they are fetched, not committed.
    # Checked by the newest face the kit embeds (Light, 300, added 2026-11-13),
    # so a tree fetched before it fetches again.
    if [ -x "$app_ws/scripts/fonts.sh" ] && [ ! -f "$app_ws/assets/fonts/Pretendard-Light.otf" ]; then
        (cd "$app_ws" && scripts/fonts.sh "$stage/fonts")
    fi

    target_dir="$(cd "$app_ws" && cargo metadata --format-version 1 --no-deps |
        python3 -I -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')"
    root_target=""
    if [ "$package_node" = 1 ]; then
        root_target="$(cd "$repo" && cargo metadata --format-version 1 --no-deps |
            python3 -I -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')"
    fi
    # What the README says about updates must be true of this build: the
    # key compiled in, and the app linking the updater that reads it.
    local links_updater=0
    (cd "$app_ws" && cargo metadata --format-version 1 --locked 2>/dev/null) |
        grep -q '"name":"vessel-app-updater"' && links_updater=1

    # Every icon, from the owner's mark (docs/design/brand/, whose README
    # says how it is used); nothing generated is committed. resvg
    # rasterises (tools/make-icons.py says why not ImageMagick's). The
    # Windows link takes the .ico, the package step the rest.
    icons="$out/icons"
    PATH="${VESSEL_TOOLS_DIR:+$VESSEL_TOOLS_DIR/bin:}$PATH" \
        python3 -I "$here/tools/make-icons.py" "$repo/docs/design/brand/vessel-mark.svg" "$icons"
    licenses "$out/licenses"

    local target
    for target in "${targets[@]}"; do
        case "$target" in
            windows-x64) compile_windows "$out" ;;
            linux-x64) compile_linux "$out" ;;
        esac
    done

    # The slot must be in each app exactly once before anything travels.
    local apps=()
    [ -f "$out/windows-x64/$app_bin.exe" ] && apps+=("$out/windows-x64/$app_bin.exe")
    [ -f "$out/linux-x64/$app_bin" ] && apps+=("$out/linux-x64/$app_bin")
    if [ -n "$key" ] && [ "$links_updater" = 1 ]; then
        python3 -I "$here/tools/patch-feed.py" check "${apps[@]}"
    fi

    # The feed tool, from this commit, for the package step to sign and
    # publish with: that machine runs no compiler.
    (cd "$here" && cargo build --release --locked -p vessel-app-feed -j "$jobs")
    local feed_target
    feed_target="$(cd "$here" && cargo metadata --format-version 1 --no-deps |
        python3 -I -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')"
    mkdir -p "$out/tools"
    install -m 0755 "$feed_target/release/vessel-app-feed" "$out/tools/vessel-app-feed"

    {
        echo "format=1"
        echo "label=$label"
        echo "targets=${targets[*]}"
        echo "commit=$commit"
        echo "package_node=$package_node"
        echo "channel=$channel"
        echo "update_key=$key"
        echo "links_updater=$links_updater"
        echo "app_bin=$app_bin"
        echo "feed_slot=512"
        echo "rustc=$(cd "$app_ws" && rustc -V)"
        echo "compiled_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    } >"$out/compile.txt"
    (cd "$out" && find . -type f ! -name SHA256SUMS -printf '%P\n' | LC_ALL=C sort | xargs -d '\n' sha256sum >SHA256SUMS)
    mv "$out" "$final"
    partial=""
    echo "compiled $label (${targets[*]}) from $commit into $final"
}

# --------------------------------------------------------------------------
# Step 2: package (the secrets live here; no compiler runs)
# --------------------------------------------------------------------------

compiled() {
    sed -n "s/^$1=//p" "$from/compile.txt"
}

# The compile step's output is what this checkout says it is.
check_compiled() {
    [ -f "$from/compile.txt" ] && [ -f "$from/SHA256SUMS" ] || { echo "$from is not a compile step's output" >&2; exit 2; }
    (cd "$from" && sha256sum --quiet --strict -c SHA256SUMS) || { echo "$from does not match its SHA256SUMS" >&2; exit 1; }
    [ "$(compiled format)" = 1 ] || { echo "$from: compile.txt is not format 1" >&2; exit 1; }
    [ "$(compiled label)" = "$label" ] || { echo "$from was compiled as $(compiled label), not $label" >&2; exit 1; }
    local target
    for target in "${targets[@]}"; do
        [[ " $(compiled targets) " == *" $target "* ]] || { echo "$from has no $target build" >&2; exit 1; }
    done
    local head
    head="$(git -C "$repo" rev-parse HEAD)"
    [ "$(compiled commit)" = "$head" ] || { echo "$from was compiled from $(compiled commit), this checkout is $head" >&2; exit 1; }
    # The files the package takes from this checkout are the commit's.
    git -C "$repo" diff --quiet HEAD -- LICENSE.md spikes/release ||
        { echo "LICENSE.md or spikes/release differ from $head here" >&2; exit 1; }
    [ -z "${VESSEL_PACKAGE_NODE:-}" ] || [ "$VESSEL_PACKAGE_NODE" = "$(compiled package_node)" ] ||
        { echo "VESSEL_PACKAGE_NODE=$VESSEL_PACKAGE_NODE, but $from was compiled with $(compiled package_node)" >&2; exit 1; }
    [ "$(compiled app_bin)" = "$app_bin" ] || { echo "$from built $(compiled app_bin), not $app_bin" >&2; exit 1; }
    [ "$(compiled feed_slot)" = 512 ] || { echo "$from has a feed slot of $(compiled feed_slot) bytes" >&2; exit 1; }
}

# The compressed copy of a file the updater downloads (updater/src/stage.rs
# says why gzip, and why a copy rather than instead): -n leaves out the
# name and time, so one file always gives the same bytes.
gzip_copy() {
    gzip -9 -n -c "$1" >"$1.gz"
}

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

# Every DLL `exe` imports must be one Windows ships (the list above).
check_dlls() {
    local exe="$1" dll unknown=0 headers
    # Read once into a variable: `grep -q` on a pipe would close it early
    # and, under pipefail, fail the check of a good executable.
    headers="$(x86_64-w64-mingw32-objdump -p "$exe")"
    for dll in $(sed -n 's/^\s*DLL Name: //p' <<<"$headers" | tr 'A-Z' 'a-z' | sort -u); do
        if ! grep -qw -- "$dll" <<<"$windows_dlls"; then
            echo "${exe##*/} imports $dll, which is not on the list of DLLs Windows ships" >&2
            unknown=1
        fi
    done
    [ "$unknown" = 0 ] || exit 1
}

check_imports() {
    local exe="$1" headers
    check_dlls "$exe"
    headers="$(x86_64-w64-mingw32-objdump -p "$exe")"
    # The icon and the version block (compile_windows links them) beside
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

# The app as it ships: the compile step's executable with the feed's
# address written into a copy (the compile step's file is never changed),
# or the executable as compiled when this build does not update itself.
app_copy() {
    local compiled_exe="$1" shipped="$2"
    if [ "$writes_feed" = 1 ]; then
        python3 -I "$here/tools/patch-feed.py" write --address-file "$VESSEL_UPDATE_FEED_FILE" "$compiled_exe" "$shipped"
    else
        install -m "$(stat -c %a "$compiled_exe")" "$compiled_exe" "$shipped"
    fi
}

# The modes the published sets have always had (dirs 0775, files 0664,
# programs 0775), set here because the compile step's files come back with
# whatever mode the transfer gave them (the remote gate's are 0500).
stage_modes() {
    local dir="$1"
    shift
    find "$dir" -type d -exec chmod 0775 {} +
    find "$dir" -type f -exec chmod 0664 {} +
    [ $# -eq 0 ] || chmod 0775 "$@"
}

# The symbols of a label, never in the dist folder: kept per label in the
# symbols directory with the shipped executable's SHA-256 beside them.
symbols_dir() {
    echo "${VESSEL_SYMBOLS_DIR:-$(dirname "$VESSEL_DIST_DIR")/vessel-app-symbols}/$label"
}

package_windows() {
    local built="$from/windows-x64" name="vessel-$label-windows-x64"
    rm -f "$out/$name"*
    local dir="$stage/windows/Vessel"
    mkdir -p "$dir"
    app_copy "$built/$app_bin.exe" "$stage/windows/app.exe"
    local exe="$stage/windows/app.exe"
    check_imports "$exe"
    cp "$exe" "$dir/Vessel.exe"
    local companions=()
    if [ "$package_node" = 1 ]; then
        # In bin\, so the command line keeps its name beside Vessel.exe
        # (Windows' names ignore case) and only bin\ goes on PATH.
        local program
        mkdir -p "$dir/bin"
        for program in vessel vesseld; do
            check_dlls "$built/bin/$program.exe"
            cp "$built/bin/$program.exe" "$dir/bin/$program.exe"
            install -m 0775 "$built/bin/$program.exe" "$out/$name-$program.exe"
            gzip_copy "$out/$name-$program.exe"
        done
        companions=(-DCOMPANIONS=1)
    fi
    cp "$repo/LICENSE.md" "$dir/LICENSE.md"
    cp -r "$from/licenses" "$dir/licenses"
    readme "$here/windows/README.txt" | sed 's/$/\r/' >"$dir/README.txt"
    local programs=("$dir/Vessel.exe")
    [ "$package_node" = 0 ] || programs+=("$dir/bin/vessel.exe" "$dir/bin/vesseld.exe")
    stage_modes "$dir" "${programs[@]}"
    (cd "$stage/windows" && TZ=UTC zip -q -X -r "$out/$name.zip" Vessel)
    install -m 0775 "$exe" "$out/$name.exe"
    gzip_copy "$out/$name.exe"
    makensis -V2 -DLABEL="$label" -DSTAGE="$dir" -DICONS="$from/icons" "${companions[@]}" \
        -DOUTFILE="$out/$name-setup.exe" "$here/windows/installer.nsi"
    shipped_apps+=("$out/$name.exe")
    echo "windows-x64: $(du -h "$exe" | cut -f1) executable"
    local symbols
    symbols="$(symbols_dir)"
    mkdir -p "$symbols"
    local pdb="${app_bin//-/_}.pdb"
    install -m 0664 "$built/$pdb" "$symbols/$pdb"
    (cd "$symbols" && sha256sum "$pdb" >SHA256SUMS)
    printf '%s  %s\n' "$(sha256sum <"$exe" | cut -d' ' -f1)" "$name.exe" >>"$symbols/SHA256SUMS"
    echo "windows-x64: $pdb kept in the symbols directory under $label"
}

package_linux() {
    local built="$from/linux-x64" name="vessel-$label-linux-x64"
    rm -f "$out/$name" "$out/$name.gz" "$out/$name.tar.gz" "$out/$name-vessel" "$out/$name-vessel.gz" \
        "$out/$name-vesseld" "$out/$name-vesseld.gz"
    local dir="$stage/linux/$name"
    mkdir -p "$dir"
    local bin="$stage/linux/$app_bin"
    app_copy "$built/$app_bin" "$bin"
    chmod 0755 "$bin"
    # Not `vessel`: that is the command line's name.
    cp "$bin" "$dir/vessel-app"
    if [ "$package_node" = 1 ]; then
        local program
        mkdir -p "$dir/bin"
        for program in vessel vesseld; do
            install -m 0755 "$built/bin/$program" "$dir/bin/$program"
            install -m 0755 "$built/bin/$program" "$out/$name-$program"
            gzip_copy "$out/$name-$program"
        done
    fi
    cp "$repo/LICENSE.md" "$dir/LICENSE.md"
    cp -r "$from/licenses" "$dir/licenses"
    readme "$here/linux/README.txt" >"$dir/README.txt"
    # The menu entry and its icons (the freedesktop sizes), and the script
    # that installs them for the user who unpacked the tarball.
    local size
    for size in 16 24 32 48 64 128 256 512; do
        mkdir -p "$dir/icons/hicolor/${size}x${size}/apps"
        cp "$from/icons/tile-$size.png" "$dir/icons/hicolor/${size}x${size}/apps/vessel-app.png"
    done
    cp "$here/linux/vessel-app.desktop.in" "$here/linux/install-desktop-entry.sh" "$dir/"
    local programs=("$dir/vessel-app" "$dir/install-desktop-entry.sh")
    [ "$package_node" = 0 ] || programs+=("$dir/bin/vessel" "$dir/bin/vesseld")
    stage_modes "$dir" "${programs[@]}"
    tar --sort=name --owner=0 --group=0 --numeric-owner --mtime='@0' -C "$stage/linux" -czf "$out/$name.tar.gz" "$name"
    cp "$bin" "$out/$name"
    chmod 0755 "$out/$name"
    gzip_copy "$out/$name"
    shipped_apps+=("$out/$name")
    echo "linux-x64: $(du -h "$bin" | cut -f1) executable"
    local symbols
    symbols="$(symbols_dir)"
    mkdir -p "$symbols"
    install -m 0775 "$built/$app_bin.debug" "$symbols/$app_bin.debug"
    (cd "$symbols" && sha256sum "$app_bin.debug" >SHA256SUMS-linux)
    printf '%s  %s\n' "$(sha256sum <"$bin" | cut -d' ' -f1)" "$name" >>"$symbols/SHA256SUMS-linux"
    echo "linux-x64: $app_bin.debug kept in the symbols directory under $label"
}

feed_tool() {
    "$feed_tool_bin" "$@"
}

package_step() {
    from="$(cd "$1" && pwd)"
    check_compiled
    : "${VESSEL_DIST_DIR:?VESSEL_DIST_DIR names the dist root}"
    package_node="$(compiled package_node)"
    local channel key links_updater
    channel="$(compiled channel)"
    key="$(compiled update_key)"
    links_updater="$(compiled links_updater)"
    feed_tool_bin="${VESSEL_FEED_TOOL:-$from/tools/vessel-app-feed}"
    [ -x "$feed_tool_bin" ] || { echo "no feed tool at $feed_tool_bin" >&2; exit 2; }

    # The address goes in when the app can use it: a key compiled in and
    # the updater linked. Then it is required, and the Linux build proves
    # the app reads it (below); otherwise it is refused, so no build carries
    # an address it never reads.
    writes_feed=0
    if [ -n "$key" ] && [ "$links_updater" = 1 ]; then
        : "${VESSEL_UPDATE_FEED_FILE:?this build carries a release key: give VESSEL_UPDATE_FEED_FILE}"
        [[ " ${targets[*]} " == *" linux-x64 "* ]] ||
            { echo "the feed slot is proven on the Linux build: package linux-x64 too" >&2; exit 2; }
        writes_feed=1
        echo "updates: channel $channel, from the feed's address written in now"
    else
        [ -z "${VESSEL_UPDATE_FEED_FILE:-}" ] ||
            { echo "this build reads no feed address (no key compiled in, or no updater linked); unset VESSEL_UPDATE_FEED_FILE" >&2; exit 2; }
        echo "updates: none in this build"
    fi
    # The signature must be one the app accepts: the sums are signed and
    # then checked with the key compiled in, before anything is published.
    if [ -n "${VESSEL_UPDATE_SECRET:-}" ] && [ -z "$key" ]; then
        echo "a signing key is given but no release key is compiled in" >&2
        exit 2
    fi
    if [ "$publish" = 1 ]; then
        : "${VESSEL_UPDATE_SECRET:?--publish signs the manifest: set VESSEL_UPDATE_SECRET}"
        [ "$writes_feed" = 1 ] || { echo "--publish needs a build that updates itself" >&2; exit 2; }
    fi

    if [ "$writes_feed" = 1 ]; then
        updates_ko="이 빌드부터 스스로 업데이트합니다. 시작 직후와 한 시간마다 새 빌드를 확인해 뒤에서 받아 두고, 다음에 Vessel을 열면 새 빌드로 시작합니다(받아 두면 화면 아래에 알려 줍니다). 0.0.0.0.2 이하를 쓰던 경우 이번 한 번만 이 빌드를 직접 설치하세요. 자동 확인을 끄려면 노드 목록과 같은 폴더의 app-updates.json 에서 automatic 값을 false로 바꿉니다(한 번만 끄려면 VESSEL_APP_NO_UPDATE=1)."
        updates_en="From this build on, Vessel updates itself: it checks shortly after start and hourly, downloads a new build in the background, and opens it the next time Vessel starts (the window's foot says when one is ready). Coming from 0.0.0.0.2 or earlier, install this build by hand once. To turn the automatic checks off, set automatic to false in app-updates.json beside the node book (VESSEL_APP_NO_UPDATE=1 turns updates off for one run)."
    else
        updates_ko="이 빌드는 스스로 업데이트하지 않습니다. 새 빌드는 새로 받아 같은 방법으로 설치하면 덮어씁니다."
        updates_en="This build does not update itself. Install a new build the same way; it replaces this one."
    fi
    if [ "$package_node" = 1 ]; then
        updates_ko="$updates_ko 명령줄(vessel)과 노드(vesseld)는 앱 옆 bin 폴더에 들어 있고 앱과 함께 바뀝니다. 명령줄을 쓰려면 그 bin 폴더를 PATH에 넣으세요."
        updates_en="$updates_en The command line (vessel) and the node (vesseld) are in the bin folder beside the app and change with it; put that bin folder on PATH to use the command line."
    fi

    # Before the real address goes anywhere: the compiled Linux app, given
    # a test feed on this machine, asks it for the channel's manifest. This
    # is what shows the optimiser did not fold the slot's marker into the
    # code (updater/src/config.rs, FEED_SLOT_LEN).
    if [ "$writes_feed" = 1 ]; then
        "$here/tools/feed-slot-proof.sh" test "$from/linux-x64/$app_bin" "$channel"
    fi

    local final="$VESSEL_DIST_DIR/$label"
    partial="$VESSEL_DIST_DIR/.$label.partial.$$"
    out="$partial"
    mkdir -p "$out"
    if [ -d "$final" ]; then
        # Hard links: no copy of tens of MiB; every file this run writes is
        # removed first (fresh) so the published one is never written through.
        cp -al "$final/." "$out/"
    fi
    rm -f "$out/SHA256SUMS" "$out/SHA256SUMS.minisig"

    shipped_apps=()
    local target
    for target in "${targets[@]}"; do
        case "$target" in
            windows-x64) package_windows ;;
            linux-x64) package_linux ;;
        esac
    done

    if [ "$writes_feed" = 1 ]; then
        # Every app that ships holds the address once, in the slot, and no
        # marker; then the Linux one, as it ships, reads the real feed and
        # checks its signed manifest with the key compiled in.
        python3 -I "$here/tools/patch-feed.py" verify --address-file "$VESSEL_UPDATE_FEED_FILE" "${shipped_apps[@]}"
        "$here/tools/feed-slot-proof.sh" real "$out/vessel-$label-linux-x64" "$channel" "$VESSEL_UPDATE_FEED_FILE"
    fi

    if [ -n "${VESSEL_UPDATE_SECRET:-}" ]; then
        # The label names the set in the signature's comment: the files sit in
        # a hidden staging directory now, whose name must not be signed.
        feed_tool sums --key "$VESSEL_UPDATE_SECRET" --label "$label" "$out"
        # Signed by a key the app accepts, or nothing is published.
        feed_tool check-sums --public "$key" "$out"
    else
        (cd "$out" && find . -maxdepth 1 -type f ! -name 'SHA256SUMS*' ! -name '.*' -printf '%f\n' | sort |
            xargs sha256sum >SHA256SUMS)
        echo "SHA256SUMS unsigned (VESSEL_UPDATE_SECRET unset)"
    fi
    (cd "$out" && sha256sum --quiet -c SHA256SUMS)

    # The set is complete and checked: put it in place. Between the two
    # renames the label's directory is briefly absent, never partial.
    if [ -d "$final" ]; then
        local old="$VESSEL_DIST_DIR/.$label.old.$$"
        mv "$final" "$old"
        mv "$out" "$final"
        rm -rf "$old"
    else
        mv "$out" "$final"
    fi
    partial=""
    out="$final"

    if [ "$publish" = 1 ]; then
        local artifacts=()
        [ -f "$out/vessel-$label-windows-x64.exe" ] &&
            artifacts+=(--artifact "windows-x86_64:executable:$label/vessel-$label-windows-x64.exe")
        [ -f "$out/vessel-$label-linux-x64" ] &&
            artifacts+=(--artifact "linux-x86_64:executable:$label/vessel-$label-linux-x64")
        # ADR 0094: the companions present in the set travel with their app.
        local companion file role
        for companion in windows-x64-vessel.exe:windows-x86_64:cli windows-x64-vesseld.exe:windows-x86_64:node \
            linux-x64-vessel:linux-x86_64:cli linux-x64-vesseld:linux-x86_64:node; do
            IFS=: read -r file target role <<<"$companion"
            [ -f "$out/vessel-$label-$file" ] &&
                artifacts+=(--companion "$target:executable:$role:$label/vessel-$label-$file")
        done
        local notes=()
        [ -f "$out/NOTES.txt" ] && notes=(--notes "$out/NOTES.txt")
        # Every file the manifest names has its gzip copy (gzip_copy above).
        feed_tool publish --key "$VESSEL_UPDATE_SECRET" --feed "$VESSEL_DIST_DIR" --channel "$channel" --compressed gzip \
            --label "$label" "${notes[@]}" "${artifacts[@]}"
        feed_tool check --feed "$VESSEL_DIST_DIR" --public "$key" --channel "$channel"
    fi

    ls -l "$out"
    # Last: the sums, checked where the owner downloads from, and their
    # signature with the key compiled into the app.
    (cd "$final" && sha256sum -c SHA256SUMS)
    if [ -n "${VESSEL_UPDATE_SECRET:-}" ]; then
        feed_tool check-sums --public "$key" "$final"
    fi
}

if [ -n "$compile_to" ]; then
    compile_step "$compile_to"
elif [ -n "$package_from" ]; then
    package_step "$package_from"
else
    # Both steps here: the compile step's secrets check would refuse what
    # the package step needs, so they are set aside for it and restored.
    feed_file="${VESSEL_UPDATE_FEED_FILE:-}" secret="${VESSEL_UPDATE_SECRET:-}"
    unset VESSEL_UPDATE_FEED_FILE VESSEL_UPDATE_SECRET
    (compile_step "$stage/compiled")
    [ -z "$feed_file" ] || export VESSEL_UPDATE_FEED_FILE="$feed_file"
    [ -z "$secret" ] || export VESSEL_UPDATE_SECRET="$secret"
    package_step "$stage/compiled"
fi
