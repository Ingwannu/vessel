# The page's file follows the disk (ADR 0105, page packet)

Status: implemented on the node line after the file panels' activation
(ADR 0105). This is the page half of ADR 0105's callers; the native panel's
watch, Duplicate and Download are app-line packets. The page's files stay
on the node line so the app line's root keeps equal to a node head.

## Paseo reference (0.15.4)

- `app/src/file-panels/file/handle.ts:147-176`: a file is observed while
  its tab is on screen; coming back reports what changed meanwhile.
- `handle.ts:228-259`: a version the handle already has (its own write
  included) is no change; nothing is reported while the handle writes.
- `file-panels/source/editor/model.ts`: a clean buffer follows the disk,
  a modified one keeps its text and says "Changed on disk"; a deleted
  file keeps its copy (`getFileConflictCallout`, `canOverwrite` =
  modified).
- `file-panels/source/conflict-alert.tsx`: the alert under the toolbar
  (amber triangle and title, the description, Overwrite and Reload as
  outline buttons, an amber rule); `panel.tsx:182-196`: Reload asks
  "Reload from disk?" when the buffer is modified.

## What is built

- `web/src/data/workspace/watch.ts`: the change rule, the native watch's
  (ADR 0036): a version the page shows is no change; the first answer is
  compared with what is shown (a missing file is a change); then any state
  other than the anchor is one. Vitest cases in `watch.test.ts`.
- `data/workspace/files.ts` `observeFile`: one `file.observe` call.
- `data/workspace/watch.ts` `diskAfter`: the disk against what is shown
  (`same`, `changed`, `deleted`); a version the page shows is the same
  disk again (a file put back as shown, the page's own save).
- `features/workspace/use-disk.ts`: from the file's first read on (so the
  first answer is judged against what is shown), one observation at a
  time, anchored on the last answer, `pace.ts`'s quiet time after each
  answer. A save in flight is waited for before an answer is judged, so
  the page's own write is never a change (Paseo's "nothing while the
  handle writes"). The node's `unavailable` (its 64 observers busy) is
  asked again after the quiet time; any other failure ends the watch.
  Leaving the file ignores its call: the bridge has no cancel, so the
  node's call runs to its 30 s heartbeat, holding one observer and one of
  the origin's 8 wait shares until then.
- `features/workspace/use-file.ts`: gives the watch what is shown and the
  save in flight; `refresh` reads the file again from its start without
  blanking the view.
- `features/workspace/file-change-alert.tsx`: Paseo's conflict alert under
  the file's head: "Changed on disk" with Paseo's description and
  Overwrite while the edit holds changes, Reload always (it asks "Reload
  from disk?" / "Your local changes will be lost." when it would drop
  them); "File deleted on disk" / "The open copy is preserved." with no
  choices. Outline buttons are the kit's `outline` variant (DESIGN.md
  §6.1), Paseo's. Dialogs now layer above sheets (z 52 over 51), so the
  question also comes over the session panel's sheet in a narrow window.
- `features/workspace/file-view.tsx`: with no edit open the view follows
  the disk; an open edit keeps its text under the alert; a deleted file
  keeps its copy under the alert, which goes when the file comes back as
  shown. A save into a changed file still meets the existing two-sided
  conflict, which replaces the alert while shown.

## Differences from Paseo

- The node's observation is a finite call (ADR 0083): the page asks again,
  at most once a second.
- The page edits on Edit (Paseo's editor is always on): a clean, open
  edit is not replaced under the person; the alert offers Reload (no
  question, nothing to lose), and Cancel shows the disk too.
- A watch error other than the node's `unavailable` ends the watch
  silently (Paseo shows the error with Retry); the page then behaves as
  before, as it does on an older node, which refuses `file.observe`
  before sending.

## Validation

- `watch.test.ts` (7 cases): a heartbeat with the same state, the disk
  past the anchor, the page's own save, the first answer against what is
  shown, a deletion reported once, the disk gone, back or moved on
  (`diskAfter`), states told apart by kind, hash and size.
- `e2e/code.spec.ts` "a shown file follows the disk; an open edit keeps
  its text under Paseo's alert": a file changed elsewhere while shown is
  followed with nothing pressed; with an edit open the text stays under
  the alert with Overwrite and Reload; Reload asks first (Cancel keeps the
  edit), then shows the disk; a deleted file keeps its copy under "File
  deleted on disk", which goes when it is put back as shown. Needs the
  file panels' protocol on the fleet's node.
- The existing edit test (a save after a change on disk shows both sides)
  finds the save's conflict by its words: with the watch on, the alert
  may say "Changed on disk" before the save replaces it.
