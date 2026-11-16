# Paseo parity: where the work stands

Updated 2026-11-16, 07:05 UTC. The full row-by-row state is in `TRACKER.md`. The scored catalogue is in `measure/` (latest `SUMMARY-20261116-0657.md`).

## Measured score (source coverage, 1,419 catalogue rows)

| Surface | 2026-11-15 08:20 | 2026-11-15 12:00 | 2026-11-16 06:57 |
|---|---:|---:|---:|
| Native app (`spikes/native`) | 27.45 % | 28.08 % | 28.47 % |
| Node (`crates/`) | 30.02 % | 30.02 % | 30.09 % |
| Web page (`web/`) | 28.47 % | 28.47 % | 28.47 % |

`README.md` explains how the score is computed. The 06:57 measurement reads native package base 14.6 (be93a803) and node 5243a701. It moved little because most of the night's work landed behind a protocol constant that is still unset, a test-only module, or a staged producer with no caller; the strict rule scores such code at most "partial". The summary lists what scores as soon as it is switched on: file panels (`file.*`), usage and account reports, sub-agent relations, layout residents, node diagnostics in the app, and labels. Areas with nothing yet: plugins (39 rows) and the Hub (8 rows).

## Lines and builds

- Node line (`backend/m12-node`), head **13b01a92f**:
  - Gate 62 green on all 18 steps: Rust 2,862 passed / 0 failed / 12 ignored; web unit 465; browser end-to-end 80; forge 4; generated wire types unchanged.
  - Store schema 22 (the session mutation clock). Since 27502436: the lifecycle package, diagnostics, settings writer, relay lifetime, plugin values, usage backend preparation (constants off), the mutation clock, and a spawn-log test isolated into its own binary.
  - Next (gate 63, ordinary lane): session titles, workspace labels (schema 23), and a move-only split of the forwarding table. Then workspace services and the DNS resolution primitive (ADR 0104).
- App line (`backend/m12-package-base`), package base **14.9** at **95a601ff**:
  - Green: native 1,810 passed / 0 failed / 2 ignored; the wasm32 check of the link and app crates has no warnings.
  - Since base 12: node 13b01a92f, popup placement (the reference's vertical anchor rules), the pane header's maximise/restore actions, the compact dialog profile, the secure and code fields, shown-source receipts, scroll notification for virtual lists, and the Devices settings page.
- Published build: **0.0.0.0.22** (preview feed, 2026-11-15 19:35 UTC).

## Open work

- The Codex-based worker tracks are paused for the week (the maintainer keeps 30 % of that quota). Work continues with Claude Opus and Sonnet workers: reviews, receiving packets, visual comparison against the pinned reference, and switching staged work on.
- Next: switch on what is already built (file panels, usage reports, sub-agent relations, labels, residents, diagnostics UI), then the largest empty areas (usage and quota cards, notifications, PR timeline, scripts and services, browser pane, terminal input).
- Open visual follow-ups from today's comparisons: the pane header's split menu and new-tab menu (TRACKER 776), the import sheet's header search and press-to-import rows (TRACKER 777), the host-confirmation route rows, and the lightbox toolbar's disabled-state styling.

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
