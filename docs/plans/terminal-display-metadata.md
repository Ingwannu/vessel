# Authoritative display metadata and measured cursor corrections

[Decision Log]
- Purpose: a selected frame must carry the program's cursor overrides and
  physical row endings in the same read cut as its cells, without guessed
  defaults or another tracker.
- Existing assets: the maintained engine's cursor/grid state and the same
  decoded VTE parameter metadata. Reuse both; snapshots remain readonly.
- Alternatives: inspect raw bytes in a second parser, infer wrapping from text,
  or restore a decoder by replaying serialized ANSI. They lose chunking/state
  or duplicate interpretation.
- Choice: typed optional CursorShape and CursorBlink overrides, plus outgoing
  RowBreak Hard/Soft. Program defaults remain unset. Preserve blink-only state,
  soft-reset presentation without moving/erasing cells, inherit1049 alternate
  cursor coordinates, and clamp restored pending wrap as the reference does.
- Why: actual pinned-headless oracle cases exposed two prior semantic failures,
  including an existing alternate entry position error that affects capture/CPR,
  followed by a pending-wrap restore difference. These need engine corrections,
  not node projection workarounds.
- Effects: no new parser, crate/runtime/dependency, wire method, actor, renderer,
  mutable decoder restoration or NodeAuthority activation. The optional shape
  and blink are independent; an unset value belongs to renderer defaults.

References: Paseo server terminal/terminal-serializer.ts25-41 preserves cursor
overrides; terminal/test-utils/terminal-state.ts37 maps next-row isWrapped to
outgoing wrap. Native terminal-selection.ts333-365 joins soft-wrapped rows.
Exact installed headless InputHandler2817/2858/2995/3017 and BufferSet103-112
define soft reset, cursor style/save/restore and alternate entry. Source-map SHA
90f9e075de89f71a2e0510b72ab83a3a7619c51089e2d0ed857c2d6a43f13af4.

Raw metadata golden initially26 checkpoints is retained; its every field/value
was compared unchanged before adding cell text/foreground/cursor and programs
to34 checkpoints. All original31 cell goldens,239 mode and11 marker checkpoints
are unchanged. The latent-column unit fixture now explicitly homes its own test
text, retaining every original A/G/bounds/inactive assertion; independent bare
1049 cases require nonzero/pending entry, so that fixture cannot hide the fix.

Pre-fix3666 executed44/2 with strict0. d105 executed46/2 and test-only lint
failure. Corrected b4be source executed48/0 and strict0 in the separate display
serializer branch, including five serializer-only tests. This isolated minimum
producer deliberately omits that serializer, its readonly cell constructors
and its vendor width-table accessor. Its own exact narrow/enclosing qualification
and source review remain owed. Frozen2d5 whole is unchanged; no full consumer
or universal Unicode/control-stream parity follows from finite fixtures.
