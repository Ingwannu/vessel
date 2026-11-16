# File panels callers (ADR 0105)

Status: A1 (live watch of the shown file), A2 (Explorer Duplicate) and A3
(Explorer Download) implemented on package base 14.9. The node packet
assigns ADR 0083's five methods protocol 27 (assigned at its node-line
merge, node head d077aa94); until that number reaches this line every caller
here keeps its earlier behaviour (`view::workspace::file_panels::offered` is
false while `FILE_PANELS_PROTOCOL` is None or the machine is older). The page's
watch of the shown file is a node-line packet (`web/` stays equal to a
node head here): `docs/plans/impl-files-page.md` on the node line.

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
  change. Nothing is asked before a read of the file has landed, so the
  first answer is judged against what is shown. After each answer the rest
  of `pace::QUIET` (1 s) is waited out. The node's `unavailable` (its 64
  observers busy) is asked again after the pause from the same anchor; any
  other failure stops the watch until the file is opened again. The link
  has no cancel: an observation of a file left is ignored and holds a node
  observer and a wait share until the node's 30 s heartbeat.
- `app::areas::workspace::code::watch`: asks while the panel is out on Files
  and the machine speaks the file panels protocol (after the shown file's
  read answers, when the panel opens, when the tab turns to Files, after
  each pause); a change runs `reread_shown_file`, the same re-read a git
  write runs (extracted from `reread_files` with no change in behaviour).
- `link::workspace`: `FileAsk::Observe` and `Answer::Observed` over the
  client SDK's `observe_workspace_file` (its 70 s deadline maps to
  `Problem::Gone`).

## A2: Explorer Duplicate

- Paseo: `components/file-actions-menu.tsx:214-222` puts Duplicate in the
  manage group after Rename, only when `fsEntryDuplicateEnabled`;
  `hooks/use-file-explorer-actions.ts:267-282` lists the parent again after
  a success; `components/file-explorer-pane.tsx:786-803` selects the copy or
  shows `duplicateFailed`.
- Built: `Action::Duplicate` in group 3 after Rename, in the menu only when
  the root's machine has the file panels' calls; `FileAsk::Duplicate` over
  the SDK's `duplicate_workspace_entry`; on success the copy's folder is
  listed again; a refusal shows the machine's reason under the existing
  "The machine did not make this change" note, as rename and delete do.
  Words: Duplicate / 복제 (Paseo's `fileActions.duplicate`).
- Difference: this tree has no selected row, so the copy is not selected.

## A3: Explorer Download

- Paseo: `components/file-actions-menu.tsx:187-196` puts Download in the
  reference group (after Copy relative path and Reveal, before Add to
  chat) for an existing file; `stores/download-store.ts:356-376` hands the
  bytes to the browser's download (Electron asks where to save);
  `components/download-toast.tsx:20-21` says `downloadComplete`, or the
  call's message, else `downloadFailed`.
- Built: `Action::Download` for a file, in the menu only when the root's
  machine has the file panels' calls; the platform save dialog starts in
  the Downloads folder with the file's name; `FileAsk::Download` runs the
  client SDK's `fetch_workspace_file` (pages of `file.read_bytes`, each
  checked against the first page's version) into
  `link::workspace::download::Output`: a hidden partial file in the chosen
  folder, hashed while written, renamed over the chosen path only when its
  SHA-256 is the version's, removed otherwise (an error, a changed file, a
  dropped transfer). A toast says Download complete / 다운로드 완료, the
  machine's reason, a changed file, or Download failed / 다운로드 실패 with
  the system's reason (also when the platform cannot show its save dialog,
  as under a bare X server with no desktop portal).
- Dependency: `sha2 0.11` for the link, already in the native lock through
  the client's crates (no new package; the root workspace's version). About
  lists it (MIT OR Apache-2.0) with the role "Checking downloaded files" /
  "다운로드한 파일 확인".
- Differences: no progress toast while the pages arrive; a partial file
  needs room beside the chosen one.

## Differences from Paseo

- The node's observation is a finite call, not a pushed stream (ADR 0083):
  the loop asks again, at most once a second.
- A missing file is not confirmed by a separate read first: the re-read the
  change triggers is that read (the editor model turns its NotFound into the
  Deleted conflict).
- Paseo observes every file tab on screen; this panel shows one file at a
  time, so there is one observation.
- A watch error other than the node's `unavailable` shows nothing and
  stops the watch (Paseo shows the error with Retry); the panel then
  behaves as it did before the watch existed.

## Validation

- View unit tests (`watch_tests.rs`): protocol gate, one observation at a
  time and the pause, quiet heartbeat versus change, the editor's own save,
  the first answer against the shown content, deletion once, stale answers,
  nothing asked before a read landed, busy observers asked again from the
  same anchor, every other failure stopping the watch.
- App GPUI test over the framed test link: a changed revision for the
  shown, clean editor sends `file.read` for that file.
- Menu order tests: Download before Add to chat in the reference group,
  Duplicate after Rename in the manage group, both only with the calls.
- Link sink tests: the whole version replaces an old file at the chosen
  path; other bytes or an unfinished download leave nothing; a partial name
  in use takes the next; a hash refusal is an inconsistent download.
- Live observation against a node at the assigned number is owed to the
  captures after the node packet merges.
