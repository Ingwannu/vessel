# Paseo parity: where the work stands

Updated 2026-11-15, 14:55 UTC (end of the day's push). The full row-by-row state is in `TRACKER.md`. The scored catalogue is in `measure/`.

## Measured score (source coverage, 1,419 catalogue rows)

| Surface | 08:20 UTC | 12:00 UTC |
|---|---:|---:|
| Native app (`spikes/native`) | 27.45 % | 28.08 % |
| Node (`crates/`) | 30.02 % | 30.02 % |
| Web page (`web/`) | 28.47 % | 28.47 % |

`README.md` explains how the score is computed. Each surface is scored against all 1,419 rows. The goal of parity "today" was not reached. Most of the afternoon's work is reviewed but not yet merged into a scored head; it is listed below. The next measurement counts it.

## Lines and builds

- App line (`backend/m12-package-base`):
  - At e1b73c9e (package base 8 = node 82ae7c91 + S2's menu per-open lease + W12's staged delivery + W4's field identity), plus W12's selected() getter, the release branch and the Subagents leaf.
  - The next head, c3429952 (W12's core union and W1's search ownership move), is gating.
- Node line (`backend/m12-node`):
  - At 82ae7c91.
  - Gate 49 runs batches 11 and 12. Batch 11: W11 base fixes, W16 portable wait, W8 setup values, dictation's settings diagnostic, W2's detach core, W11's registry, and the ConnectionLife seam. Batch 12: W7's attention values, the shared Settings writer, G1's staged git compare, N1's diagnostics (protocol 24), and W11's project admission.
- Published build: 0.0.0.0.21 (preview feed, 14:02 UTC).
  - It adds the menu fix, dialog focus return, the outline rail following resizes, and resumable compressed updates.
  - Compiled on the build VM, including the first Windows build there. The Linux smoke test passed.

## Reviewed and waiting for a base (not yet in a scored head)

- Shared controls:
  - G2: multiline field and the picker per-open admission.
  - G3: SearchField and ExternalLink.
  - G2: Circle button.
  - W19: the shared popover.
  - S2: SavePrefs boxing and a declarative glyph inventory.
- Node packages:
  - N1: diagnostics (protocol 24).
  - W11: project admission and fixed ids, which unblocks W3's agent creation.
  - GKS nodectl: the shared Settings writer.
  - Dictation's pure codec seam.
  - G4: relay lifetime.
  - C2: terminal screen engine (gate owed).
  - W18: Skills and browser values carrier.

## Open work, by owner

- Agent creation (W3): adapters are written against the admission package, and wait for the gate-49 node head.
- Projects and workspaces (W11):
  - The native registry projection waits for base 9.
  - An early values carrier was reverted, because `(pending)` rows need node handlers.
- Usage (W5) and Schedules (G2): wait for L's generator fix, which splits generated resolve functions over 100 lines.
- Plugins (W16), scripts and services (W8), preview (GKS workspace), browser (W14): each has its contract reviewed. Each waits on a named producer: the service DTO, the canonical connection, the browser test-executable run.
- Terminal (C2): the node answers terminal queries and strips them from presentation. A rare ambiguous overlap is left to the client. It stays off until that path passes.
- Git (G1) and providers (W22): still skeleton. Corrections are in review.

## Rules settled today that affect the remaining work

- Protocol numbers are assigned at merge, in merge order. A reservation only marks intent.
- The priority build lane is only for:
  - a shared seam that named workers are blocked on;
  - the release label;
  - node integration gates.
- Values carriers must build the root node before they reach the app line.
- When a survey row and the pinned Paseo source disagree, the source wins.

## Not in scope yet

The Paseo Hub and the phone app are not part of this plan's parity rows until the maintainer decides.
