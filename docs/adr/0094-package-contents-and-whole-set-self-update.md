# ADR 0094: The app's package carries the command line and the node, and updates them as one set

Status: accepted by the coordinator, 2026-11-15 (milestone 12, track R).
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
- Windows file names ignore case, so `vessel.exe` and `Vessel.exe` are one
  name. The managed node may be running while the app updates.

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
  - The installed file name comes from the role and the system, never from
    the feed: `vessel` and `vesseld` on Linux, `vessel-cli.exe` and
    `vesseld.exe` on Windows (`Role::file_name`).
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
  - After a whole set, Unix removes the asides. Windows leaves them for the
    next start's clean-up, which now knows the companions' names too.
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
  - The files go beside the app in the zip, the installer (NSIS
    `-DCOMPANIONS=1`; the uninstaller always removes them) and the
    tarball. Each is also in the dist folder alone
    (`vessel-LABEL-<target>-vessel[-cli.exe]`, `-vesseld[.exe]`), as the
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
  (W15) must restart its node after the app's restart, or when the
  node's version differs from the app's label. The uninstaller cannot
  remove a node that is still running; the controller should stop it
  first.
- **Platforms:**
  - AppImage and macOS bundles carry no companions. The updater refuses
    companions beside them, and the build makes neither yet.
  - Whether `vessel-node` and `vessel-cli` build and run on Windows is
    shown only by the first build with the flag on. The workspace gates
    run on Linux, and this track has not run the node's Windows code.
- **Verification:** `vessel-app-updater` tests cover the published types'
  reading of a manifest with companions (a frozen copy of the schema-1
  types), roles and bounds, the set swap, rollback after a failure half
  way, the AppImage refusal, the companions' leftovers, and end to end: a
  newer set, a companion that does not verify (nothing changes), and the
  repair. `vessel-app-feed` tests cover `--companion` and check.
