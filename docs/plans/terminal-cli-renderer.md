# CLI authoritative-frame renderer (staged)

[Decision Log]
- Purpose: present authoritative typed cells and negotiated modes without
  forwarding query-bearing output or reconstructing a decoder from a pruned log.
- Existing assets: the pure frame/cell facade, current raw-mode guard/input
  sender/detach scanner, and the one node actor screen helper. The reference
  CLI's terminal commands provide capture/send-keys, while agent attach prints
  structured timeline entries; this interactive renderer preserves Vessel's
  existing terminal attach under the approved multiplexer design.
- Alternatives: a second parser, synthetic ANSI state restoration, raw-suffix
  replay after a visible snapshot, or removing old interactive attach. They
  duplicate interpretation, lose decoder state or remove a working feature.
- Choice: one bounded ANSI display backend taking typed cell projections.
  It produces bytes only; it does not write stdin/stdout or feed an engine.
  Real terminal input/send ordering and existing detach/reset guards stay at
  the CLI integration edge. Wire DTO mapping comes from the node owner.
- Why: the renderer can read the same cell contract from a borrowed engine
  frame or an authenticated wire frame, with no parser/model recreation.
- Cost: cell text must reject control injection before any output is written;
  clipped wide cells and cursor/mode bounds need independent terminal tests.

## Staging boundary

The initial pure renderer is not connected to attach until typed frame transport,
old-peer compatibility and ownership are agreed and checked. No runtime
attach cutover, new RPC, query delegation or NodeAuthority activation occurs.
The frozen engine producer at2807 is unchanged in its own worktree/whole gate.
Private renderer tests are not a physical terminal or full CLI parity claim.
Normal queries remain node-owned; ambiguous overlap candidates grant no reply
permission and frame-only overlap remains unanswered under the explicit rule.
Program cursor shape remains pending; this first step uses position/visibility
and leaves the host's chosen cursor shape unchanged.

## Reference reads

Paseo packages/cli/src/commands/agent/attach.ts prints projected timeline output;
packages/cli/src/commands/terminal/{capture,send-keys}.ts define terminal CLI
surfaces. packages/app/src/terminal/runtime/terminal-emulator-runtime.ts560–561
sets its browser cursor to blinking bar. The existing Vessel CLI guard resets
keyboard/mouse/alternate/bracketed-paste state on every exit; changes must keep
that lifecycle. Output ANSI is a physical display backend, never engine replay.

## Resume

A pure bounded display serializer and readonly cell constructors are staged
inside the existing screen facade. The CLI will call that single serializer;
no separate renderer/parser crate or GUI control is added. Validation rejects
cell control injection before the caller obtains any output bytes. Program
cursor shape remains inherited rather than guessed in this first stage.
Renderer/readonly cell construction tests are next. No new compile result,
consumer integration, ANSI oracle or visual evidence exists for this branch.

## Staged implementation and next verification

The pure serializer is implemented with one callback adapter for engine or
wire cells, a1MiB encoded-byte ceiling, early cursor/cell validation, explicit
positions, default-blank skipping and clipped-wide handling. Modes replace
local Kitty flags at the current stack entry; enter_display supplies one push
for the CLI guard to pop later. No runtime CLI caller is connected yet.
Read-only constructors validate control-free cell text and declared span
against the vendored engine width table, with the original22-byte ceiling.
Tests include injection/geometry/pair refusal, early byte-budget termination
and four stdout-only probe cases for an independent pinned terminal oracle.
No process/filesystem/environment/toolkit or testkit dependency is added.
Probe export is data generation, not a terminal-parity test; the independent
oracle must run and be inspected separately. Source checks remain owed.

First remote check at93b9 compiled the crate and ran42 tests:41 passed,
the display fixture refused with Unavailable. The corresponding strict check
found a non-Copy private channel enum passed by value. The enum is corrected;
the fixture now checks every selected cell with scene/coordinate context before
encoding, so a projection defect cannot hide behind a generic display refusal.
The original failure and all expectations remain; a corrected runtime result
and the independent display oracle are still owed.

Readonly cell admission also follows the engine's documented one positive-width
base plus zero-width combining marks invariant. A string such as two ASCII
letters cannot masquerade as one wide lead/continuation pair merely because
its summed string width is two. The small existing vendor width-table accessor
checks that representation without another parser, normalization or grapheme
implementation. The finite upstream cell/grapheme limitation remains unchanged.

The strengthened d7dd fixture failed at history row1, column1: upstream trims
default trailing cells before storing history, so the frame facade must project
implicit blanks. Its old tests read only the first glyph and did not catch this.
The strict d7dd check separately rejected the fixture's println macro; the
stdout-only data export now writes through a locked stdout handle. No fixture
expectation is removed and no terminal-parity result is inferred.

## Next display metadata contract

Reuse the existing engine's cursor presentation and physical row wrap flags;
do not infer them from text or replay control strings. A readonly frame will
carry typed optional program cursor shape/blink overrides, with unset values
resolved by each renderer's configured defaults. Physical row ending is typed
Hard/Soft, meaning continuation onto the NEXT row, independent of the first
retained row's unavailable predecessor. No authority, route or revision is
inferred from either property.

References read: server terminal/terminal-serializer.ts25-41 preserves explicit
cursor overrides and leaves unset presentation to renderer defaults;
terminal/test-utils/terminal-state.ts37 maps next-row isWrapped to outgoing
wrap flags. Native renderer terminal-selection.ts333-365 joins soft-wrapped
rows rather than inventing line breaks. The pinned headless InputHandler
source map implements DECSCUSR at2858 and soft reset at2817; these must be
measured, including reset/invalid parameters, rather than guessed from a generic
terminal specification. DEC12 blink changes require its optional compatibility
quirk in this reference; the normal factory does not enable that quirk.

Decision: extend the existing pure facade and the one display serializer.
The engine is the only owner of interpreted presentation; a new scanner,
synthetic restoration or per-consumer cursor/wrap tracker is rejected. Keep
the engine extension whole frozen in its own tree. These additions remain
staged until their independent oracle, narrow checks and consumer integration.

The readonly metadata facade and26 exact pinned-headless checkpoints are now
staged. Cursor overrides retain shape and blink independently, including unset
renderer defaults; row endings are read from actual physical rows. The first
implementation exposes existing engine state without changing its control
interpretation, so the new regression can reveal unsupported cursor/reset cases
before any parity repair. Golden JSON is copied unchanged from the actual oracle.
No measured cursor/wrap parity or actual display consumer is claimed yet.

Exact3666 baseline compiled and passed strict Clippy, then executed44 tests
successfully with2 new semantic failures. The original26 metadata checkpoints
are preserved unchanged. The failures expose independent blink-only overrides
and alternate1049 entry using the wrong initial cursor. Minimal engine changes
use the SAME decoded CSI parameter metadata, a typed optional blink override,
soft-reset mode/saved-presentation reset without erasing/moving live cells, and
physical alternate cursor inheritance including a pending-wrap column.

The oracle is strengthened to34 checkpoints with full projected ASCII text,
foreground cells and cursor position/visibility, including soft reset after
origin/scroll-region/style changes and alternate entry/exit at pending wrap.
Every original26 field/value was independently compared unchanged before adding
the new fields/programs. The original oracle and failing run remain retained.
Readonly snapshots must preserve the same metadata/cell cut. No new parser,
ANSI feedback, dependency/runtime/I/O or actual terminal consumer is introduced.
This pending repair has not changed frozen2d5 or its whole; checks remain owed.
