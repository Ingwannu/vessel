# Page workspace tabs resume

TRACKER 817 lives on `backend/m12-page-tabs`. The implementation runs from
`55aa80f06` through `3bd1e64cf`. ADR 0118 explains workspace identity,
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

The follow-up web gate `impl-page-tabs-20261120T041717Z-3bd1e64c` is
queued at this checkpoint. After it passes, export the same commit and
capture the follow-up frames with the browser fixture. Anchor Close
other tabs on Diff to include the terminal. Send the frame index and
Paseo source lines to the coordinator for visual review, then follow the
page review and integration handoff.

T2 reviewed the standalone-terminal seam and found no remaining issues
in this slice. It uses the existing terminal protocol with polling and
attach/focus resize. TRACKER 826 owns automatic replies to live terminal
queries. The browser entry awaits the browser pane.
