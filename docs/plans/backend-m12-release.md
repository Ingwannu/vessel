# Milestone 12, track R: releases, installers and updates

Status: started 2026-11-13. Branch `backend/m12-release`, workspace
`spikes/release/` (its own, outside Vessel's, as every milestone 12
track's is). Answers questions 4 (Windows and macOS) and 8 (updates) of
`docs/plans/backend-m12.md`, and builds what the owner installs.

Read first: ADR 0035 (the native app), ADR 0008 (how `vesseld` releases
are signed and installed), ADR 0014 (what may be linked), and
`spikes/release/patches/README.md` (the patched GPUI crate).

## What this track owns

```
spikes/release/
  build.sh               every artifact of one label, on Linux
  hello/                 the smallest GPUI window (a target's first proof)
  updater/               vessel-app-updater: the app's updater, no GPUI
  feed/                  vessel-app-feed: release key, signed feed, sums
  patches/               gpui-pre-windows 0.3.10, patched (README there)
  tools/fxc-wine         Microsoft's fxc.exe under wine, for GPUI's shaders
  tools/fetch-fxc.py     fetches fxc.exe and d3dcompiler_47.dll from the SDK
  windows/installer.nsi  the per-user installer
  windows/README.txt, linux/README.txt   what each package says
  ci/app-release.yml     the hosted-runner workflow (not under .github: not run)
```

The app itself is the desktop track's (`spikes/native/`, merged from
`backend/m12-spike`). This track adds one section to its manifest: the
`[patch.crates-io]` line that points `gpui-pre-windows` at the patched
copy, which only a Windows build compiles.

## Running it

Once per machine (Ubuntu 24.04): `sudo apt-get install wine wine64
mingw-w64 nsis msitools` (installed here on 2026-11-13), then
`cargo install --root "$VESSEL_TOOLS_DIR" cargo-xwin` (0.23.1 here) and
`python3 -I spikes/release/tools/fetch-fxc.py "$VESSEL_TOOLS_DIR/fxc"`.
cargo-xwin fetches Microsoft's CRT and SDK libraries into
`$VESSEL_TOOLS_DIR/xwin-cache` on its first build; `fetch-fxc.py`
fetches the HLSL compiler. Both are Microsoft's files under Microsoft's
terms (see "Left for the owner").

A build:

```bash
VESSEL_DIST_DIR=<dist root> VESSEL_TOOLS_DIR=<tools> \
VESSEL_UPDATE_SECRET=<secret key file> \
spikes/release/build.sh 0.0.0.0.1 windows-x64 linux-x64
```

With `VESSEL_UPDATE_FEED` and `VESSEL_UPDATE_KEY` set, the app is built
to update itself from that feed; `--publish` then writes the channel's
signed manifest (the moment every installed copy sees the new build).

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

## The installer

NSIS (`windows/installer.nsi`, the Decision Log is at its top), built on
Linux by `makensis` 3.09. Per user (`RequestExecutionLevel user`), into
`%LOCALAPPDATA%\Programs\Vessel`, a Start menu shortcut, an
"Installed apps" entry under HKCU with an uninstaller, Korean first and
English. Zed's Inno Setup and WiX need a Windows-only compiler; MSIX
needs a signing certificate to install at all.

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

Tests (`cargo test` in `spikes/release`): 22 unit tests of the crate
(labels, channels, paths that try to leave the feed, digests, bounds, a
tampered manifest, another key, another channel, a later schema, a
duplicate, the swap, a failed swap, the clean-up), 5 end-to-end tests
through a feed served over loopback HTTP (an update installed and the
staging files gone; the same label is no update; bytes other than the
signed ones never installed and the download removed; another key's
feed refused; a missing feed retried), 6 of the feed tool (arguments,
key file modes, refusal to replace a key, publish then check, a changed
artifact caught, signed sums).

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
--no-run`) and run under wine 9.0: the 22 unit tests pass, and the five
end-to-end ones (loopback HTTP, a real download and swap) pass once the
first one expects what Windows does: the old executable is renamed to
`vessel-app.old-0.0.0.0.1` beside the new one (Windows will not delete
an executable file in use), to be removed at the next start; on Linux
the old file is simply replaced. Wine emulates Windows' file semantics
and does not lock a running executable as Windows does, so this shows
the Windows code path runs, not that Windows lets it rename the running
`Vessel.exe` (Windows does allow renaming a running image; the first
real-Windows update is the proof).

### What the app calls

Wired in by this track (coordinator, 2026-11-13) as one module,
`spikes/native/app/src/update.rs`, and two lines in `main.rs`: when the
build has a feed and a key, a one-thread tokio runtime of its own runs
the updater and a GPUI task follows its state; once a build is
installed the frame's foot says it opens at the next start. Without the
settings nothing starts. There is no "restart now" control yet (the
shell's screens are the desktop track's); `Updater::restart` is there
for it. No feed address exists yet (the dist directory is behind a login
the app cannot pass; where a public feed lives is the owner's decision),
so every build so far is built without one and its README says it does
not update itself.

The lines, for reference:

```rust
let updater = match vessel_app_updater::settings_from_build()? {
    Some(settings) => {
        let installation = vessel_app_updater::Installation::from_process(
            settings.target.default_packaging(),
        )?;
        Some(vessel_app_updater::Updater::start(settings, installation, runtime_handle)?)
    }
    None => None, // a development build
};
// UI: show updater.state() (a watch); on UpdateState::Ready, a
// "Restart to update" control calls
// updater.restart(std::env::args_os().skip(1)) and then cx.quit().
```

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
