# Syntax decoration for existing timeline output

This preparation composes the checked body prerequisite with K's minimal syntax
producer on package base13. Neither its assembly nor this extension is validated
yet. Both tool-body facade/kernel keep their literal staging attributes.

[Decision Log]
- Purpose: canonical read/write/edit previews should use the same semantic file
  syntax and selected code colors as the editor, without opening provider paths.
- Existing assets and constraints: the View lexer already supplies byte-range
  tokens and block-comment state. K owns one seven-kind adapter and CodeStyles
  palette resolver. Kit output already owns bounded virtual rows, diff emphasis,
  numbering, scrolling and exact copy data. A duplicate renderer or color table
  would drift and could lose changed ranges or UTF-8 boundaries.
- Alternatives: a new grammar dependency or a second editor/preview component
  adds parser/style ownership and state. Baked foreground colors in cached lines
  would leave old colors after theme changes.
- Choice: optional SyntaxLines wraps the existing AnnotatedLines data; the same
  Kept cache validates semantic runs and the same uniform list draws them. UTF-8
  validation is shared with background ranges. The current CodeStyles resolves
  kinds at draw time. GPUI's existing combine_highlights produces disjoint spans
  for independent syntax foreground and changed-word background attributes.
- Why: Lines and AnnotatedLines callers remain source compatible, text/copy/line
  numbers and tones stay unchanged, and warm draws do not tokenize again.
- Effects: file previews carry lexer state across lines; independent diff lines
  start from Code because omitted context cannot establish block state. Only
  decoration is limited to MAX_SYNTAX_BYTES, never source text or copy content.
  Missing/non-UTF-8 paths yield ordinary output; a provider path is a display hint
  and grants no file, workspace, pane or node authority.

Code and patch cache revisions include reported path and numbering inputs; an
equal payload refined to another language rebuilds semantic runs once. Scheme
and palette choices remain outside those revisions and recolor existing runs.
No syntax changes are applied to terminal prose, JSON or arbitrary MCP data.
K's exact minimal producer also lowers code.rs's prior color literal count from
18 to6; this receiving union lowers that recorded ratchet without adding one.

Meaningful checks cover shared lexer state, independent diff state, absent and
changed path hints, exhausted syntax budget preserving all text/copy, invalid
and overlapping UTF-8 spans, current-theme colors with preserved word emphasis,
and actual virtualized warm draws reusing one factory across theme changes.
These are model/control evidence only until executed. No screenshot, UI, V1,
whole A7 or product resource/transport claim follows from this preparation.

Plain file tokens map to the existing Variable syntax role, matching CodeTokens'
preview foreground in the same independent palette. They do not fall back to UI
foreground while keywords alone follow a code palette. This is one existing role
selection, not another lexer-kind table; a cross-resolver regression pins it.

Actual prior-source072c19b9 execution completed App141/0/2 +Kit256/0/0 =397/0/2
in run wp-timeline-syntax-tests-20261116T020757Z-072c19b9. All named semantic
lexer, budget, copy, UTF-8, merged-color and actual warm virtualized cases passed.
Its strict run returned only three syntax/style diagnostics: an unnecessary
self qualification and two single-range Vec test initializers. They are corrected
without changing assertions or behavior. The normal announced14.2 merge is also
present; actual receiving checks on this newer source are still owed. No A7
main/host/raster/V1/F5 approval is borrowed from that earlier-source execution.
