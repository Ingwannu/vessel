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
and its vendor width-table accessor. Exact96271b3e now executed its own
43 tests with zero failures, and all-target Clippy with warnings denied passed.
The commands visibly compiled/checked vessel-terminal-screen; these are the
isolated producer results, not the serializer branch's48-test result. The
changed-source enclosing whole and source review remain owed.
Frozen2d5 whole is unchanged; no full consumer
or universal Unicode/control-stream parity follows from finite fixtures.

Source review returned two additional concrete cases: DECSTR left the facade's VT input
switches stale, and repeated1049 erased already active alternate cells. A new
oracle attachment adds nine checkpoints, preserving every original34 snapshot
field/value and program event. Its red regression is202764a2, submitted before
the repair. The same decoded mode owner now resets app-cursor/bracketed-paste.
The server keyboard tracker still owns Kitty/Win32 flags and saved stack: it
does not recognize DECSTR/RIS. Both reference traces are retained explicitly;
no raw tracker or expected value inversion is introduced.

Re-entry retains DECSC on the current buffer, but clears/advances alternate
epoch only on a real transition. Cursor ink is saved per physical buffer so
repeated alternate saves cannot overwrite primary restore attributes; DECSTR
resets only the active buffer's saved presentation. Oracle checks include
repeated entry, restore position and foreground; a marker regression guards
against a spurious epoch reset. Earlier readonly snapshots are checked again
after every later mutation. Actual changed-source checks remain owed.

Exact202764a2 red executed43 passed/two failed, reproducing both named
returns without a compiler failure. Corrected346653a9 executed46/0/0,
including retained snapshot and marker checks. Its strict check found only
three unused test Replies results; the follow-up strengthens these to explicit
empty-reply assertions, with no product or golden change. Corrected strict and
the final enclosing whole remain owed. Original red logs are retained.

Final3de strict passed. Further source inspection of the same BufferSet path
found47 cursor carry and1047 absent, plus alternate marker disposal delayed
until re-entry. Twelve actual reference checkpoints supplement the untouched
earlier43. The first new fixture decode failed because numeric enum names needed
explicit serde spellings; this harness failure is retained and is not reported
as a semantic reproduction. Corrected baselineba03bc1f is submitted separately.
The pending repair shares entry/exit transitions, carries coordinates without
extra save/restore for47/1047, and disposes alternate markers on actual exit.
Repeated1049 still performs only its specified cursor save. No other parser,
reference adaptation or additional foreign merge is introduced.

Final verification before the changed-source enclosing gate: corrected
baselineba03bc1f executed46/2, specifically47 wrong cursor and1047 retaining
primary text instead of entering alternate. Correctedf0c71a8e executed48/0/0
and all-target strict Clippy passed on that exact source. All55 actual reference
checkpoints ran through live and retained snapshots, with original43 unchanged.
Raw test/strict log SHA prefixes are9626dfab74f365d3/e439d815ea849109.
The earlier40/8 harness error,43/2 semantic return,46/0 intermediate correction
and unused-result strict failure remain preserved as their actual dispositions.

The frozen predecessor2d5 completed its whole with18 actual steps/18 zero,
Rust2589/0/12, web464, E2E80 and forge4. It does not contain these later fixes.
One meaningful changed-source whole on this final producer is still required;
its result and source recheck must precede any consumer qualification. No node
responder, actor, EOF sealed owner, wire/frame DTO, native/CLI renderer or OS
evidence is borrowed from the pure producer tests. NodeAuthority remains off.
