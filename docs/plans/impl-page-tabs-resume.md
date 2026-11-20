# Page workspace tabs resume

TRACKER 817 lives on `backend/m12-page-tabs`. The implementation runs from
`55aa80f06` through `08624f6c9`. ADR 0118 explains workspace identity,
route selection, file editor lifetime, and per-item bulk close.

The first exact web gate, `impl-page-tabs-20261120T035230Z-30e4e9cd`,
passed all nine steps, with 699 unit tests and 107 browser tests. Export
`impl-page-tabs-20261120T040222Z-30e4e9cd` yielded 80 indexed frames across
1280 and 390, dark and light, English and Korean.

The coordinator returned the first frames. The strip, terminal, diff,
and compact switcher matched. The follow-up keeps the session title,
project, and actions above the tabs, uses the requested menu icons and
copy, shows the registered launcher shortcuts, and fits compact close
sheets to their content. Bulk confirmations carry the question mark and
the browser suite checks a live terminal count and warning.

The follow-up web gate `impl-page-tabs-20261120T041717Z-3bd1e64c`
passed all nine steps for `3bd1e64cf`, with 699 unit tests and 107
browser tests. Capture export `impl-page-tabs-20261120T044449Z-3bd1e64c`
provided 80 verified frames. The coordinator gave R2 visual PASS and
noted that long header titles truncate early at 390.

The node reviewer returned three required lifecycle fixes. `c5e51a863`
counts live terminals beside each session before archive, picks a live
anchor for new worktree terminals, removes missing session tabs, and
reveals a closed tab when the user re-enters its session address. The
coordinator's R2 visual approval stands. The follow-up gate
`impl-page-tabs-20261120T052151Z-c5e51a86` passed all nine steps, with 700
unit and 107 browser tests. The logs are in
`/tmp/work/impl-page-tabs/gate-r11`. Node reviewer `6fcba8b5` received the
exact source and test evidence in `/tmp/work/impl-page-tabs/PACKET-R3.md`.
The reviewer accepted R1–R3 and returned R4 because an ended, unarchived
session can still host a terminal. The coordinator updated the ruling
to prefer a live session and fall back to an ended, unarchived one.

`08624f6c9` adds that fallback, keeps the archive warning and updates the
worktree browser case to launch a terminal after the last live agent
exits. Its exact web gate `impl-page-tabs-20261120T054654Z-08624f6c`
passed all nine steps, with 700 unit and 107 browser tests. Logs are in
`/tmp/work/impl-page-tabs/gate-r12`. Reviewer `6fcba8b5` received
`/tmp/work/impl-page-tabs/PACKET-R4.md`. Send the packet to integrator
`ec7c91e9` on approval. There are no queued verification runs and the
isolated capture fleet has exited.

T2 reviewed the standalone-terminal seam and found no remaining issues
in this slice. It uses the existing terminal protocol with polling and
attach/focus resize. TRACKER 826 owns automatic replies to live terminal
queries. The browser entry awaits the browser pane.
The page still uses protocol 36-compatible polling and resize. T2 owns
the node's workspace-folder terminal ownership change in ADR 0121.
