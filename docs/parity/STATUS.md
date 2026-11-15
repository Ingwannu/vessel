# Paseo parity: where the work stands

Updated 2026-11-15, 19:50 UTC. The full row-by-row state is in `TRACKER.md`. The scored catalogue is in `measure/`.

## Measured score (source coverage, 1,419 catalogue rows)

| Surface | 08:20 UTC | 12:00 UTC |
|---|---:|---:|
| Native app (`spikes/native`) | 27.45 % | 28.08 % |
| Node (`crates/`) | 30.02 % | 30.02 % |
| Web page (`web/`) | 28.47 % | 28.47 % |

`README.md` explains how the score is computed. Each surface is scored against all 1,419 rows. The goal of parity on 2026-11-15 was not reached. No re-measurement has run since 12:00. The packages that landed since then (listed below) are counted at the next measurement.

## Lines and builds

- Node line (`backend/m12-node`), head **27502436**:
  - Gate 54 green on all 18 steps: Rust 2,635 passed / 0 failed / 12 ignored; web unit 464; browser end-to-end 80; forge 4.
  - Protocol 25 (the project and workspace registry's six metadata calls), minimum protocol 2, store schema 21 (session listing).
  - Landed since the morning: diagnostics (protocol 24), project admission, the shared Settings writer, the registry assignment, the session lifecycle package, workspace file I/O, the MCP tool and browser values carrier, and the timeline notice values.
  - Gate 55 (RPC caller-drop fence, schedule edits on ended runs, relay lifetime, the migrations file split) was 17/18. Its one browser test failed on the intermittent sign-in refusal described below. Gate 56b carries the same changes plus the sign-in fix and two more packages.
- App line (`backend/m12-package-base`), package base **12** at **b5b46357**:
  - Green: native 1,614 passed / 0 failed / 2 ignored, with the locked dependency set unchanged.
  - Since base 9: node 27502436, the import window for earlier conversations, the toolbar button and file-close guard, panel glyphs, the shared window registry, the narrow-width sidebar policy (compared against Paseo captures), the Explorer and lazy agent-sheet body, the translation generator, and wasm32 isolation for the app link crate.
- Published build: **0.0.0.0.22** (preview feed, 19:35 UTC). It was smoke-tested on Linux against a real node. It is the first update downloaded compressed and resumable from 0.0.0.0.21; both a fresh and an interrupted download were verified.

## Open work, by owner

- Sign-in (N2): a timed-out sign-in wait now closes as "try again" instead of "unauthorized". A retained browser error page confirmed that this refusal caused the intermittent test failures of the day. The fix is in the node integration gate now.
- Settings (nodectl with N5): one shared owner for the settings record, its writer and a pre-spawn admission fence. The contract and the first lane are reviewed. The joint gate is next. Restart recovery stays off until its own conditions are met.
- Provider preparation (W22): one shared seam keeps the caller's permit until blocking preparation ends, and keeps the create exclusion through registration. Corrections are in review.
- Terminal (C2, W17): the screen engine now matches the reference on alternate-screen entry and exit, cursor carry and soft reset (55 reference checkpoints). Its final full gate is running. The query responder stays off until then.
- Voice (W19): the inference runtime dependency is provisioned offline (dynamic loading, no build-time download). The values carrier and the compile are next.
- Desktop ownership (W15): the app owns a local node only after proving the peer: kernel peer credentials on Unix, and on Windows a one-use challenge over a private per-launch pipe. Mutual proof for the shared Windows pipe is a separate open item.
- Changes and git (W13, G1): the shown-Changes producer that carries the original context is in progress. Git writes on data without that context stay refused.
- App wasm32 build: 7 native-only references remain (TRACKER 768).
- On 2026-11-15 at 19:05 UTC the night's model credit ran out. Most worker tracks are paused until it is renewed, and each track records its resume point.

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
