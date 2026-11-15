# Milestone 12, track R: releases, installers and updates

Status: started 2026-11-13. Branch `backend/m12-release`, workspace
`spikes/release/` (its own, outside Vessel's, as every milestone 12
track's is). Answers questions 4 (Windows and macOS) and 8 (updates) of
`docs/plans/backend-m12.md`, and builds what the owner installs.

Read first: ADR 0035 (the native app), ADR 0008 (how `vesseld` releases
are signed and installed), ADR 0014 (what may be linked), and
`spikes/release/patches/README.md` (the patched GPUI crates).

## What this track owns

```
spikes/release/
  build.sh               every artifact of one label, on Linux, in two steps
                         (--compile-only on the build VM, --package-from where
                         the secrets are; ADR 0099)
  update-key.pub         the release key's public half, compiled into the app
  hello/                 the smallest GPUI window (a target's first proof)
  updater/               vessel-app-updater: the app's updater, no GPUI
  feed/                  vessel-app-feed: release key, signed feed, sums
  patches/               patched crates (README there): gpui-pre-windows and
                         gpui-pre-linux 0.3.10, and from 0.0.0.0.15
                         gpui-base 0.8.1 (a steady caret while motion is held)
  tools/fxc-wine         Microsoft's fxc.exe under wine, for GPUI's shaders
  tools/fetch-fxc.py     fetches fxc.exe and d3dcompiler_47.dll from the SDK
  tools/patch-feed.py    writes the feed's address into a built app (ADR 0099)
  tools/feed-slot-proof.sh  runs a built Linux app to show it reads that slot
  windows/installer.nsi  the per-user installer
  windows/README.txt, linux/README.txt   what each package says
  ci/app-release.yml     the hosted-runner workflow (not under .github: not run)
```

The app itself is the desktop track's (`spikes/native/`, merged from
`backend/m12-spike`). Its manifest's `[patch.crates-io]` section points
each patched crate at its copy here: `gpui-pre-windows` (compiled only by
a Windows build), `gpui-pre-linux` (only by a Linux build) and, from
0.0.0.0.15, `gpui-base` (both). The release workspace uses none of
gpui-base, so its own patch table is unchanged.

## Running it

Once per machine (Ubuntu 24.04): `sudo apt-get install wine wine64
mingw-w64 nsis msitools` (installed here on 2026-11-13), then
`cargo install --root "$VESSEL_TOOLS_DIR" cargo-xwin` (0.23.1 here) and
`python3 -I spikes/release/tools/fetch-fxc.py "$VESSEL_TOOLS_DIR/fxc"`.
cargo-xwin fetches Microsoft's CRT and SDK libraries into
`$VESSEL_TOOLS_DIR/xwin-cache` on its first build; `fetch-fxc.py`
fetches the HLSL compiler. Both are Microsoft's files under Microsoft's
terms (see "Left for the owner").

A build, since 2026-11-15 in two steps on two machines (ADR 0099, and
"Labels compiled without secrets" below):

```bash
# the build VM (no secret; the remote gate's release compile mode runs this)
VESSEL_TOOLS_DIR=<tools> spikes/release/build.sh 0.0.0.0.21 windows-x64 linux-x64 --compile-only DIR
# the machine with the secrets, in a checkout of the same commit; no compiler
VESSEL_DIST_DIR=<dist root> VESSEL_UPDATE_FEED_FILE=<address file> \
VESSEL_UPDATE_SECRET=<secret key file> \
spikes/release/build.sh 0.0.0.0.21 windows-x64 linux-x64 --package-from DIR [--publish]
```

The compile step compiles in the label and the release key's public half
(`update-key.pub`); the package step writes the feed's address into the
built app and signs. `--publish` then writes the channel's signed manifest
(the moment every installed copy sees the new build). Labels up to
0.0.0.0.20 were built in one step with the address compiled in
(`VESSEL_UPDATE_FEED`, now refused).

## Step 1: a GPUI window built for Windows on Linux

What was run, in order, and what it printed (2026-11-13; GPUI is the
desktop track's pin, `gpui-pre` 0.3.10 = zed@08cbb88; toolchain 1.98.1).

1. **cargo-xwin, msvc**: `cargo xwin build --release --target
   x86_64-pc-windows-msvc -p vessel-release-hello`. It downloaded the
   MSVC CRT and the Windows SDK libraries (1.1 GB cache) and compiled
   everything until `gpui-pre-windows`:

   ```
   error: couldn't read `.../gpui-pre-windows-.../out/shaders_bytes.rs`: No such file or directory (os error 2)
       --> .../gpui-pre-windows-0.3.10/src/directx_renderer.rs:1869:5
   ```

   Upstream's build script compiles the shaders only under
   `#[cfg(target_os = "windows")]`, which in a build script is the host.
   mingw was not needed: msvc linked once the shaders were solved.
2. **The shaders.** DXC (Microsoft's only Linux compiler) emits DXIL for
   Direct3D 12; the Windows renderer is Direct3D 11 and loads DXBC
   (shader model 4.1). Microsoft's `fxc.exe` and `d3dcompiler_47.dll`
   were taken from the Windows SDK 10.0.26100 package
   (`tools/fetch-fxc.py`) and run under wine 9.0:

   ```
   compilation header save succeeded; see Z:\...\quad_vs.h
   ```

   A copy of `gpui-pre-windows` (`spikes/release/patches/`) asks Cargo
   for the target and takes the compiler from `GPUI_FXC_PATH`, which
   points at `tools/fxc-wine`. The release build then finished:
   `Finished release profile [optimized] target(s) in 5m 03s`, an 8.2 MB
   `vessel-hello.exe`.
3. **Static libraries for msvc only**: none met; lld-link linked the
   whole tree with the xwin libraries.
4. **Resources and the manifest**: embedded, by `embed-resource` with
   LLVM's `llvm-rc`. `llvm-readobj --coff-resources` shows one
   `RT_MANIFEST`: `asInvoker`, Windows 10 `supportedOS`, DPI awareness
   `true/pm` and `PerMonitorV2`, `SegmentHeap`, Common Controls 6. No
   `activeCodePage` UTF-8: GPUI calls the wide (`W`) Windows APIs, so the
   code page does not reach its text; noted, not changed.
5. **The renderer**: Direct3D 11 with DirectComposition
   (`gpui-pre-windows`); there is no wgpu on Windows in this snapshot.
   The Windows platform is a target dependency of `gpui-pre-platform`
   with no feature to turn on (unlike Linux's `x11`/`wayland`): the
   window opened under wine, so the exe does not come up windowless.
6. **Imports.** First build: `VCRUNTIME140.dll` and the
   `api-ms-win-crt-*` sets, which a clean Windows does not always have
   (the Visual C++ redistributable). Rebuilt with `-C
   target-feature=+crt-static`: only DLLs Windows 10 and 11 ship remain:

   ```
   KERNEL32 advapi32 api-ms-win-core-synch-l1-2-0 api-ms-win-core-winrt-error-l1-1-0
   api-ms-win-core-winrt-l1-1-0 api-ms-win-shcore-scaling-l1-1-1 bcryptprimitives combase
   comctl32 d3d11 dcomp dwmapi dwrite dxgi gdi32 icuuc imm32 ntdll ole32 oleaut32 shell32
   uiautomationcore user32 userenv winmm
   ```

   `icuuc.dll` (one function, `u_strlen`, for jump lists) is in
   System32 since Windows 10 1703. `build.sh` fails a build that
   imports anything off that list, and one whose PE subsystem is not
   "Windows GUI" (a console would open behind the window).
7. **Under wine**, in order of what stopped it (a virtual X display):
   - wine 9.0 has no `icuuc.dll`: a stand-in with `u_strlen` (built with
     mingw, used only for these runs, never shipped).
   - `Creating DirectX resources: Not implemented. (0x80004001)`:
     DirectComposition's swap chain; `GPUI_DISABLE_DIRECT_COMPOSITION=1`
     (an upstream switch) takes the plain swap chain.
   - `initializing Direct Manipulation: Not implemented. (0x80004001)`:
     wine stubs the touchpad API; the patched copy opens the window
     without it (patches/README.md, change 2).
   - Then the Direct3D 11 device, the swap chain and the build's shaders
     were created under wine's wined3d (OpenGL, llvmpipe), and the first
     text panicked:

     ```
     failed to resolve font '.SystemUIFont' or any of the fallbacks: .ZedMono, .ZedSans, Helvetica, Segoe UI, ...
     ```

     wine 9.0's DirectWrite does not implement
     `IDWriteFontCollection1::GetFontSet` (`fixme:dwrite:dwritefontcollection1_GetFontSet`),
     and WineHQ's 11.0 (unpacked under the tools directory, not
     installed) has it but stubs `IDWriteFontSet::GetMatchingFonts`
     (`fixme:dwrite:dwritefontset_GetMatchingFonts`). GPUI finds every
     font, its own embedded ones included, through that call. So no text
     can be drawn under wine, and this is wine's limit, not the build's.

   **Not verified on a real Windows**: that a window with text draws,
   Hangul input, DPI scaling, and the installer's run. The CI file runs
   the exe on a hosted Windows runner for exactly this; until it runs or
   the owner starts the build, the Windows build has been seen to start,
   open its window and create its renderer, and no more.

## Step 2: the app for Windows

Label `0.0.0.0.1`, built by `build.sh` from this branch at `85cc4cb`
(the merge of the desktop track's `99f50d8`; the app as it was then:
machines and sessions from a real node, the board, a session's
conversation with permissions answered and Markdown drawn, pairing). No
update settings compiled in: the app does not link the updater yet. The
label's directory also holds the desktop track's Linux tarball
(`vessel-app-0.0.0.0.1-linux-x86_64.tar.gz`, from its own script), which
`build.sh` kept and summed.

- **The build**: 11 min 49 s cold, 10 min 9 s after the merge (`-j 6`,
  fat LTO); a 33 MB
  `vessel-app.exe`, GUI subsystem, static CRT. Its imports are the
  hello window's plus `bcrypt`, `crypt32`, `ws2_32` (rustls and the
  node connection) and `gdiplus` (in System32 on every Windows since XP;
  first refused by the import check, then added to its list).
- **Under wine 9.0** (the installed copy, `GPUI_DISABLE_DIRECT_COMPOSITION=1`,
  the `icuuc` stand-in beside it): the log shows `Created device with
  Direct3D 11.1 feature level`, the kit adds its embedded faces, and the
  first text ends it at the same `failed to resolve font '.SystemUIFont'`
  as the hello window. So wine shows the app starts, opens its renderer
  and loads its fonts, and cannot show a single drawn screen. Hangul
  input, the pairing flow and a node's sessions on Windows are not
  verified anywhere yet.
- **The installer under wine** (64-bit wine; a 32-bit NSIS stub needs
  32-bit wine, which this machine does not have, so the installer is
  built for amd64): `setup.exe /S` exits 0 and writes `Vessel.exe`,
  `LICENSE.md`, `README.txt` and `uninstall.exe` under
  `%LOCALAPPDATA%\Programs\Vessel`, a `Vessel.lnk` in the user's Start
  menu, and the HKCU uninstall entry (`DisplayName Vessel`,
  `DisplayVersion 0.0.0.0.1`, `Publisher Team-WICKED`, the uninstall and
  quiet-uninstall strings). `uninstall.exe /S` removes all of it,
  including an `Vessel.exe.old-*` and a `.vessel-update-*.part` placed
  there to stand for the updater's leftovers. The installer's own pages
  (directory choice, finish with "run Vessel") were not seen.
- **The fonts' licence**: the app embeds Geist, Geist Mono and
  Pretendard (SIL OFL 1.1), whose licence asks every copy to carry its
  text; the zip, the installer and the tarball carry them in
  `licenses/` (added at 03:30 UTC; the first 03:26 set lacked them).
- **Not done on this machine and why**: a drawn frame of the app (wine's
  DirectWrite), SmartScreen's warning (Windows only), DPI scaling on a
  real display.

## Build 0.0.0.0.2

Built by `build.sh 0.0.0.0.2 windows-x64 linux-x64` at `d45fd96`, the
merge of the desktop track's `5211c3c` (its gate passing), the set put
in place at 04:48 UTC; `sha256sum -c` and `vessel-app-feed check-sums`
(the signature with the release key's public half) both pass where the
owner downloads from, as `build.sh`'s last two steps. Updates are not
compiled in (no feed address yet).

- **Windows**: 37 MB, GUI subsystem, static CRT; the import list is the
  one `build.sh` allows; resources: icon, group icon, version block and
  GPUI's manifest (`build.sh` checks all four). The installer installs,
  registers `DisplayVersion 0.0.0.0.2` with the exe as `DisplayIcon`, and
  uninstalls, under wine.
- **Under wine** the build now stops earlier than 0.0.0.0.1: the
  Direct3D 11.1 device is created and the first text (now resolved
  before the main window opens) ends it at wine's DirectWrite, so not
  even the window's styles can be seen running.
- **The title bar** is the app's own (the desktop track's `chrome.rs`):
  on Windows GPUI keeps `WS_SYSMENU | WS_THICKFRAME | WS_MAXIMIZEBOX |
  WS_MINIMIZEBOX` on the window and hides the system bar by answering
  `WM_NCHITTEST` itself: the app's `Drag`, `Min`, `Max` and `Close` areas
  become `HTCAPTION`, `HTMAXBUTTON` and `HTCLOSE`, the top edge resizes
  (read in `gpui-pre-windows` 0.3.10, the path Zed ships on Windows). Not
  run on any Windows. If the drag or the buttons fail, the system menu
  and the keyboard still work, because the styles are there: Alt+F4
  closes, Alt+Space opens the window menu (Move, Size, Close), Win+arrows
  snap, maximise and minimise, the taskbar's menu closes it. There is no
  switch back to the system's bar; one would be a line in the kit's
  `window_options` (the desktop track's).
- **Linux**: 71 MB; the tarball started from an empty configuration under
  a virtual display and drew the app's bar, the mark and the pairing
  screen.

## Icons (from build 0.0.0.0.2)

The owner gave the mark on 2026-11-13 (`docs/design/brand/`, merged from
`backend/m11` at `ad783be`; its README says how it is used). Every icon
is made from `vessel-mark.svg` by `tools/make-icons.py` at build time
into the build's stage; nothing generated is committed.

- **The tile**: the mark in white on a `#1a5868` square with corners
  rounded at 18.75% of its side, the mark filling 86% of it.
- **Rasteriser**: resvg 0.50 (installed under the tools directory with
  `cargo install`; MPL-2.0, a build tool only). ImageMagick 6 here
  renders SVG with its own MSVG renderer: drawn at 256 px beside
  resvg's, its nested lines come out thicker and merged, unlike the
  owner's paths; at 16 to 24 px it looks bolder only because it is not
  the mark. Each size is rasterised from the vector at that size.
- **What the small sizes look like** (frames of the `.ico`, enlarged):
  at 16 and 20 px the nested lines merge into a pale V on the tile and
  the star is a white dot; at 24 px the star reads as a star and the V
  as one band; at 32 px the V shows a hint of its lines; from 40 px the
  lines separate and from 64 px it is the mark. As the brand README
  says, a version drawn for small sizes would be the owner's to give.
- **Windows**: `vessel.ico` (16, 20, 24, 32, 40, 48, 64, 256; PNG
  frames, which Windows reads from Vista on) is compiled with a version
  block (`FileDescription` Vessel, the label as product and file
  version, so Task Manager says "Vessel") by `llvm-rc` into a `.res`
  that lld-link links beside GPUI's manifest. `build.sh` refuses an exe
  without the icon, the group icon, the version block and the manifest.
  The installer and its uninstaller use the same `.ico`; its pages show
  the mark in the brand colour on the white header (150 x 57) and in
  white on the brand colour beside the finish page (164 x 314). Seen
  under wine: the directory page with the header mark at the right.
- **Linux**: the tile at 16, 24, 32, 48, 64, 128, 256 and 512 px under
  `icons/hicolor/<n>x<n>/apps/vessel-app.png` in the tarball, a
  `vessel-app.desktop.in`, and `install-desktop-entry.sh`, which copies
  the icons into `~/.local/share/icons` and writes a menu entry pointing
  at the unpacked `vessel-app` (`--remove` takes both away).
- **Not mine**: the app's own drawing of the mark (title bar, sidebar,
  the window's icon at run time) is the desktop track's.

## The installer

NSIS (`windows/installer.nsi`, the Decision Log is at its top), built on
Linux by `makensis` 3.09. Per user (`RequestExecutionLevel user`), into
`%LOCALAPPDATA%\Programs\Vessel`, a Start menu shortcut, an
"Installed apps" entry under HKCU with an uninstaller, Korean first and
English. Zed's Inno Setup and WiX need a Windows-only compiler; MSIX
needs a signing certificate to install at all.

## The title bar's buttons on Windows (build 0.0.0.0.3)

The owner's first report from a real Windows 11, on 0.0.0.0.3: minimise,
maximise and close needed a double click.

- **Cause** (read in `gpui-pre` 0.3.10 and the kit, 2026-11-13):
  - GPUI answers Windows' `WM_NCHITTEST` with the first window-control
    area, in paint order, whose hitbox is in the window's hit test
    (`on_hit_test_window_control` in gpui's `window.rs`).
  - The kit's bar (`Drag`) is the parent of the three buttons, so it is
    painted first. The buttons did not occlude it, so over a button the
    bar's hitbox was under the mouse too and Windows was told
    `HTCAPTION`.
  - One click was therefore a press on the caption: `DefWindowProc`
    starts the move loop and the release ends it. A double click was a
    caption double click, which maximises or restores whichever button
    it is on.
  - The platform side is not at fault. `gpui-pre-windows` records the
    button on `WM_NCLBUTTONDOWN` and acts on `WM_NCLBUTTONUP` when the
    same button is released. Zed's caption buttons sit inside its drag
    bar in the same way and occlude it (read only,
    `platform_title_bar/src/platforms/platform_windows.rs`).
- **Fix**: the desktop track's `7f120ca`: every window control occludes
  the bar, and on Windows the control area is all there is (no click
  handler of the app's).
- **The check that would have caught it**: the kit's test
  `chrome::tests::over_the_window_controls_the_bar_is_not_the_drag_area`
  (`aa80a37`). It puts a scroll listener on the bar, whose predicate
  (`Hitbox::should_handle_scroll`, "the hitbox is in the hit test") is
  the one the Windows hit test uses for each control area. It then
  scrolls over close, maximise, minimise and the trailing element and
  fails if the bar is reached. It fails on `7f120ca` with the occlusion
  removed and passes with it. GPUI's test window has no public way to
  ask the hit test itself, so this is the closest observable stand-in.
- **Under wine 9.0**, before and after:
  - The window had only the kit's title bar and no text. It was a
    scratch example, not committed: `title_bar` in a window from
    `window_options`.
  - It ran inside wine's own virtual desktop (`explorer /desktop`), so
    wine's user32 handles the non-client area as Windows does; under a
    real X window manager, wine let the manager decorate the window.
  - Each action was one `xdotool` click on display :91, logged with
    `WINEDEBUG=+message`.

  | | Before (`4a771e3`'s kit) | After (`7f120ca` + `aa80a37`) |
  |---|---|---|
  | Hit test over minimise, maximise, close | 2 `HTCAPTION` on all three | 8 `HTMINBUTTON`, 9 `HTMAXBUTTON`, 20 `HTCLOSE` |
  | One click on minimise | `WM_SYSCOMMAND` `0xF012` (move), nothing | `WM_SIZE` `SIZE_MINIMIZED` |
  | One click on maximise | the same, nothing | `WM_SIZE` `SIZE_MAXIMIZED`; one more click restores (`SIZE_RESTORED`) |
  | One click on close | the same, nothing | `WM_CLOSE`, `WM_DESTROY` |
  | Double click on close | `0xF030` (`SC_MAXIMIZE`): maximised | not needed |

- **Only a real Windows shows**:
  - Windows 11's snap-layout flyout when hovering maximise. It needs
    `HTMAXBUTTON`, which is now answered.
  - The buttons' hover and pressed colours under DWM at the owner's
    scaling.
  - The owner's own single clicks.

## Updates (question 8)

### What the references do (read 2026-11-13; behaviour only)

**Zed** (GPL; its `auto_update` crates were read, nothing taken):
polls hourly (nightly every 15 minutes), never in a dev build; asks
`/releases/<channel>/latest/asset?asset=zed&os=..&arch=..` for a version
and a URL; downloads, and installs before the restart: macOS mounts the
DMG and copies the bundle over itself, Linux unpacks a tarball over
`~/.local/zed.app`, Windows runs its Inno Setup installer silently into
a staging directory and leaves the swap (with rollback) to a helper
program that waits for Zed to exit. No signature of its own: TLS, and
the systems' code signing (Developer ID and notarisation on macOS, Azure
Trusted Signing on Windows; Linux unsigned). The UI says "restart to
update", the user chooses when.

**Tauri 2's updater** (MIT or Apache-2.0, `tauri-plugin-updater` 2.14.1)
and its standalone form **cargo-packager-updater** 0.2.3: endpoints with
`{{target}}`/`{{arch}}`/`{{current_version}}` in the URL, answering a
static `latest.json` (version, notes, date, per-platform URL and
signature) or a dynamic answer; a minisign signature per artifact,
verified with `minisign-verify` against a public key compiled in;
semver ordering; on Windows it runs the NSIS or MSI installer and exits;
on macOS it unpacks a `.app.tar.gz` and swaps the bundle with a backup;
on Linux it replaces the AppImage.

### Code or design: cargo-packager-updater was not taken

Read against ADR 0014 and the lints, and decided not to link it:

- **Labels are not semver.** The owner's builds are `0.0.0.0.1`, five
  numbers; its comparator and its manifest take `semver::Version` only.
- **The manifest is unsigned.** Only artifacts are signed, so whoever
  controls the host can serve an older signed build as the newest; a
  channel is not bound either.
- **The whole artifact in memory** (`download` returns a `Vec<u8>`), and
  blocking `reqwest` on a thread of its own.
- **It exits the process** (`std::process::exit(0)` after starting the
  installer on Windows), from inside a library, past the app's own
  shutdown; Vessel's lints forbid `exit` in our code for that reason.
- **Windows updates through an installer**, so the portable zip could
  not update, and the installer's elevation and UI are in the path.
- **Its tree**: `reqwest` 0.12 (Vessel's is 0.13), `thiserror` 1,
  `dirs` 5, a `ctor` (code run at load), and its last release was
  2025-07; a second HTTP stack in the app for one feature.

Taken as design: the minisign format (also ADR 0008's choice), the
public key compiled in, a feed keyed by system, the background check
and the "restart to update" of Zed.

### What was built: `vessel-app-updater` and `vessel-app-feed`

- **Feed**: a directory any static HTTPS host serves. `<channel>.json`
  is an envelope `{ "manifest": "<text>", "signature": "<minisign>" }`;
  the manifest (schema 1) names the channel, the label, when, notes,
  and per target and packaging an artifact's path, size and SHA-256.
  Artifacts are under `<label>/`, beside a `SHA256SUMS` and its
  `SHA256SUMS.minisig` for people checking by hand. The dist root is the
  feed's base, so the coordinator serves `<dist>/` as it is.
- **Why one signed envelope** (Decision Log in `manifest.rs`): one
  signature binds version, channel and every artifact; a publish is one
  rename, so a reader sees the old feed or the new; the text is verified
  before it is parsed.
- **The app**: five seconds after start and then hourly (a sleeping
  machine checks once on waking), reads the envelope (at most 256 KiB),
  verifies it, compares labels; if newer, streams the artifact for its
  target and packaging into `.vessel-update-<label>.<pid>.part` beside
  itself, refusing it past the signed size, hashing as it goes, then
  `fsync`; swaps (Windows: rename the running exe to
  `<name>.old-<label>`, rename the new one in, rename back on failure;
  Unix: `chmod 755`, rename over); publishes `Ready`, stops checking.
  Restart starts the new executable with the same arguments and
  `VESSEL_APP_RESTARTED_FROM=<label>`; the app then quits through GPUI.
  Leftovers are removed at the next start (bounded to 4,096 directory
  entries).
- **Settings at build time** (`VESSEL_BUILD_LABEL`, `VESSEL_UPDATE_FEED`,
  `VESSEL_UPDATE_KEY`, `VESSEL_UPDATE_CHANNEL`): no host name in the
  repository; a build without them never updates; the crate's build
  script fails a malformed one.
- **HTTPS** as the main workspace does it: `reqwest` 0.13 with rustls,
  `ring` and the system's roots; redirects refused; `http://` only to a
  loopback address (tests, and trying a feed locally).
- **The key**: made once with `vessel-app-feed keygen` outside the
  repository (directory 0700, secret 0600, created with that mode),
  unencrypted so a script can sign; never printed, never in a worktree
  or an artifact. Its public half is the build's `VESSEL_UPDATE_KEY`.
- **macOS** (`app-bundle` packaging) is named in the feed and the state,
  but its swap is not built: it answers `InstallError::Unsupported`. It
  cannot be run here; it comes with the CI's macOS job.

Tests (`cargo test` in `spikes/release`): 26 unit tests of the crate
(labels, channels, paths that try to leave the feed, digests, bounds, a
tampered manifest, another key, another channel, a later schema, a
duplicate, the swap, a failed swap, the clean-up, the memory file, and
on Windows the "Installed apps" entry), 7 end-to-end tests through a
feed served over loopback HTTP (an update installed and the staging
files gone; the same label is no update; bytes other than the signed
ones never installed and the download removed; another key's feed
refused; a missing feed retried; with automatic updates off only an
asked-for check runs; `set_automatic(true)` changes a running updater),
6 of the feed tool (arguments, key file modes, refusal to replace a key,
publish then check, a changed artifact caught, signed sums).

### End to end, on Linux, with the real app

2026-11-13, a feed on a loopback static server (`python3 -m http.server`):

1. `build.sh 0.0.0.0.91 linux-x64 --publish` with the feed's address and
   the release key's public half compiled in: a 52 MB `vessel-app`, the
   tarball, signed sums, and `preview.json` written and then read back by
   `vessel-app-feed check` (`linux-x86_64 executable ... ok`). The build
   took 33 min (fat LTO on a machine with a load average of 30 to 46).
2. A label `0.0.0.0.92` published by `vessel-app-feed publish`: the
   0.0.0.0.91 executable with 16 bytes appended (another file that still
   runs), with notes.
3. The 0.0.0.0.91 executable copied into a scratch directory and started
   under a virtual display with an empty configuration. The server's log:

   ```
   "GET /preview.json HTTP/1.1" 200 -
   "GET /0.0.0.0.92/vessel-0.0.0.0.92-linux-x64 HTTP/1.1" 200 -
   ```

   The app's: `an update is installed and runs from the next start`. The
   file on disk was then byte for byte 0.0.0.0.92's, mode 0755, with no
   staging file left beside it, and the window's foot read "Vessel
   0.0.0.0.92 is installed and opens the next time Vessel starts." while
   the sidebar's foot still said 0.0.0.0.91 (the running build).

### The Windows swap, under wine

The updater's tests built for `x86_64-pc-windows-msvc` (`cargo xwin test
--no-run`) and run under wine 9.0: the unit tests pass (26 on
2026-11-13, the "Installed apps" entry's included), and the end-to-end
ones (loopback HTTP, a real download and swap; 7 on 2026-11-13, both
`set_automatic` tests included) pass once the first one expects what
Windows does: the old executable is renamed to
`vessel-app.old-0.0.0.0.1` beside the new one (Windows will not delete
an executable file in use), to be removed at the next start; on Linux
the old file is simply replaced. Wine emulates Windows' file semantics
and does not lock a running executable as Windows does, so this shows
the Windows code path runs, not that Windows lets it rename the running
`Vessel.exe` (Windows does allow renaming a running image; the first
real-Windows update is the proof).

### What the app calls

This track first wired the updater into `spikes/native/app` as one
module and two lines in `main.rs`. The desktop track then rewrote
`app/src/update.rs` around it (`f008aa0`, the update UI as Zed shows it).
The parts are:

- **The title bar's status**: nothing while idle, "Checking for
  updates…", "Downloading update… 42%", "Restart to update" once a build
  is in place, or a quiet failure with "Try again".
- **The About screen**: "Check for updates" (`Updater::check_now`) and
  an "Update automatically" switch.
- **After an update**: the first start of the new build says "Updated
  to …" once, with the feed's notes.

What the app uses from the crate:

- `settings_from_build()`, `Installation::from_process`,
  `Updater::start`, `state()` (a watch), `check_now()` and
  `restart(args)`.
- `set_automatic(bool)` with `automatic()` (`2150243`, tested in
  `ef832a4`). It turns the hourly checks off and on in a running
  updater. The desktop track's first version starts a second updater
  with a year-long period instead, and it is to switch to
  `set_automatic`.
- On Windows the crate also updates the installer's "Installed apps"
  entry (`DisplayVersion`) after an update. It does this only when the
  entry's `InstallLocation` is the updated executable's directory, so a
  copy from the zip is left alone.

From build 0.0.0.0.3 the builds have the feed compiled in (see "Builds
with updates" below).

## Builds with updates

From 0.0.0.0.3 the builds have the feed's address and the release key's
public half compiled in. `build.sh` reads the address from a file
outside the repository, and neither the logs nor the README print it.
Every set is published under a new label, and a published label is
never rebuilt.

- **0.0.0.0.3**:
  - Built from `2a14c42` and published to `preview` at 05:41 UTC.
  - Its update UI is this track's first one: a line at the window's
    foot, "Vessel <label>을(를) 설치했습니다. 다음에 Vessel을 열면 이
    빌드로 시작합니다." It has none of the desktop track's `f008aa0`: no
    "Restart to update", no remembered label, no notes.
- **0.0.0.0.4**:
  - Built from `ab71e0a`: the desktop track's title-bar fix `7f120ca`,
    the kit test `aa80a37`, `set_automatic` wired to the switch, and the
    corrected mark.
  - The set went into place at 07:00 UTC; `preview.json` was published
    at 07:01:55 UTC with notes.
  - Files:

    | File | Bytes |
    |---|---|
    | `vessel-0.0.0.0.4-windows-x64-setup.exe` | 10,580,977 |
    | `vessel-0.0.0.0.4-windows-x64.exe` | 38,099,968 |
    | `vessel-0.0.0.0.4-windows-x64.zip` | 15,297,027 |
    | `vessel-0.0.0.0.4-linux-x64` | 74,510,872 |
    | `vessel-0.0.0.0.4-linux-x64.tar.gz` | 25,189,636 |

  - Checked before publishing:
    - The eight icon frames (16 to 256) in the exe and in the installer
      are byte for byte the frames `make-icons.py` makes now from
      `vessel-mark.svg` (`d63f0da`, view box `170 170 1660 1660` read
      from the file). Every frame differs from 0.0.0.0.3's.
    - The exe's version block says 0.0.0.0.4.
    - Under wine it gets as far as every build does: it creates the
      Direct3D 11.1 device and stops at the first text (wine's
      DirectWrite).
    - `sha256sum -c` and the signature on the sums pass.
  - **The update the owner's machine makes**, shown on Linux with the
    published 0.0.0.0.3:
    - Its tarball was unpacked into a scratch directory, with an empty
      configuration, and nothing pointed at it but the feed it was built
      with.
    - It was started under a virtual display after the publish. Six
      seconds later the file was 0.0.0.0.4's bytes (mode 0755, no
      staging file left), and the window's foot said 0.0.0.0.4 is
      installed and opens at the next start.
    - The next start ran 0.0.0.0.4.
  - **No "Updated to" note this time**: 0.0.0.0.3 neither remembers the
    label it ran nor keeps the feed's notes. The note and the notes
    appear from the next update on, which 0.0.0.0.4 downloads.
  - **Only the owner's Windows shows**:
    - The rename of the running `Vessel.exe` (the old file is kept as
      `Vessel.exe.old-…` and removed at the next start).
    - "Installed apps" showing 0.0.0.0.4.
    - The one-click buttons and the snap layouts.

- **0.0.0.0.5**:
  - Built from `0a41166`, the same code as the desktop track's
    `83f8dd2`; only this document differs.
  - The set went into place at 07:20 UTC and is **not published** yet:
    the coordinator first runs the Linux build against the real node.
    Its notes wait in a scratch file.
  - Files: setup 10,595,011, exe 38,134,272, zip 15,312,999, linux
    74,581,328, tar.gz 25,207,949 bytes.
  - The checks are 0.0.0.0.4's: icon frames equal to the corrected
    mark's, version block 0.0.0.0.5, wine stops at the first text, sums
    and signature pass.
- **0.0.0.0.5** was published to `preview` at 07:24:17 UTC.
- **The first update on a real Windows (open, 2026-11-13)**:
  - The owner started the installed 0.0.0.0.3 and saw nothing at the
    foot.
  - The feed's server logged two manifest reads (07:06:54 and 07:07:14
    UTC) and no completed transfer of the Windows artifact.
  - 0.0.0.0.3 to 0.0.0.0.5 log only to stderr, which a GUI-subsystem
    process started from the Start menu does not have. So a Windows
    failure was lost, and 0.0.0.0.3 does not show it either. It retried
    at the next start or an hour later.
  - The candidates, ranked, went to the coordinator: the app closed
    during the download, an antivirus stopping an unsigned download, or
    the staging file refused beside `Vessel.exe`. A redirected stderr
    (`Vessel.exe 2> file` from cmd) gets the error from 0.0.0.0.3.

- **0.0.0.0.6**:
  - Built from exactly `e540d74` and put in place at 08:54 UTC. It is
    not published yet: the coordinator runs the Linux build against the
    real node first.
  - Files: setup 10,618,535, exe 38,234,624, zip 15,345,409, linux
    74,692,840, tar.gz 25,241,365 bytes. The icons, the version block,
    the sums and the signature were checked as for 0.0.0.0.4.
  - It is the first build with the log file. Started with a scratch
    configuration, the Linux build wrote
    `$XDG_STATE_HOME/vessel/logs/vessel.log`:
    - the first line is `Vessel starts label="0.0.0.0.6" os="linux"`
    - the updater's lines follow (`feed=https://<host>/…`, a check, "up
      to date")
    - the feed's path appears nowhere (counted, not printed)
  - Under wine, the Windows build wrote
    `%LOCALAPPDATA%\Vessel\logs\vessel.log`, with wine's font panic
    recorded with its place.
  - Seen there, and fixed after 0.0.0.0.6 was published (08:58:17
    UTC):
    - Span fields carried stderr's colour codes into the file. The file
      layer now has a field formatter of its own (`4125c37`).
    - On Windows the backtrace's frames were `<unknown>`, because no
      `.pdb` ships. The panic line now also has `frames="exe+0x<rva> …"`
      (`7b8d66f`). `build.sh` keeps each label's `vessel_app.pdb`, with
      the exe's SHA-256, under `vessel-app-symbols/<label>/` beside the
      dist root, never in the dist folder (`cfe9bd8`).
    - 0.0.0.0.6's own `.pdb` was copied there by hand; its GUID matches
      the exe's. Those of 0.0.0.0.1 to 0.0.0.0.5 were already
      overwritten.
    - `tools/pdb-frames.py <exe> <pdb> <vessel.log>` names the frames
      from the `.pdb`'s public symbols. On a scratch build under wine,
      the font panic read as gpui `TextSystem::resolve_font` ←
      `gpui_component` `resolve_default_font` ← `gpui_component::init`
      ← `vessel_app_kit::init` ← `vessel_app::main`.
    - The release profile has no debug info, so the app's frames have
      function names but no lines. `debug = "line-tables-only"` in the
      desktop track's release profile would add the lines to the
      `.pdb`; the exe itself is not changed on Windows.

- **0.0.0.0.7**:
  - Built from exactly `fd61c23`: the desktop track's line tables in
    the release profile (`7638b83`), and the Linux debug information
    split out by `build.sh`.
  - The set went into place at 11:21 UTC. It is not published yet.
  - Files: setup 10,897,729, exe 39,547,392, zip 15,789,705, linux
    74,383,456, tar.gz 25,448,397 bytes.
  - The shipped Linux executable is the stripped one, 309,384 bytes
    smaller than 0.0.0.0.6's. Its only `.debug_*` section left is
    `.debug_gdb_scripts`, a small loaded section. Its `.gnu_debuglink`
    names `vessel-app.debug`, with a matching CRC.
  - `vessel-app.debug` (230,620,128 bytes) and `vessel_app.pdb`
    (117,829,632 bytes) are in `vessel-app-symbols/0.0.0.0.7/` with
    their SHA256SUMS, and not in the dist folder.
  - The shipped executable started under a virtual display with a
    scratch configuration. Its `vessel.log` has no escape codes and no
    feed path.
  - One address resolves to a file and line on each system:
    - `vessel_app_kit::init`+0x30 → `spikes/native/kit/src/lib.rs:122`
      (`addr2line` with the `.debug`)
    - the same function on Windows → `lib.rs:121` (`llvm-symbolizer
      --relative-address` with the `.pdb`)
  - The icons, the version block, the sums and the signature were
    checked as before.

- **The signature's comment** (fixed before 0.0.0.0.8):
  - `SHA256SUMS.minisig` is signed while the set sits in its hidden
    staging directory. Up to 0.0.0.0.7 its trusted comment named that
    directory ("vessel-app .0.0.0.0.7.partial.<pid> SHA256SUMS"), and
    anyone verifying the sums sees the comment.
  - `vessel-app-feed sums` now takes `--label`, and the comment is
    "vessel-app <label> SHA256SUMS" whatever directory the files are
    in. `build.sh` and the CI file pass the label.
  - Labels 0.0.0.0.1 to 0.0.0.0.7 keep their old comment, because
    published labels are never rebuilt.

- **0.0.0.0.8**:
  - An earlier build from `ccfba8e` was held and set aside, never
    published. The real build is from `c802da9`: the desktop lead's
    `779891d4` (the permission card as Paseo's, the Linux platform
    patch `gpui-pre-linux`, the Fluent translations) plus this track's
    signature comment fix. Only `docs/` and `spikes/release/` differ
    from `779891d4`.
  - The set went into place at 18:53 UTC and was published to
    `preview` at 18:59:58 UTC, after the coordinator ran the Linux build
    against the real node. The notes gained one line asking the owner
    to check Enter during Hangul composition on Windows.
  - Checked from outside, through the public feed: the manifest names
    0.0.0.0.8, and all five files download with matching sums and
    signature.
  - The published 0.0.0.0.7 Linux build, unpacked with a scratch
    configuration, took 0.0.0.0.8 by itself at 19:01:18 UTC, and its
    log shows each step. The held `ccfba8e` set was then deleted.
  - The signature's comment is "vessel-app 0.0.0.0.8 SHA256SUMS".
  - Sizes against 0.0.0.0.7:

    | File | Bytes | Change |
    |---|---|---|
    | setup | 11,638,832 | +741,103 |
    | exe | 42,672,128 | +3,124,736 |
    | zip | 16,872,307 | +1,082,602 |
    | linux | 77,856,496 | +3,473,040 |
    | tar.gz | 26,621,127 | +1,172,730 |

  - The patched `gpui-pre-linux` compiled from its path in the release
    build.
  - The new crates (fluent-bundle, fluent-syntax, fluent-pseudo,
    fluent-langneg, intl-memoizer, intl_pluralrules, unic-langid,
    type-map, self_cell) are all Apache-2.0 or MIT; for self_cell,
    "Apache-2.0 OR GPL-2.0-only", Apache-2.0 is taken. The bundle's
    licence files are unchanged (the three OFL texts).
  - The usual checks passed: icons, version block, stripped Linux exe
    with a matching debuglink, symbols kept, the shipped exe starts and
    logs without escape codes or the feed's path.

- **0.0.0.0.9**:
  - Built from exactly `7081463`, the desktop lead's gated head, which
    holds this track's branch. It speaks node protocol 13.
  - The set went into place at 21:17 UTC. It was published to
    `preview` at 21:21:43 UTC, after the coordinator paired it with the
    real node, which still speaks protocol 11.
  - The notes gained three lines: what was checked against the older
    node, what was not checked, and what is known.
  - From outside, the public feed serves the 0.0.0.0.9 manifest and
    all five files with matching sums and signature. The published
    0.0.0.0.8 Linux build took 0.0.0.0.9 by itself at 21:22:46 UTC.
  - Files: setup 12,353,161, exe 44,480,512, zip 18,004,043, linux
    81,060,120, tar.gz 28,165,806 bytes.
  - No new crates. The two added faces are embedded in the executable,
    and their OFL texts are the bundle's existing three.
  - The usual checks passed: the signature's comment names 0.0.0.0.9,
    icons, version block, stripped Linux exe with a matching debuglink,
    symbols kept. The shipped exe starts and its log has no escape
    codes, no ERROR line and no feed path.
  - In Korean, About showed "앱이 최신 상태입니다." with 확인 enabled
    and 업데이트 disabled, because the feed held 0.0.0.0.8.

- **0.0.0.0.10**:
  - Built from `81148c8`: the desktop lead's `397e7f44` plus this
    track's documents and one release fix.
  - The fix: from `397e7f44` the app embeds the official Gemini CLI icon
    (a 46,696-byte PNG) under Apache-2.0, whose section 4(a) asks that
    the licence travel with the object form.
    - `build.sh` now copies every text in `docs/design/agent-marks/` into
      each package's `licenses/`.
    - The uninstaller removes `licenses\*.txt`, not only the fonts' texts.
    - A build already running from `397e7f44` was stopped by its process
      group and its staging directory removed before this fix went in.
  - The set went into place at 22:39 UTC and was published to
    `preview` at 22:44:11 UTC, after the coordinator's check on the real
    node (protocol 11).
  - From outside, the public feed serves the manifest and all five files
    with matching sums and signature. The published 0.0.0.0.9 Linux
    build took 0.0.0.0.10 by itself at 22:45:16 UTC.
  - The PNG's bytes are inside both the Windows exe and the Linux
    executable.
  - Sizes, with the change against 0.0.0.0.9:

    | File | Bytes | Change |
    |---|---|---|
    | setup | 12,460,513 | +107,352 |
    | exe | 45,014,016 | +533,504 |
    | zip | 18,303,454 | +299,411 |
    | linux | 81,594,744 | +534,624 |
    | tar.gz | 28,502,801 | +336,995 |

  - It is the first two-digit label. The updater compares labels as
    numbers: 0.0.0.0.10 called the feed's 0.0.0.0.9 not newer.
  - The usual checks passed: the signature's comment, icons, version
    block, stripped Linux exe with a matching debuglink, symbols kept,
    and a clean log.

- **0.0.0.0.11**:
  - Built from exactly `f0c1d540`, the desktop lead's head, which holds
    this track's branch up to `d2260ce3`. The app speaks protocol 14.
  - The set went into place at 23:55 UTC and was published to
    `preview` at 00:00:36 UTC on 2026-11-14, after the coordinator's
    check on the real node.
  - From outside, the public feed serves the manifest and all five files
    with matching sums and signature. The published 0.0.0.0.10 Linux
    build took 0.0.0.0.11 by itself at 00:01:53 UTC.
  - Sizes, with the change against 0.0.0.0.10: setup 12,592,815
    (+132,302), exe 45,623,296 (+609,280), zip 18,539,862 (+236,408),
    linux 82,100,320 (+505,576), tar.gz 28,671,250 (+168,449).
  - No new crates and no new assets.
  - The usual checks passed. The shipped Linux build made its log
    folder 0700 and `vessel.log` 0600 (`02d8fe0b`).

- **0.0.0.0.12**:
  - Built from `509ef7f`: the desktop lead's `280927b0` (protocol 15)
    merged into this branch. Only `docs/` differs from `280927b0`.
  - The set went into place at 00:35 UTC on 2026-11-14.
  - Sizes, with the change against 0.0.0.0.11: setup 12,680,258
    (+87,443), exe 46,038,528 (+415,232), zip 18,683,359 (+143,497),
    linux 82,481,888 (+381,568), tar.gz 28,808,401 (+137,151).
  - No new crates and no new assets. The usual checks passed.
  - **Built and withheld; the label is skipped.** The coordinator's
    check on the real node found the new Settings › Sync page telling an
    admin device that it has no permission and should use an admin
    device. That sentence is wrong for a refusal from a standalone
    protocol 11 node. Everything else in that check passed.
  - The folder stays where it is. It is never published, and the label
    is never rebuilt. The feed served 0.0.0.0.11 until 0.0.0.0.13.
  - 0.0.0.0.13 hides Settings › Sync until its page is right on every
    kind of machine (`ce880e4d`), and carries the owner's fixes to the
    marks and the slider.

- **0.0.0.0.13**:
  - Built from exactly `9461f149`, the desktop lead's head (protocol
    15), checked out detached; this branch then merged it.
  - The set went into place at 03:46:57 UTC on 2026-11-14 and was
    published to `preview` at 03:53:49 UTC, after the coordinator's
    check on the real node.
  - From outside, the public feed serves the manifest and all five files
    with matching sums and signature. The published 0.0.0.0.11 Linux
    build took 0.0.0.0.13 by itself at 03:56:13 UTC, past the withheld
    label.
  - Sizes, with the change against 0.0.0.0.11, the last published:

    | File | Bytes | Change |
    |---|---|---|
    | setup | 12,746,999 | +154,184 |
    | exe | 46,142,976 | +519,680 |
    | zip | 18,566,980 | +27,118 |
    | linux | 82,670,256 | +569,936 |
    | tar.gz | 28,732,276 | +61,026 |

  - The Gemini CLI PNG is gone. Five 64-byte slices of it are found in
    0.0.0.0.11's executables and in neither of 0.0.0.0.13's, which carry
    the spark's vector path instead. Against the withheld 0.0.0.0.12
    the executables grew (exe +104,448, linux +188,368) while the zip
    and tar.gz shrank (-116,379, -76,125).
  - The spark comes from `google/adk-docs` under Apache-2.0
    (`docs/design/agent-marks/README.md`), so `google-Apache-2.0.txt`
    still belongs to something shipped. Every package carries the 4
    texts, as before.
  - The new card paints per frame while it moves or shows the top stop's
    field. The packaged Linux build's CPU was read against two fake
    nodes (`scripts/fleet.sh`) on Xvfb with Mesa's software Vulkan. That
    is an upper bound, not what a desktop with a GPU sees.

    | State | This track | Coordinator (real node) |
    |---|---|---|
    | card closed | 7.3% of a core | about 6% |
    | open, middle stop (High) | 7.9% | about the same |
    | open, top stop (Ultracode) | 307% | about 310% (Fast on: 320%) |

    The frame-rate work goes to the desktop lead for the next build.
  - The usual checks passed. The 4 headless WARNs (no desktop portal on
    Xvfb) are the same as in 0.0.0.0.12's run.

- **0.0.0.0.14**:
  - Built from exactly `a792d766`, the desktop lead's head (the native
    gate passed at `52344564`; above it only plans), checked out
    detached; this branch then merged it.
  - The set went into place at 06:04:53 UTC on 2026-11-14 and was
    published to `preview` at 06:55:18 UTC, after the coordinator's
    check on the real node.
  - From outside, the public feed serves the manifest and all five files
    with matching sums and signature. The published 0.0.0.0.13 Linux
    build took 0.0.0.0.14 by itself at 06:57:04 UTC.
  - The notes carry one known cost: with the composer focused, the
    caret's blink repaints the window (on the software renderer about
    31% of a core at 1280×800 and 62% at 1920×1200, the coordinator's
    reading). It is the desktop lead's first follow-up.
  - In it: the slider's fill inside its pill, popovers that take the
    pointer, the card's decorations at 30 frames a second coming to
    rest, the whole app's motion held still on a software renderer, and
    no redraw for an unchanged fleet read.
  - Sizes, with the change against 0.0.0.0.13: setup 12,751,182
    (+4,183), exe 46,177,792 (+34,816), zip 18,583,488 (+16,508), linux
    82,676,920 (+6,664), tar.gz 28,748,306 (+16,030).
  - The usual checks passed. On Xvfb the shipped Linux build logged "a
    software renderer: the app holds its motion still" and used 0.3% of
    a core over 15 s on its first screen with no node.

- **0.0.0.0.15**:
  - Built from exactly `a925f5e9`, the desktop lead's head (native gate
    green there, 705 tests), checked out detached and run through the
    machine's build slots (`cargo-slot`, slot 4 after 826 s in line);
    this branch then merged it.
  - The set went into place at 12:45:12 UTC on 2026-11-14 and was
    published to `preview` at 13:34:12 UTC, after the coordinator's
    smoke test on the real node.
  - From outside, the public feed serves the manifest and all five files
    with matching sums and signature. The published 0.0.0.0.14 Linux
    build took 0.0.0.0.15 by itself at 13:40:29 UTC. The `/dl/` page is
    behind the preview's sign-in (302 from outside); its list, rendered
    with the proxy's own function over the dist folder, puts 0.0.0.0.15
    first.
  - A fault in this track's checks, found the same hour: the debuglink
    check ran `objcopy --dump-section` with no output file, which rewrote
    the dist folder's Linux executable in place for 0.0.0.0.13, .14 and
    .15 (each before its publish: same bytes, new mtime; every label's
    sums and the three signatures verify after). The check now reads a
    copy and writes its output elsewhere, and leaves the dist file's
    mtime unchanged.
  - New in the build: the patched `gpui-base` 0.8.1 (a steady caret
    while motion is held), compiled from `spikes/release/patches/gpui-base`
    (the build log names that path).
  - Sizes, with the change against 0.0.0.0.14: setup 12,841,933
    (+90,751), exe 46,493,184 (+315,392), zip 18,700,707 (+117,219),
    linux 83,149,200 (+472,280), tar.gz 28,890,686 (+142,380).
  - The usual checks passed. The shipped Linux build on Xvfb held its
    motion (software renderer) and used 0.4% of a core on its first
    screen.

- **0.0.0.0.16**:
  - Built from exactly `cb261b03`, the desktop lead's head (native gate
    green there, 990 tests), checked out detached and run through
    `cargo-slot` (slot 2 after 718 s in line, 5 jobs); this branch then
    merged it.
  - The set went into place at 17:14:45 UTC on 2026-11-14 and was
    published to `preview` at 18:34:05 UTC, after the coordinator's
    smoke test on the real node. The notes carry one known problem: in a
    window about 800 wide with the right panel open, the conversation
    column is squeezed (fixed in the next build).
  - From outside, the public feed serves the manifest and all five files
    with matching sums and signature; the published 0.0.0.0.15 Linux
    build took 0.0.0.0.16 by itself at 18:35:39 UTC; the `/dl/` list (the
    proxy's own function over the dist folder) puts 0.0.0.0.16 first.
  - Gemini's mark is now Google's own file from Google's CDN, embedded
    byte for byte (27,794 bytes, the SHA-256 in
    `docs/design/agent-marks/README.md`); three slices of it are in both
    executables and none in 0.0.0.0.15's. No Apache-2.0 artwork is
    embedded any more; `google-Apache-2.0.txt` still ships because it
    still sits beside the marks (harmless, removable since `7151985e`).
  - Sizes, with the change against 0.0.0.0.15: setup 13,159,965
    (+318,032), exe 47,844,352 (+1,351,168), zip 19,220,192 (+519,485),
    linux 84,708,968 (+1,559,768), tar.gz 29,425,441 (+534,755).
  - The usual checks passed (the debuglink read from a copy; the dist
    file's mtime unchanged). The shipped Linux build on Xvfb held its
    motion and used 0.5% of a core on its first screen.

- **0.0.0.0.17**:
  - Built from exactly `56a7cbd3`, the desktop lead's merge of the file
    editor, checked out detached and run through `cargo-slot`'s priority
    lane (`VESSEL_SLOT_PRIORITY=1`, 5 jobs, a slot at once); this branch
    then merged it. Windows took 7 min 49 s, Linux 22 min 47 s.
  - The set went into place at 23:11:14 UTC on 2026-11-14 and was
    published to `preview` at 23:30:47 UTC, after the coordinator's
    smoke test on the real node (by then on the node line `a365d295`).
    The notes carry one known problem: ended (interrupted or exited)
    sessions are hidden in the sidebar by default; the Display menu's
    "Show ended sessions" shows them, and the next build shows them by
    default.
  - From outside, the public feed serves the manifest (equal to the
    local one) and all seven files, each byte for byte the dist folder's,
    with matching sums and signature. The published 0.0.0.0.16 Linux
    build took 0.0.0.0.17 by itself 19 s after its start (23:32:40 UTC;
    no error, no leftover `.part`, no feed token in its log). The `/dl/`
    page answers 200 on the proxy's loopback port with the public host
    name (a plain GET, the unit untouched) and lists 0.0.0.0.17 first
    with its seven files; from outside it answers 302 (sign-in).
  - Sizes, with the change against 0.0.0.0.16: setup 13,484,447
    (+324,482), exe 49,435,648 (+1,591,296), zip 19,698,508 (+478,316),
    linux 86,414,936 (+1,705,968), tar.gz 29,920,176 (+494,735).
  - The usual checks passed: icons the same as 0.0.0.0.16, the Windows
    version resource 0.0.0.0.17, the debuglink read from a copy (CRC
    matches, the dist file's mtime unchanged), the four licence texts in
    both archives, no feed token or path in the build log. The updated
    Linux build (the one that replaced 0.0.0.0.16) on Xvfb held its
    motion (software renderer) and used 0.5% of a core on its first
    screen, ERROR 0.

- **0.0.0.0.18**:
  - Built from exactly `1d1f11df` (`backend/m12-spike`, native gate
    1169 passed, root gate green), checked out detached and run through
    `cargo-slot`'s priority lane (5 jobs, a slot at once); this branch
    then merged it. Windows took 6 min 40 s, Linux 9 min 6 s.
  - The set went into place at 05:10:02 UTC on 2026-11-15 and was
    published to `preview` at 05:51:37 UTC, after the coordinator's
    smoke test on the real node. The notes carry one known problem (in
    a small window or with many projects the bottom of the sidebar is
    pushed off screen and does not scroll) and what was not checked on
    screen (the attachment file dialog, pasted file list and drag and
    drop; pressing the rename pencil; the PR view against real GitHub
    and on Windows). The terminal model and the provider removal rule
    were left out of the notes: it was not shown that either is visible
    yet.
  - From outside, the public feed serves the manifest (equal to the
    local one) and all seven files, byte for byte the dist folder's,
    with matching sums and signature. The published 0.0.0.0.17 Linux
    build took 0.0.0.0.18 by itself 10 s after its start (ERROR 0, no
    leftover `.part`, no feed token in its log). `/dl/` on loopback lists
    0.0.0.0.18 first with its seven files; from outside it answers 302.
  - Sizes, with the change against 0.0.0.0.17: setup 13,601,986
    (+117,539), exe 49,992,192 (+556,544), zip 19,887,870 (+189,362),
    linux 86,921,328 (+506,392), tar.gz 30,103,857 (+183,681).
  - Checks: icons the same as 0.0.0.0.17, the Windows version resource
    0.0.0.0.18, the debuglink read from a copy (CRC matches, mtime
    unchanged), the four licence texts in both archives, no feed token
    or path in the build log. Not measured this time (the budget was
    tight): CPU on Xvfb.

- **0.0.0.0.19**:
  - Built from exactly `d28d7551` (`backend/m12-spike`, native gate
    1253 passed and 2 ignored; the root gate of the package base
    `719bbba1` stands, as nothing outside `spikes/native` but plans
    changed), through `cargo-slot`'s priority lane (5 jobs, 164 s in
    line: the machine has 2 slots now). Windows took 8 min 9 s, Linux
    14 min 4 s.
  - New code faces: `scripts/fonts.sh` ran before the build (the build
    only fetches when a face it knows is missing) and added Geist Mono
    Bold, Italic and BoldItalic from `geist` 1.9.2; every face and
    licence text already there came out byte for byte the same. All
    four Geist Mono faces are inside both executables. The kit's only
    fallback after Geist and Geist Mono is Pretendard (no generic
    `monospace` or `serif`, TYPE-07).
  - The set went into place at 07:27:08 UTC on 2026-11-15 and was
    published to `preview` at 07:33:26 UTC, after the coordinator's
    smoke test on the real node. The notes say S2's `0690faed` fixes the
    sidebar's known problem of 0.0.0.0.18, carry two known problems
    (the Settings pencil can overlap its neighbour; a terminal's wrapped
    lines may not be rejoined after its width changes) and what was not
    checked on screen.
  - From outside, the public feed serves the manifest (equal to the
    local one) and all seven files, byte for byte the dist folder's,
    with matching sums and signature. The published 0.0.0.0.18 Linux
    build took 0.0.0.0.19 by itself 12 s after its start (ERROR 0, no
    leftover `.part`). `/dl/` on loopback lists 0.0.0.0.19 first with
    its seven files; from outside it answers 302.
  - Sizes, with the change against 0.0.0.0.18: setup 13,956,153
    (+354,167), exe 51,661,312 (+1,669,120), zip 20,524,176 (+636,306),
    linux 88,784,040 (+1,862,712), tar.gz 30,743,512 (+639,655).
  - Checks: icons the same as 0.0.0.0.18, the Windows version resource
    0.0.0.0.19, the debuglink read from a copy (CRC matches, mtime
    unchanged), the four licence texts in both archives, no feed token
    or path in the build log.

- **0.0.0.0.20**:
  - Built from exactly `01b1e220` (`backend/m12-spike`, native gate
    1277 passed and 2 ignored; outside `spikes/native` only documents
    changed since `d28d7551`, so the root gate of `719bbba1` stands),
    through the priority lane (164 s in line). Windows took 10 min 38 s,
    Linux 31 min 12 s (the machine was loaded). `fonts.sh` ran first and
    left all 15 faces and texts byte for byte as they were.
  - In place at 08:55:06 UTC on 2026-11-15, published to `preview` at
    09:02:12 UTC after the coordinator's smoke test (the palette in
    Korean, the eight themes and Ctrl+Alt+T's selection). The notes list
    a desktop that takes Ctrl+Alt+T first (Ubuntu opens a terminal with
    it) among what was not checked on screen.
  - From outside, the feed serves the manifest and all seven files, byte
    for byte the dist folder's, with matching sums and signature; the
    published 0.0.0.0.19 Linux build took 0.0.0.0.20 by itself 12 s after
    its start (ERROR 0); `/dl/` lists 0.0.0.0.20 first.
  - Sizes, with the change against 0.0.0.0.19: setup 13,951,901
    (-4,252), exe 51,643,904 (-17,408), zip 20,516,184 (-7,992), linux
    88,754,616 (-29,424), tar.gz 30,743,390 (-122).
  - Checks: icons the same as 0.0.0.0.19, the Windows version resource
    0.0.0.0.20, the debuglink from a copy, the four licence texts in both
    archives, no feed token or path in the build log.
  - The same morning the owner's Windows copy of 0.0.0.0.14 took
    0.0.0.0.19 slowly: the proxy's log shows the 51.7 MB executable
    started at 08:15:42 and broken off at 31.9 MB at 08:17:39 (about
    0.27 MB/s), with no retry reaching the feed afterwards; the same file
    came in 0.87 s from this machine through the public path, so the line
    to the owner was the limit. The owner took the 14 MB installer from
    `/dl/` instead. Follow-ups the coordinator approved: the feed answers
    a byte range since 09:08 UTC (the preview proxy, its README says how),
    and the updater resumes, downloads a compressed file and shows a
    broken-off download (ADR 0094's branch).
- **0.0.0.0.21**, the first label built the ADR 0099 way:
  - Source: `e134d16a2`. That is package base 8 (`91407173`, the lead's
    gate 1431/0/2) merged into this track's branch, which brings ADR 0094
    (companions off) and ADR 0099. Beyond base 8 it adds only the release
    workspace, the app's two update-words files, one `Cargo.lock` edge
    (`flate2`) and documents.
  - Compiled on the build VM: `--release-compile 0.0.0.0.21 windows-x64
    linux-x64`, in the priority lane the coordinator opened for labels.
    It waited 50 s and took 1038 s, Windows and Linux together. This was
    the build VM's first Windows build. The slot was found once in each
    app.
  - Packaged here at 13:58-13:59 UTC on 2026-11-15:
    - The test proof took 6 s and the real proof 6 s.
    - Both shipped apps pass `patch-feed.py verify`.
    - The sums' signature verifies with the key compiled in.
    - The build log holds neither the address nor its token.
  - Published to `preview` at 14:02:52 UTC on 2026-11-15, after the
    coordinator's Linux smoke on the real preview node over the relay
    (pairing, a session, the outline rail across 1280 → 800 → 1280, and
    the menus closing with Esc).
    - It was published with the label's own feed tool
      (`DIR/tools/vessel-app-feed`) and `--compressed gzip`. The signed
      notes are byte for byte the notes file (2,356 bytes).
    - From outside, the feed serves the manifest and all nine files, byte
      for byte the dist folder's. The sums and their signature check, and
      so do both gzip copies (each unpacks to its file). A range of the
      Windows gzip copy answers 206 with the file's own bytes.
    - The published 0.0.0.0.20 Linux build took 0.0.0.0.21 by itself 7 s
      after its start: the raw file, since 0.0.0.0.20's updater knows no
      gzip. ERROR 0, no token in its log, no `.part` left.
  - Sizes, with the change against 0.0.0.0.20: setup 14,274,020
    (+322,119), exe 53,210,624 (+1,566,720), zip 20,963,088 (+446,904),
    linux 90,384,408 (+1,629,792), tar.gz 31,201,943 (+458,553). The first
    gzip copies: exe.gz 20,856,477 (39.2% of the exe), linux.gz
    30,953,121 (34.2%).
  - Checks: icons the same as 0.0.0.0.20, the Windows version string
    0.0.0.0.21, the DLL and resource checks, the debuglink CRC, the four
    licence texts in both archives, `/dl/` lists it.

## The command line and the node in the package (ADR 0094, 2026-11-15)

The coordinator decided that the package carries `vessel` and `vesseld`
beside the app (W15's local-node controller needs the node; Paseo's desktop
ships its daemon and command line), and that updates replace the three as
one set. `docs/adr/0094-package-contents-and-whole-set-self-update.md` has
the reasons and the trade-offs. Where it stands:

- Built behind `VESSEL_PACKAGE_NODE=1` (`build.sh`, off by default): the
  zip, the installer and the tarball get the two programs in a `bin`
  folder beside the app (`bin\vessel.exe` and `bin\vesseld.exe`,
  `bin/vessel` and `bin/vesseld`; only `bin` goes on `PATH`, the layout W15
  and W19's getter use), and the dist folder gets each alone as the
  update's companions; `--publish` names them with `--companion`.
- The manifest stays schema 1; companions are an optional field the
  published apps (0.0.0.0.3 to 0.0.0.0.18) skip, and the first build with
  them puts in the companions an old app could not.
- The updater: the whole set downloaded and verified before any swap,
  companions first and the app last, every placed file put back when one
  fails; tests listed in the ADR.
- Resuming, a gzip copy of every update file, and a failed state that
  says what was kept and when the next try is (the ADR's addendum, after
  the owner's slow Windows update of 2026-11-15): these need no flag and
  reach the owner with the first label built from this branch; the gzip
  copies are made by every build from then on.
- Not shipped in any label until the local-node controller is in a build.
  Shown on 2026-11-15: `vessel-cli` and `vessel-node` build for Windows
  with `cargo-xwin` (console programs, 74.4 MB and 56.9 MB) and import
  only DLLs Windows ships. Not shown: that they run there.

## Labels compiled without secrets (ADR 0099, 2026-11-15)

The operator ruled that no compiler runs on the machine with the feed's
token and the signing key, and the build VM keeps no secret. The
coordinator chose to compile labels on the build VM with a fixed slot for
the feed's address and to write the address into the built executables
here. `docs/adr/0099-release-builds-compile-without-secrets.md` has the
reasons, the checks and the tests. Where it stands:

- Built on `backend/m12-release-adr0094` (commit 364b9345d), on top of ADR
  0094's branch. The first label built from it also brings ADR 0094's
  resuming and gzip updates. Its companions stay behind
  `VESSEL_PACKAGE_NODE`.
- The updater reads the address from a 512-byte slot through
  `black_box`. `build.rs` and `build.sh` refuse `VESSEL_UPDATE_FEED`.
- `build.sh --compile-only DIR` runs on the build VM.
  `build.sh --package-from DIR` runs here: it checks DIR, proves the slot,
  writes, verifies, packages, signs and checks with the compiled-in key.
  It signs with the feed tool that the compile step built, so nothing
  here runs `cargo`.
- The build VM's Windows cross tools were installed by this track on
  2026-11-15 and checked against this machine:
  - fxc gives the same shader bytes.
  - make-icons gives the same 18 files.
  - Where they are is recorded with the VM's provisioning notes.
- Shown without compiling, 2026-11-15:
  - `tools/test_patch_feed.py`: 13 of 13 pass.
  - A dry run of `--package-from` on stand-ins (no compiler, a throwaway
    key, a loopback feed) passed. Its 13 refusal cases each stopped in
    their place.
  - The proof script's own run: a stand-in that reads its slot asked the
    test feed after 4 s.
- Gates on the build VM, 2026-11-15:
  - The native gate of e3ae1c1ae is green: fmt, quality, clippy, build,
    and tests 1253 passed, 0 failed, 2 ignored.
  - That gate also found two non-exhaustive matches in the app's update
    words on ADR 0094's new errors, fixed in e3ae1c1ae. Earlier heads of
    the branch do not build the app.
  - The release workspace on e3ae1c1ae, through `--release-cargo`:
    - `test -p vessel-app-updater -p vessel-app-feed`: 69 passed, 0 failed.
    - `clippy --all-targets -- -D warnings`: clean.
    - 0.0.0.0.21's source has the same release workspace, apart from the
      vendored GPUI patches.
  - The updater's own Windows-only test code is not checked yet: no gate
    selection checks the release workspace for Windows. The app's Windows
    check builds the updater's library only.
- The first real compile was trial label 0.0.0.99.1, never published:
  - `--release-compile 0.0.0.99.1 linux-x64` of e3ae1c1ae took 605 s.
    The slot was found once, at 0xcc0da4.
  - Packaged here with the real address and no signing key, into a
    scratch directory that was then removed. The compiled app asked a test
    feed written into its slot after 6 s. The shipped app read the real
    feed's signed manifest after 6 s.
  - The package log holds neither the address nor its token.

## Logs and retries (from the build after 0.0.0.0.5)

- **The log file**: `spikes/release/applog` (`vessel-app-log`). The app
  calls `vessel_app_log::init(LABEL)` first thing in `main`.
  - Output goes to stderr as before, and to `vessel.log` in
    `vessel_app_log::dir()`:
    - Windows: `%LOCALAPPDATA%\Vessel\logs`
    - macOS: `~/Library/Logs/Vessel`
    - Linux: `$XDG_STATE_HOME/vessel/logs`
  - The file is rotated past 1 MiB, keeping `vessel.1.log` and
    `vessel.2.log`.
  - It is written line by line, and a panic hook records the message,
    the place and a backtrace.
  - `RUST_LOG` filters; by default everything at info and the updater
    at debug.
  - The About screen's "open the log folder" calls `dir()`. That screen
    is the desktop track's.
  - The Decision Log, with the alternatives, is at the top of
    `applog/src/lib.rs`.
- **What the updater records**:
  - its start (label, channel, target, packaging, the feed's host only)
  - each check, and the newest label
  - the artifact and its size
  - the staging file's name
  - the feed's answer to the download (status, length)
  - the bytes received when a download breaks off or stalls
  - each rename of the swap, and leftovers removed or kept
  - each failure with its whole cause chain (`report::chain`), which
    carries an `io::Error`'s OS code
  - file names only, because the directory carries the account's name
- **The feed's address never reaches a log**:
  - `FeedUrl` prints its scheme and host only, and reqwest errors are
    kept without their URL.
  - `tests/update.rs::the_log_never_names_the_feeds_path` runs a good
    update, a 404 download and an unreachable feed under a token path,
    with every updater record at trace. It fails if the token appears;
    it did fail with the URL left in reqwest's error.
- **Retries**:
  - After a transient failure the next checks come 1, 5 and 15 minutes
    later, then hourly (`RETRY_AFTER`, `Settings::retry_after`).
  - A manifest that does not verify, or no build for this machine,
    waits the hour, because only a new publish changes it.
  - Tests check the schedule's count and that a retry installs a build
    the failed check could not reach.
- Verified on Linux, and on Windows under wine: the log crate's 3
  tests, the updater's 27 unit tests and its 10 end-to-end tests.

## Licences of what this track pulls in (ADR 0014)

Beyond what Vessel's main workspace already links:

| crate | licence | where |
|---|---|---|
| `minisign-verify` 0.3.0 | MIT | the app (the updater) |
| `minisign` 0.10.0 | MIT | the feed tool and tests only |
| `ct-codecs` 1.1.8 | MIT | the feed tool (through minisign) |
| `scrypt` 0.11, `pbkdf2` 0.12, `salsa20` 0.10 | MIT or Apache-2.0 | the feed tool (through minisign) |
| `rpassword` 7.5, `rtoolbox` 0.0.6 | Apache-2.0 | the feed tool (through minisign) |
| `gpui-pre-windows` 0.3.10, patched | Apache-2.0 | the Windows app |

Build tools, not linked: cargo-xwin (MIT or Apache-2.0); Microsoft's
CRT, SDK libraries, `fxc.exe` and `d3dcompiler_47.dll` (Microsoft's
terms; only the static CRT ends up in the exe, as in every MSVC build);
NSIS (zlib); wine (LGPL, only to run tools and tests).

## CI for the native app and the release crates (2026-11-14)

`.github/workflows/native.yml`: one Linux job that runs the native gate
exactly as `spikes/native/README.md` states it (`cargo fmt --all --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo test`, from
`spikes/native`), then the same three in `spikes/release` (updater, feed
tool, log, hello). It runs as `ci.yml` does: every push to `main` and
`backend/**`, every pull request, and by hand. Nothing in it publishes or
signs: the token is read-only and it uses no secret. It is active for a
branch once a commit holding it is pushed (the owner's step, through a
pull request to `main`; this track pushes nothing).

[Decision Log]
- Purpose: the quality audit found no CI for the native app or the
  release crates. `ci.yml` checks only the main workspace, and
  `spikes/release/ci/app-release.yml` is a release recipe that does not
  run. Each track ran the gate by hand; nothing checked a merge.
- Constraints:
  - Both are workspaces of their own, on Rust 1.98.1 (`rust-toolchain.toml`;
    the GPUI snapshot needs 1.95 or newer, the main workspace stays on 1.93).
  - GPUI links `libxcb`, `libxkbcommon` and `libxkbcommon-x11` on Linux
    (the shipped executable's `NEEDED` entries); fontconfig, freetype,
    Wayland and Vulkan are loaded at run time.
  - The app embeds its faces with `include_bytes!` and they are not
    committed (`spikes/native/.gitignore`), so the kit does not compile
    without them.
  - The native app depends on the main workspace's `vessel-client`,
    `vessel-rpc` and `vessel-proto` by path.
  - The repository is public, so standard hosted runners cost nothing.
- Alternatives:
  - A job in `ci.yml`: its matrix and its 1.93 toolchain are the main
    workspace's, and two toolchains in one job confuse its cache.
  - Windows and macOS runners: the Windows build is a cross build that
    `build.sh` makes and checks for every label (a hosted Windows build
    would check a second way of building it), and macOS is not released.
    Worth adding when the Windows-only code grows.
  - A path filter on `spikes/**`: it would miss a change in the main
    workspace's crates that breaks the app.
- Choice: one Linux job, both gates in sequence, no path filter.
  - The faces as the README says: `npm ci` in `web/` (Pretendard comes from
    the page's packages, cached by `setup-node`), then `scripts/fonts.sh`
    (Geist through `npm pack`).
  - The toolchain read from both `rust-toolchain.toml` files, failing if
    they differ.
  - `Swatinem/rust-cache` over both workspaces' `target`.
- Costs:
  - Run time, measured 2026-11-14 on this machine with 4 jobs (a hosted
    runner has 4 vCPUs) while it was busy (load about 18 on 16 cores):

    | Step | Cold (s) | Cached (s) |
    |---|---|---|
    | `npm ci` | 9 | |
    | `fonts.sh` | 1 | |
    | native fmt / clippy / test | 5 / 175 / 642 | – / 96 / 486 |
    | release fmt / clippy / test | 1 / 87 / 150 | – / 65 / 176 |
    | cargo steps in all | about 18 min | about 14 min |

    The cache saves little because the workspace's own test binaries link
    GPUI and are rebuilt on every run (`cargo test` compiles for 9.5 of
    its 10.7 cold minutes; the 610 native and 47 release tests run in
    about 12 s). With setup and the cache's upload, expect about 20-25
    minutes, then 15-18. The job's timeout is 60.
  - Disk: a cold run's targets reach about 13 GB (native) and 3.4 GB
    (release). The job first removes the image's Android, .NET, Haskell
    and CodeQL toolsets and prints the free space before and after. If it
    ever runs out, `CARGO_PROFILE_DEV_DEBUG=line-tables-only` for the job
    is the next step (smaller test binaries, the same results).
  - The cache counts against the repository's 10 GB, with `ci.yml`'s;
    GitHub evicts the oldest entries first.
- Measured: 610 native tests passed (1 ignored: the window capture, which
  needs a display) and 47 release tests passed, with no display, no
  Vulkan and a fresh target.
  - Run by `run.sh` and `warm.sh` in a scratch worktree of this branch
    (removed afterwards); the cached run first dropped what rust-cache
    drops, the local path packages.
  - The workflow itself has not run on GitHub yet; its YAML parses.

## CI: what the owner decides

`spikes/release/ci/app-release.yml` builds Windows x64 (natively, with a
10-second start on a real Windows), macOS arm64 and x64 (an ad hoc
signed `.app`, a `.dmg` and a `.app.tar.gz`, a start on arm64) and Linux
x64, then signs and attaches them to a GitHub pre-release. It does not
run: it is outside `.github/workflows/`. Before it can:

1. **Pushing.** Branches and tags must be pushed (AGENTS.md section 11
   forbids it today).
2. **Where releases and the feed are published**: GitHub releases hold
   the artifacts; the feed (`<channel>.json` and `<label>/`) needs a
   static HTTPS host whose address is compiled into the app
   (`vars.VESSEL_UPDATE_FEED`). It can be the host the coordinator
   serves the dist directory from now.
3. **The release key in CI**: the secret half as a repository secret
   (`VESSEL_APP_UPDATE_SECRET`), the public half as a variable; or keep
   signing on this machine. Losing it means a new key and a manual
   reinstall of every copy.
4. **Windows code signing**: a certificate (or Azure Trusted Signing, as
   Zed uses). Until then SmartScreen warns on first run.
5. **macOS**: an Apple Developer account for Developer ID signing and
   notarisation; without them Gatekeeper blocks a downloaded app
   (right-click > Open, or `xattr -d com.apple.quarantine`).
6. **Microsoft's terms for the Linux cross build**: cargo-xwin and
   `fetch-fxc.py` download Microsoft's files without Visual Studio's
   installer; the alternative is building Windows only on CI, or the
   `-gnu` target with mingw (no Microsoft runtime, still `fxc.exe` for
   the shaders).

## Left for the owner

The six points above, and which channels beyond `preview` exist.
