# ADR 0105: File panels activation and their callers

Status: proposed. The node part (the protocol number for ADR 0083's five
methods) was drafted as 26 and took the next free number in merge order at
the node-line merge: 27 (account usage reports took 26 just before). The native and page callers follow as separate packets.

[Decision Log]
- Purpose and intent: turn on ADR 0083's reviewed file-panel methods
  (`file.inspect`, `file.read_bytes`, `file.duplicate`, `file.observe`,
  `file.suggest`) and give them the callers Paseo 0.15.4 has: a file shown
  in a panel follows the disk while it is on screen
  (`app/src/file-panels/file/handle.ts:147-176,228-259`), and the Explorer
  offers Duplicate and Download (`components/file-actions-menu.tsx:187-222`,
  `hooks/use-file-explorer-actions.ts:267-282`).
- Existing implementation and constraints: the node handlers, their
  admission (four work slots, 64 observers, a 60 s deadline, no long poll
  for a delegated call) and the client SDK (70 s deadlines, the hash-checked
  page fetch into a `ChunkSink`) landed with `FILE_PANELS_PROTOCOL = None`,
  so both ends refuse them. The native Files tab already re-reads the shown
  file through the editor model's `reconcile` (a clean buffer follows the
  disk, a modified one meets the change as a conflict, a missing file is the
  Deleted conflict) and its menu leaves out Duplicate and Download.
- Alternatives considered: (a) one number per method, activating each with
  its first caller: five numbers for one reviewed feature, and peers that
  speak half of it; (b) wait for the paused W13 app branch (an owner rework
  over 17k lines, not reviewed as a unit) before any number: the reviewed
  node work stays unreachable; (c) a polling timer in the app instead of
  `file.observe`: rejected in ADR 0083.
- Choice: one number for all five, assigned at the node-line merge. Every
  caller checks the machine's protocol: on an older or unassigned peer it
  keeps today's behaviour (no watch; no Duplicate or Download item, as
  Paseo hides Duplicate without `fsEntryDuplicateEnabled`). The native
  watch is one finite observation at a time for the file the Files tab
  shows, anchored on the last answer, at most one request a second; a
  changed revision that is not the editor's own saved content, or Missing,
  runs the existing shown-file re-read; while the editor's write is in
  flight the re-read waits for it (Paseo ignores observations while
  writing). Leaving the file or the root drops the observation. Download
  asks where to save (Paseo's desktop download opens the save dialog),
  fetches with the SDK into a sink that writes a temporary file beside the
  target, checks the whole hash, then renames it into place; a failure
  leaves nothing.
- Why: each of the five has a complete, bounded node effect and a reviewed
  admission; one number keeps the feature whole on every peer. Gating by
  protocol keeps old nodes working without a refusal shown to the person.
- Advantages, costs and effects: PROTOCOL_VERSION moves to the assigned
  number; MIN stays 2 and the store schema does not change. An observation
  holds one node observer slot and one wait share per shown file; a
  watch starts once the file's first read has landed. Leaving the file
  ignores its call (the bridge has no cancel), which holds both until the
  node's 30 s heartbeat. The node's `unavailable` (observers busy) is
  asked again after the quiet time; any other watch error stops the watch
  until the file is opened again (the native watch takes the same rules
  on the app line). The page watch and Paseo's conflict alert are
  `docs/plans/impl-files-page.md`; the page's file action menu, the
  Modified sort (`file.inspect`) and directory suggestions
  (`file.suggest`) in Add project are later work with no caller in this
  ADR's packets.
