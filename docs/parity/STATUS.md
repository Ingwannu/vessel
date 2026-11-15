# Paseo parity: where the work stands

Updated 2026-11-15, 13:00 UTC. The full row-by-row state is in `TRACKER.md`. The scored catalogue is in
`measure/`.

## Measured score (source coverage, 1,419 catalogue rows)

| Surface | 08:20 UTC | 12:00 UTC |
|---|---:|---:|
| Native app (`spikes/native`) | 27.45 % | 28.08 % |
| Node (`crates/`) | 30.02 % | 30.02 % |
| Web page (`web/`) | 28.47 % | 28.47 % |

`README.md` explains how the score is computed. Each surface is scored against all 1,419 rows, so the
node's number is not a server-only completion rate. Much of the work since 12:00 is merged into
neither scored head yet: packages waiting for review, gates and base merges. The next measurement
will count it.

## Tracker rows by state

From `TRACKER.md`, which owners update when their package passes:

| State | Rows |
|---|---:|
| unknown (owner has not audited it against Vessel yet) | 378 |
| missing | 30 |
| skeleton (types, wiring and placeholders in place) | about 290 |
| built | 33 |
| passed | 2 |
| merged | 0 |

## How work flows

1. A package is written on its own branch by its owner (see `PLAN.md` section 5). It follows the four-layer
   shape in `PLAN.md` section 2: words, state and rules, node boundary, screen.
2. A reviewer checks the source against the survey: V1 for the strict app tracks, V3 for node and
   takeover tracks, V4 for packages. The package then runs its narrow checks and one whole gate on the
   build machine.
3. The node line (`backend/m12-node`) and the app line (`backend/m12-spike`, the same as
   `backend/m12-package-base`) move forward by base merges only. Node changes reach the app line through
   those merges, never directly.

## Open work, by area

Each item names the package that owns it. "Waits on" names what must land first.

- **Shared controls (G2, G3, W4, S2)**
  - The Circle button and ExternalLink passed review and wait for the next base.
  - The field identity (W4) lands first, then G2's multiline field, then G3's shared SearchField. One writer at a time.
  - G2 adds the shared form-field frame (label, trailing, hint and error).
  - The menu per-open lease fix (S2) was returned once. A nested-dialog Escape case needs a real regression test.
- **Sessions and agents (W3, W12, W16, W2, W22)**
  - Agent creation is staged. Activating real creation waits on the project and config admission guard (W11) and a fixed-id seam.
  - Resident delivery (W12) has a correction batch for test liveness.
  - Provider coverage on the node (W22) is gating a corrected head.
- **Projects and workspaces (W11)**
  - The registry core passed review after one correction. Six core adapters remain.
  - Two shared node timing defects found by its gate (archive → resume conflict, comparison wait) are being fixed in the base.
- **Usage and limits (N2, W5)**
  - The toast, dialog and menu focus package passed all 18 gate steps and is in review.
  - The usage page's plugin rows wait on producer adapters that do not exist yet. Until then they stay Unsupported.
- **Scripts, services and preview (W8, W14)**
  - Setup UI is staged.
  - The service snapshot, health and proxy fields have no node DTO yet, so that UI is not drawn.
  - The browser snapshot script is written from Paseo's behaviour, not from its code.
- **Devices and sync (G4)**
  - A relay lifetime seam on the node: the registration generation also owns pending data dials.
- **Voice (W19)**
  - A deadlock in receipt settling was fixed before it ran. A replacement gate is queued.
- **Terminals (C2)**
  - One terminal screen engine (ADR 0095) on a vendored vt100.
- **Node wire gaps (N3, W17, W18)**
  - Launch profiles, output seams and the parser seam passed source review. Their gates are queued.
- **Native release**
  - Installable builds are labelled 0.0.0.0.N. The Windows typecheck interface was opened today.

## Not in scope yet

The Paseo Hub and the phone app are not part of this plan's parity rows until the maintainer decides.
