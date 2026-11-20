# ADR 0123: Open in editor and reveal in the file manager

Status: proposed (impl F, 2026-11-20; TRACKER 467). The coordinator
recorded 0123 as this ADR's number, ruled on the processes it starts and on
the marks, and put the forge link ("Open on GitHub") in a later step B.

## Context

Paseo's desktop app opens a workspace, a folder or the file shown in the
person's editor, and reveals entries in the file manager (pinned 0.15.4):

- the desktop side lists 25 targets (`desktop/src/features/editor-targets/`):
  Cursor, Trae, Kiro, VS Code, VS Code Insiders, VSCodium, Zed, Antigravity,
  Android Studio, Xcode, ten JetBrains IDEs, WebStorm, then Finder,
  Explorer or the desktop's file manager ("Files"). A target is installed
  when its command is on PATH or at an install path, or (macOS) its app
  bundle exists. Xcode is listed only for a workspace holding a document it
  opens;
- opening expands `~`, requires absolute paths that exist, and starts the
  command detached with no standard streams and without the runtime's
  control variables; a macOS app without a command goes through
  `/usr/bin/open -a`; the file manager is Electron's `showItemInFolder` and
  `openPath`;
- the app side shows the targets only with the desktop bridge and a local
  daemon: the workspace header's split button (Open, and "Choose editor"
  with a check on the preferred target; choosing one only makes it the
  preferred), the Files menu's "Open in <editor>" (folders) and "Reveal in
  <file manager>", and the sidebar's "Open in file manager". The
  preferred target is kept per (server, project).

Vessel's native app had none of it (the Files menu left them out by design).
Since TRACKER 816 the link knows which nodes run on this computer
(`ThisComputer`).

## Decision

1. Only for a machine whose node runs on this computer, and only for paths
   that exist here. A session's folder (its card's `cwd`) is the root.
2. The view holds Paseo's rules without I/O (`view/src/open_targets/`):
   the 25 targets with Paseo's ids, labels, kinds, command names, install
   paths, macOS app names and argument shapes; the listing; the Xcode
   document search; the path guard; the launch plan. The app hands it a
   probe of this computer.
3. The guard (the coordinator's conditions): every path is absolute,
   canonicalised (links resolved), exists, and is the workspace root or
   under it, so no argument made from it can begin with `-`; a link out of
   the workspace is refused. Windows' `\\?\C:\` prefix is dropped from
   canonical drive paths.
4. Listing and launching run on the background executor. A workspace's
   list is kept for 60 s (Paseo's `staleTime`) and asked again when shown
   after that. A failure is a toast.
5. Places: the session header's split button before the code panel's
   toggle (Paseo's header order), icon only as Paseo mounts it; the Files
   menu's "Open in <preferred editor>" for folders and "Reveal in <file
   manager>" for every entry, in Paseo's groups; the sidebar session menu's
   "Open in file manager" between Labels and Archive.
6. The preferred target is kept in the native preferences document per
   project (`preferred_editors`, Paseo's target ids, tidied with the other
   per-project entries when projects or machines change).
7. Marks (the coordinator's ruling (a)): vector marks for the targets
   Paseo gives a PNG, from openly licensed or vendor sources, never from
   Paseo's PNGs; Paseo's symbols for the rest. Six have marks (Cursor,
   Zed, Android Studio, Xcode and WebStorm from Simple Icons 16.38.0,
   CC0-1.0; VS Code, Microsoft's own icon unchanged). Antigravity and
   Finder keep the symbols. Each editor has one mark for both schemes,
   as Paseo ships one icon (the coordinator's ruling on the frames): Zed,
   Cursor and Android Studio on an app tile as Paseo draws them, VS Code
   bare (Microsoft allows no background). Sources, hashes and terms:
   `docs/design/editor-marks/README.md`. They are drawn through the kit's
   mark raster path (sharp at any scale factor).

## External processes

ADR 0078 (the in-app browser) covers browser guests that Vessel runs. This
ADR covers a local app the person chose and installed. The coordinator
approved these, with the conditions above:

1. **An editor's command-line program**: the command of the chosen target,
   resolved once per launch from PATH (Windows: `.exe .cmd .bat .com`) or
   the target's install paths. Started with `std::process::Command` as
   argv, never through a shell line; stdin, stdout and stderr null; its own
   process group on Unix (`process_group(0)`); the app's environment
   without the `VESSEL_APP_*` variables that steer Vessel. A named thread
   waits on it so it is reaped; the UI never waits. The resolved path is
   logged, the environment never. On Windows a `.cmd` or `.bat` goes
   through Rust std's batch handling (cmd.exe with std's quoting, which
   refuses an argument it cannot quote safely; that refusal is the toast).
   Allowed because the person chose the program and the paths, both are
   checked, and nothing of a node's or an agent's text becomes an argument.
2. **macOS `open -a`**: `/usr/bin/open -a <app bundle> <paths>` for an
   editor present only as an app bundle (and Xcode), as Paseo. Same rules.
3. **The file manager**: GPUI's `reveal_path` (Linux: the
   xdg-desktop-portal `OpenDirectory` over D-Bus, falling back to the
   `open` crate's launchers; macOS `NSWorkspace selectFile`; Windows
   `SHOpenFolderAndSelectItems`) and `open_with_system` (Linux `xdg-open`,
   macOS `NSWorkspace`, Windows `ShellExecute`). No process code of ours;
   the same calls `about.rs` already makes.

Everything goes through one seam (`open_targets/launcher.rs`). Tests use a
recording launcher and a PATH of empty stub files that are never run; the
test build has no real launcher at all. Captures use a stub script on a
scratch PATH that only writes its arguments to a file.

## Differences from Paseo

- Paseo's sidebar item shows for any host's path on the desktop; Vessel's
  only for a machine on this computer.
- Paseo reads an older device-wide preferred editor as a fallback; Vessel
  never had one.
- Antigravity and Finder are drawn with Paseo's symbols, not their app
  icons; the other marks are vectors, not Paseo's PNGs.
- The editor list opens below its caret at the caret's left edge (the
  kit's list); Paseo aligns its menu's right edge with the caret.
- Paseo shows the desktop's English errors; Vessel says them in the
  person's language.
- The forge link ("Open on GitHub") is step B, a later packet: it needs
  the remote's web address in git status, a node protocol change.

## Consequences

- The native app starts programs on this computer, only on the person's
  press, only through the seam above.
- The page gets nothing in this step (a browser has no machine on this
  computer); step B gives it the forge link.
- Tests: the targets, ids and order; listing per platform; Xcode's
  document; each family's arguments against Paseo's; the guard (relative,
  missing, a link out of the root, Windows' prefix); the preferred target;
  the words; the app's listing, launch, reveal, refusal toasts and the
  machine-elsewhere case with the recording launcher; the marks' geometry
  and colours.
