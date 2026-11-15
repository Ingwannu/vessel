# Paseo parity: plan, tracker and measurements

Vessel's goal for milestone 12 is parity with Paseo 0.15.4. Paseo's behaviour and look are reproduced in
strict Rust. Its code and assets are never copied. Extras beyond Paseo (PDF and SVG viewers, LSP, saved
prompts, moving a session) are welcome after parity and are marked "beyond".

This folder holds the working documents that drive that effort. It is kept in the repository so that the
plan, what is done, what is in progress and what is not started are all in one place.

| File | What it is |
|---|---|
| `STATUS.md` | Where the work stands now: the latest measured score and the open work, by area |
| `PLAN.md` | The implementation plan: architecture every package follows, packages and owners, rulings |
| `TRACKER.md` | One row per Paseo feature: owner, package and state (unknown / missing / skeleton / built / passed / merged) |
| `FEATURES.md` | Every Paseo feature (260 features, about 700 sub-features), with pointers into the surveys |
| `CATALOGUE.md` | The 1,419-row feature catalogue the measurement scores against |
| `CONNECTIONS.md` | How the features depend on each other (which ones share state, wire or screens) |
| `IMPLEMENTATION-PLAN.md` | The first cut of the plan, before owners were assigned |
| `gaps.md` | Features the first survey missed, added later |
| `SURVEY-BRIEF.md` | The brief the surveys were written against |
| `vessel-today.md` | Vessel's state when the plan was written (2026-11-15 morning) |
| `survey/` | The surveys of Paseo, by area (behaviour, states, values and wire, as Paseo 0.15.4 has them) |
| `measure/` | Measurements: each run's summary and the scored catalogue |

## How the score is computed

`score = 100 × (present + 0.5 × partial) / 1,419`, separately for the native app, the node and the web
page. A row counts as present only when the source implements the whole behaviour. It counts as
partial when part of it exists. Rows that are unclear or unmeasured stay in the denominator. The score
measures source coverage. It is not a usability score or a release approval.

## Notes for readers

- Commit hashes in these documents were taken from the working history before it was published. The
  history was rewritten for publication: AI tool attribution lines and committed build output were
  removed, and machine names and paths were replaced. So hashes taken before the publication differ
  from the published ones. `STATUS.md` uses published hashes.
- Worker labels (W1 … W22, G1 … G4, N1 … N5, V1 … V4 and so on) name work packages and their reviewers.
  `PLAN.md` section 5 lists them.
- Paths such as `/tmp/work/…` and `/tmp/build/…` stand for the build machine's scratch and build
  folders. Their files (gate logs, captures, audits) are not part of the repository.
