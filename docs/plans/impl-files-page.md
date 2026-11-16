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
  a modified one keeps its text and says "Changed on disk".

## What is built

- `web/src/data/workspace/watch.ts`: the change rule, the native watch's
  (ADR 0036): a version the page shows is no change; the first answer is
  compared with what is shown (a missing file is a change); then any state
  other than the anchor is one. Vitest cases in `watch.test.ts`.
- `data/workspace/files.ts` `observeFile`: one `file.observe` call.
- `features/workspace/use-disk.ts`: while a file is shown, one
  observation at a time, anchored on the last answer, `pace.ts`'s quiet
  time after each answer, cancelled when the file is left; `disk` turns
  `changed` when the disk moved past what is shown. A save in flight is
  waited for before an answer is judged, so the page's own write is never
  a change (Paseo's "nothing while the handle writes").
- `features/workspace/use-file.ts`: gives the watch what is shown and the
  save in flight; `refresh` reads the file again from its start without
  blanking the view.
- `features/workspace/file-view.tsx`: with no edit open the view follows
  the disk; an open edit keeps its text under a "Changed on disk" banner
  (status, not alert), and its save then meets the change as the existing
  conflict (both sides, overwrite nothing).

## Differences from Paseo

- The node's observation is a finite call (ADR 0083): the page asks again,
  at most once a second.
- The page edits on Edit (Paseo's editor is always on), so a clean draft
  is not replaced under the person: the banner says the disk moved and
  Cancel shows it.
- A watch error ends the watch silently (Paseo shows the error with
  Retry); the page then behaves as before, as it does on an older node,
  which refuses `file.observe` before sending.

## Validation

- `watch.test.ts` (6 cases): a heartbeat with the same state, the disk
  past the anchor, the page's own save, the first answer against what is
  shown, a deletion reported once, states told apart by kind, hash and
  size.
- `e2e/code.spec.ts` "a shown file follows the disk, and an open edit
  keeps its text under Changed on disk": a file changed elsewhere while
  shown is followed with nothing pressed; with an edit open the text stays
  and the banner shows; Cancel shows the disk. Needs the file panels'
  protocol on the fleet's node (it lands after ADR 0105's activation).
- The existing edit test (a save after a change on disk shows both sides)
  is unchanged and still passes with the watch on.
