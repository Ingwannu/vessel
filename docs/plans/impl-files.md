# File panels callers (ADR 0105)

Status: A1 (live watch of the shown file) implemented on package base 14.9;
A2 (Explorer Duplicate) and A3 (Explorer Download) follow. The node packet
assigns ADR 0083's five methods one protocol number (drafted as 26) at its
node-line merge; until that number reaches this line every caller here keeps
its earlier behaviour (`view::workspace::watch::offered` is false while
`FILE_PANELS_PROTOCOL` is None or the machine is older).

## Paseo reference (0.15.4)

- `app/src/file-panels/file/handle.ts:147-176`: a file is observed while its
  tab is on screen; coming back reports what changed meanwhile.
- `handle.ts:228-259`: a version the handle already has (its own write
  included) is no change; a missing file is confirmed by a read before it is
  reported deleted; nothing is reported while the handle writes.
- `file-panels/file/daemon-session.ts:23-56`: `fs.file.subscribe` gives the
  initial version, then `fs.file.update` pushes each new one.
- `file-panels/source/editor/model.ts`: a clean buffer follows the disk, a
  modified one enters the conflict (already ported as `view::workspace::editor`).

## A1: what is built

- `view::workspace::watch`: the decision. One observation out at a time,
  anchored on the last answer (`after`); a Ready revision whose hash is the
  content shown (the editor's version, else the last read-only read) is no
  change; on the first answer a revision other than the shown content, or
  Missing, is a change; after that any state other than the anchor is a
  change. After each answer the rest of `pace::QUIET` (1 s) is waited out.
  A failed observation stops the watch until the file is opened again.
- `app::areas::workspace::code::watch`: asks while the panel is out on Files
  and the machine speaks the file panels protocol (after the shown file's
  read answers, when the panel opens, when the tab turns to Files, after
  each pause); a change runs `reread_shown_file`, the same re-read a git
  write runs (extracted from `reread_files` with no change in behaviour).
- `link::workspace`: `FileAsk::Observe` and `Answer::Observed` over the
  client SDK's `observe_workspace_file` (its 70 s deadline maps to
  `Problem::Gone`).

## Differences from Paseo

- The node's observation is a finite call, not a pushed stream (ADR 0083):
  the loop asks again, at most once a second.
- A missing file is not confirmed by a separate read first: the re-read the
  change triggers is that read (the editor model turns its NotFound into the
  Deleted conflict).
- Paseo observes every file tab on screen; this panel shows one file at a
  time, so there is one observation.
- A watch error shows nothing and stops the watch (Paseo shows the error
  with Retry); the panel then behaves as it did before the watch existed.

## Validation

- View unit tests (`watch_tests.rs`): protocol gate, one observation at a
  time and the pause, quiet heartbeat versus change, the editor's own save,
  the first answer against the shown content, deletion once, stale answers,
  every failure stopping the watch.
- App GPUI test over the framed test link: a changed revision for the
  shown, clean editor sends `file.read` for that file.
- Live observation against a node at the assigned number is owed to the
  captures after the node packet merges.
