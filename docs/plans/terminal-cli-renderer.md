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

The initial renderer and adapters are test-only until typed frame transport,
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

Renderer/readonly cell construction and tests are next. No new compile result,
consumer integration, ANSI oracle or visual evidence exists for this branch.
