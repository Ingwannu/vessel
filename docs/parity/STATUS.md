# Paseo parity: where the work stands

Updated 2026-11-15, 17:25 UTC. The full row-by-row state is in `TRACKER.md`. The scored catalogue is in `measure/`.

## Measured score (source coverage, 1,419 catalogue rows)

| Surface | 08:20 UTC | 12:00 UTC |
|---|---:|---:|
| Native app (`spikes/native`) | 27.45 % | 28.08 % |
| Node (`crates/`) | 30.02 % | 30.02 % |
| Web page (`web/`) | 28.47 % | 28.47 % |

`README.md` explains how the score is computed. Each surface is scored against all 1,419 rows. The goal of parity on 2026-11-15 was not reached. No re-measurement has run since 12:00. The packages that landed since then (listed below) are counted at the next measurement.

## Lines and builds

- Node line (`backend/m12-node`), head **1c762ccf**:
  - Gate 52 green on all 18 steps: Rust 2,550 passed / 0 failed / 12 ignored; web unit 464; browser end-to-end 80; forge 4.
  - Protocol 25: the project and workspace registry's six metadata calls (list, get, rename, pin, release automation, registry). Minimum protocol 2, store schema 20.
  - Since this morning: diagnostics (protocol 24), project admission and fixed ids, the shared Settings writer, the guarded handler boxing that fixed a worker-stack overflow, the web toast width correction, and the registry assignment.
  - Gate 53 runs next on the session lifecycle package (store schema 21).
- App line (`backend/m12-package-base`), package base **9** at **db42d9c5**:
  - Green: native 1,510 passed / 0 failed / 2 ignored, with the locked dependency set unchanged.
  - Contents: node 9c150c6e, the shared search field, the general translation words, GPUI zoom, the glyph inventory, inbox attention values, the picker admission and the workspace-tab owner hooks.
  - Gating next: the dialog header actions; then the timeline resident seam, the git control patch and node 1c762ccf (base 10 candidate).
- Published build: 0.0.0.0.21 (preview feed, 14:02 UTC).

## Reviewed and waiting for a base (not yet in a scored head)

- Shared controls: dialog header actions; git control patch 7; the timeline resident seam; message-search navigation hooks; the import window correction.
- Node packages: workspace file I/O (W13); MCP tools and browser values (W18); worktree setup progress and cancel (W8, its own handoff); the session lifecycle package (gate 53).

## Open work, by owner

- Project registry (W11): protocol 25 is assigned. The supported-wire test matrix on that head is next. Workspace membership reads for inbox read/unread are in checks.
- Settings (nodectl with N5): one shared owner for the settings record and its writer. The contract is reviewed. The restart-recovery hook and the implementation are next, followed by one joint gate.
- Provider preparation (W22): one shared seam keeps the caller's permit until blocking preparation actually ends. It also keeps the create exclusion through registration. The source is settled and the checks are owed.
- Terminal (C2, W17):
  - The screen engine's history-cell fix is reviewed.
  - The node's query responder has a written failure policy. A full reply queue never blocks output.
  - The responder stays off until the engine passes.
- Changes and git (W13, G1): a shown-Changes producer that carries the original context is being written. Git writes on data without that context stay refused.
- Voice (W19): the inference runtime dependency is decided (dynamic loading, no build-time download). Provisioning into the offline index and the compile come next.
- Web sign-in (N2): a timed-out sign-in wait now closes as "try again" instead of "unauthorized". The source is reviewed and the checks are owed.
- Wasm build of the app link (G4): failing; being classified against the base.

## Rules settled on 2026-11-15 that affect the remaining work

- Protocol numbers are assigned at merge, in merge order. A reservation only marks intent.
- The priority build lane is only for:
  - a shared seam that named workers are blocked on;
  - the release label;
  - node integration gates.
  A source scan (test modules last, function length, known lints) runs before each priority request. A failed priority run goes back to the ordinary lanes.
- Values carriers must build the root node before they reach the app line.
- When a survey row and the pinned Paseo source disagree, the source wins.
- A defect found in a shared base is fixed once in the base. Each affected track then reruns its own full gate on the corrected source.

## Not in scope yet

The Paseo Hub and the phone app are not part of this plan's parity rows until the maintainer decides.
