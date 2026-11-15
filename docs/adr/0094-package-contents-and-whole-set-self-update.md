# ADR 0094: The app's package carries the command line and the node, and updates them as one set

Status: accepted by the coordinator, 2026-11-15 (milestone 12, track R);
the `bin` layout below agreed the same day with W15 and W19, whose shared
tool-path getter (`link::desktop::bundle`) reads it.
Built behind `VESSEL_PACKAGE_NODE=1` in `spikes/release/build.sh`, off by
default. The first build after the app's local-node controller (track W15)
lands ships it; no build before that does. No wire change, no change to the
release key, the feed's address or its token.

## Purpose and intent

Paseo's desktop app runs a local daemon it manages and ships its command
line beside it. The native app needs the same: W15's local-node controller
starts `vesseld` beside the app, and W19's shell hands `vessel` to a
terminal. Until now the packages carried the app alone (`Vessel.exe`, or
`vessel-app` on Linux, with texts and icons). The three programs come from
one commit and speak one protocol range. An update has to keep them
together: a new app beside an old node is a pair nobody tested.

## Existing implementation and constraints

- The updater (`spikes/release/updater`) reads one signed manifest per
  channel (`preview.json`: the manifest's text and its minisign signature).
  The manifest names one artifact per target and packaging (path, size,
  SHA-256). The updater downloads that artifact beside the app, verifies
  it while streaming, and swaps it in. Unix uses `rename`. Windows renames
  the running file aside first, because a running `.exe` can be renamed
  but not replaced. Then it restarts on request.
- Every app published so far, 0.0.0.0.3 to 0.0.0.0.18, reads schema 1. A
  manifest of a later schema makes those apps say "download the new build
  by hand". No manifest type has ever denied unknown fields (`git log -S
  deny_unknown_fields -- spikes/release` is empty), so serde's default
  applies: a field an app does not know is skipped.
- Coordinator's conditions: one feed entry with each file's hash;
  everything verified before any swap; rollback on failure; published apps
  keep updating, either because the old format stays readable or through
  a bridge release; the feed's address and token never change.
- Windows file names ignore case, so `vessel.exe` and `Vessel.exe` cannot
  sit in one folder. The command line must be reachable by the name
  `vessel` without the app itself landing on `PATH`. The managed node may
  be running while the app updates.

## Alternatives considered

1. **A new manifest schema (2) listing a set.** It is clean, but every
   published app refuses it. A bridge release would have to be published
   first and installed everywhere before the first set: two publishes and
   a window in which some copies are stranded.
2. **Separate artifacts per program**, each a target and packaging of its
   own. The app would pick its own artifact and ignore the others, but
   nothing ties the three to one swap. The manifest's "one per target and
   packaging" rule would also need new packaging kinds that old apps
   refuse to parse.
3. **An archive (zip or tar) as the update, unpacked over the folder.**
   Old apps would read the archive's bytes as the new executable. A crash
   half way through leaves a mix, with nothing to go back to.
4. **A directory per build and a pointer to the current one.** It needs a
   launcher, or a link that the installer, the Start menu and the portable
   zip all follow; that is a second program on Windows.

## Choice

- **Manifest:** stays at schema 1. Each artifact gets an optional
  `companions` list: `role` (`cli`, `node`), `path`, `size` and `sha256`.
  The list is left out of the text when it is empty, so manifests without
  companions are byte for byte the old format.
  - Where a companion is installed comes from the role and the system,
    never from the feed: a `bin` folder beside the app's executable
    (`COMPANION_DIR`) holding `vessel` and `vesseld` on Linux, `vessel.exe`
    and `vesseld.exe` on Windows (`Role::file_name`). The folder keeps the
    command line's own name on Windows, and putting only that folder on
    `PATH` gives `vessel` and `vesseld` without the app. W15 proposed the
    folder; the first draft of this ADR put `vessel-cli.exe` beside the
    app, and was changed before anything shipped. macOS, when a bundle is
    built: `Contents/Helpers` or `Contents/MacOS`, not `Contents/Resources`
    (code there breaks signing).
  - A role this build does not know parses (`Role::Other`), so the
    manifest still opens, and is left out of the set.
  - Bounds: at most 8 companions per artifact, and a role listed twice is
    refused.
- **Update:** the app first downloads and verifies every file of the set
  (the app and its known companions), each to its own staging name. When
  one file fails, the files already staged are removed and nothing on disk
  changes.
- **Swap** (`Installation::replace_set`): each file is put in place in
  turn, the companions first and the app last, and what it replaced is
  kept aside under `<name>.old-<running label>`.
  - On Unix the aside is a hard link made before the rename, so the path
    is never missing. On Windows it is the rename aside, as for the app.
  - When one file fails, every file already placed is put back in reverse
    order: the aside is renamed over the new file, or a file the set added
    is removed.
  - The `bin` folder is made when a copy has none (one installed before
    the set).
  - After a whole set, Unix removes the asides. Windows leaves them for the
    next start's clean-up, which now looks in `bin` for the companions'
    asides too.
- **Bridge, built into the new format itself:** a published app that
  updates to a set label replaces its executable only, because it ignores
  `companions`. The build it installs then runs, finds the feed's label
  equal to its own, and puts its missing companions in place. A companion
  counts as missing when no file of its name and size is there. The app
  itself is not touched, so nothing waits for a restart.
- **Packages** (`build.sh` with `VESSEL_PACKAGE_NODE=1`):
  - `vessel-cli` and `vessel-node` are built from the repository's own
    workspace and release profile (fat LTO, symbols kept), with the page
    the node serves built in (`VESSEL_WEB_REQUIRE_BUNDLE=1`).
  - Windows uses `cargo-xwin` with a static CRT, as console programs
    without the app's GUI subsystem or resources. Their imports go through
    the same list of DLLs that Windows ships.
  - The files go in `bin` in the zip, the installer (NSIS
    `-DCOMPANIONS=1`; the uninstaller always removes them and the folder)
    and the tarball. Each is also in the dist folder alone
    (`vessel-LABEL-<target>-vessel[.exe]`, `-vesseld[.exe]`), as the
    companions' artifacts. `--publish` and `vessel-app-feed publish
    --companion TARGET:PACKAGING:ROLE:PATH` add them to their artifact.

## Why this over the alternatives

It keeps every published app updating: they read the new manifests as
they read the old ones. It needs no bridge publish, and the first set
label repairs itself on the copies that had to take it the old way. One
swap path serves the app alone and the set. It needs no helper process
or launcher, and after any single failure the folder holds the old set or
the new one.

## Advantages, drawbacks and impact

- **Download size:** each set update downloads the app and both companions
  every time. From the `a365d295` node staging on Linux, `vessel` is
  64.9 MB and `vesseld` 49.4 MB, page included, beside the app's 87 MB. The
  Windows sizes are not measured yet, and the archives and installer grow
  accordingly. Delta updates are not built.
- **A crash between two renames** of the set leaves a mix until the next
  check. The repair puts back the companions of the running label. A
  newer app beside an older node for that time is the case the protocol
  range already covers.
- **The repair checks names and sizes, not hashes:** the check runs
  hourly, and hashing 115 MB each time is not worth it. A hand-replaced
  file of the same size is not detected. A companion the user deleted
  comes back at the next check, because it is part of the app.
- **A running node is replaced on disk, not restarted.** Linux keeps the
  old inode running; Windows renames it aside. The local-node controller
  (W15) stops the node it owns before the app's update restart and
  starts the bundled one after it. To notice a binary replaced under a
  running node (a repair), it compares the file's identity (SHA-256, or
  size and modification time) with the one it spawned. The node's own
  version (`CARGO_PKG_VERSION`, 0.1.0) is not the app's label, and this
  ADR does not compile the label into the node: that would be a node-line
  change. A node the app only attaches to is never restarted by it;
  compatibility there is the handshake's protocol range. The uninstaller
  cannot remove a node that is still running; the controller should stop
  it first.
- **Platforms:**
  - AppImage and macOS bundles carry no companions. The updater refuses
    companions beside them, and the build makes neither yet.
  - Windows build, checked 2026-11-15 with `build.sh`'s own command
    (`cargo-xwin`, static CRT, page included):
    - `vessel.exe` 74,353,152 bytes and `vesseld.exe` 56,924,160 bytes,
      both console programs (Windows CUI).
    - Every DLL both import is on the list of DLLs Windows ships.
    - It needed the Windows target in the repository's toolchain (1.93),
      which `build.sh` now adds.
    - It took about 2 hours on the loaded build machine.
    - Still not shown: that they run on Windows. The workspace gates run
      on Linux, and this track has not run the node's Windows code.
- **Verification:** `vessel-app-updater` tests cover the published types'
  reading of a manifest with companions (a frozen copy of the schema-1
  types), roles and bounds, the set swap, rollback after a failure half
  way, the `bin` folder made for a copy without one, the AppImage refusal,
  the companions' leftovers, and end to end: a newer set into a copy with
  no `bin`, a companion that does not verify (nothing changes), and the
  repair. The installer with `-DCOMPANIONS=1` was compiled, installed and
  uninstalled silently under wine (dummy programs): `bin\vessel.exe` and
  `bin\vesseld.exe` beside `Vessel.exe`, all removed with the folder. `vessel-app-feed` tests cover `--companion` and check.

## Addendum, 2026-11-15: resuming, the compressed copy, and a visible stop

Approved by the coordinator the same day, after the owner's Windows copy
of 0.0.0.0.14 took 0.0.0.0.19 over a slow line.
- The preview proxy's log shows the 51.7 MB executable starting at
  08:15:42 UTC and breaking off at 31.9 MB at 08:17:39, about 0.27 MB/s.
- No retry reached the feed afterwards.
- The same file came in 0.87 s from the build machine through the same
  public path, so the line to the owner was the limit.
- Every retry would have begun again at zero.

The decision log is in `spikes/release/updater/src/stage.rs`. In short:

- **Resuming.**
  - Each staging file is named from the label and the digest of the bytes
    it will hold, no longer from the process id. A later attempt, in this
    run or after a restart, finds what an earlier one kept.
  - The kept bytes are hashed again, and the rest is asked for with
    `Range: bytes=<kept>-`.
  - A server that answers the whole file (200) starts the file over. One
    that answers 416, or another range, is asked for the whole file.
  - A download whose line failed keeps its file. One whose bytes are
    wrong (size or digest) loses it.
  - The file being written is locked (`File::try_lock`).
  - At start the app no longer removes staging files. Each manifest read
    removes the ones the current set does not want (`prune_staging`).
  - The feed side: since 09:08 UTC the preview proxy answers a single
    byte range on `/feed`. Its README and its `feed_range` decision log
    say how.
- **The compressed copy.**
  - `Artifact` and `Companion` gain an optional `compressed`
    (`encoding`, `path`, `size`, `sha256` of the compressed bytes). It is
    left out of the text when absent, so earlier apps read every manifest
    as before and download the plain file.
  - The newer updater downloads the gzip copy (resumably). It checks the
    copy's digest before unpacking, and the unpacked file against the
    file's own digest.
  - An unknown `encoding` (a later build's zstd, say) falls back to the
    plain file.
  - gzip was chosen because `flate2` is already in the app's tree, so it
    adds no crate. zstd would be about a quarter smaller but adds a
    decoder.
  - Measured on 0.0.0.0.20's update files with `gzip -9 -n`: Windows
    51,643,904 → 20,415,014 bytes (39.5%), Linux 88,754,616 →
    30,501,649 (34.4%). Compressing takes about a minute each on the
    build machine.
  - `build.sh` writes the copies beside every update file.
    `vessel-app-feed publish --compressed gzip` offers them, after
    proving each one unpacks to the signed file; `check` proves it again.
- **A visible stop.**
  - `UpdateState::Failed` gains `retry_in`, the wait before the updater
    tries by itself (`None` with automatic checks off).
  - It also gains `kept`, the bytes a broken-off download kept, with the
    set's size as it travels.
  - The app compiles with them and still shows its "try again" line.
    Wording them ("62% kept, trying again in a minute") is W15's update
    screen.
- **What it does not change.**
  - Apps published before it get no resuming and no compressed copy.
    They send no range and read no `compressed`; their path stays the
    `/dl/` installer when the line is slow.
  - The feed's address, token, key and schema are unchanged.
  - Between attempts, up to one set's files stay beside the app.
- **The command line on PATH (agreed with W19, 2026-11-15).** The app's
  "install the command line" adds exactly `<app>\bin` to the user's PATH
  (HKCU `Environment\Path`, REG_EXPAND_SZ); there is no `vessel.cmd`
  trampoline (cmd.exe reads batch files in the OEM code page, so a
  non-ASCII path in one is misread).
  - The uninstaller (`un.RemoveBinFromPath`) takes exactly that entry out
    again, ignoring case as Windows does. It keeps every other entry
    unexpanded and writes REG_EXPAND_SZ, deleting the value if nothing is
    left, then broadcasts `WM_SETTINGCHANGE "Environment"`.
  - It leaves a PATH of 1023 characters or more untouched. NSIS strings
    stop there (`NSIS_MAX_STRLEN` 1024 in the build's makensis), and
    writing back a cut value would lose the rest. An entry for a folder
    that is gone does nothing.
  - Under wine (scratch prefix, `test-path.sh`):
    - `C:\a;<bin>;%USERPROFILE%\b` became `C:\a;%USERPROFILE%\b`
      (REG_EXPAND_SZ).
    - `<bin>` alone removed the value.
    - A 1,479-character PATH stayed as it was.
    - A PATH without the entry was untouched.
  - Not shown on real Windows.
