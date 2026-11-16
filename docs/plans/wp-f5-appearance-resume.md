# K / F5 Appearance resume — 2026-11-16 quota checkpoint

Read this file first, then `/tmp/work/gk-k-themes/f5/STATE.json` and the end
of `wp-f5-appearance.md`. This is an unfinished package, not a V1 submission.
The quota watcher allowed only the current slice and asked for committed
resume notes and owned-process cleanup. No new slice/control/merge/build is
started after that notice.

## Source, commits and completed builds

- Worktree `/home/dev/vessel-m12-logos`, branch `backend/m12-logos`,
  `project=vessel`. App base13 `2ebeedf9` merged as `f665dd12a`.
- Executable source is **235309ac4dadeac7f93bac0bef1f9483d227e5f0**.
  Later commits through `db15dc161` change documentation only. The stored
  3044-file source manifest was independently rechecked with zero changes.
- Full native **themes-20261116T004442Z-235309ac**: fonts/fmt/quality/Clippy/
  build/test all0, no skipped step; raw **1652 passed / 0 failed / 2 ignored**.
  Out `/tmp/work/gk-k-themes/f5/package-base13-235309ac` and independently
  recomputed `gate-235309ac-summary.json`. No root/web source delta from base13.
- The original app export was coupled to old protocol23. The tools owner later
  tightened the verifier. Preserve that export/logs, but **exclude** old
  `f5/captures` matrix96 and incomplete editing frames from current evidence.
- Coordinator/T authorized one new bounded native-build/export, not a whole
  rerun: **themes-20261116T022602Z-235309ac**, out `f5/capture-p25-235309ac`.
  Fonts/build/export all0; supported verification passed. App SHA-256 is
  `fa6cac96e86db7c083b95ed271d238834051ffe4b9658b3c9da594c9ac2f7931`.
  App current/capture-min25, wire-min2; separately bound base12 root
  `b5b463579873b270501ee3f6c6b19fe0bd1080e2`, protocol25/schema21, same_commit=false.
- Both VM runs are **done**, with results retrieved. There is no queued/running
  K VM run. **Do not resubmit either or repeat the whole gate on unchanged Rust.**
  All future Cargo/rustc, including capture builds, remains build-VM-only.
  Untracked Cargo configs and font files remain untouched/uncommitted.

## Current evidence, not final acceptance

Current originals are only `/tmp/work/gk-k-themes/f5/captures-p25`:

- `matrix-manifest.json`: **96 original files** across EN/KO,1280x900/390x844,
  scales1/1.25/1.5/2,Light/Dark, tiles/fields/preview. It contains saved-file
  observations and the exact new app/root binding. Some wide sections share
  a frame; file count does not mean 96 distinct rendered states.
- `derived-matrix/manifest.json`: **16 labelled resized comparison sheets**,
  all personally opened. They are derived layout aids, not replacement originals
  or proof of native physical sharpness. Original hashes remain unchanged.
- `author-inspected.json`: explicit opened-file list (currently three selected
  originals plus all16 sheets); do not inflate it to every original being opened.
- `events.json` and `behavior-proof.json`: **in-progress** editing evidence.
  Current valid observations include real focus selection, code/content/interface
  limits, uncommitted preview, Enter/blur persistence, empty restoration and
  available/unknown/ordered-family behavior. Read the final checkpoint count,
  not a planned count. A partial series is not the complete behavior manifest.
- Remaining planned component manifests: behavior67, variants96, restarts32,
  private-System20, maximum26, conversation16, boundary48 (800/1024 at1/2).
  `aggregate_package.py` expects all components (401 originals total) and
  deliberately refuses a partial/fabricated final package. **Do not run it and
  weaken its checks to publish a partial package.**

No F5 package was submitted to V1: A14 asks for one complete package, not a
partial visual review. Earlier fb711/f14 theme approval and current self-run
gates are separate scopes. No full F5 UI/feature approval or lead integration
is claimed. Tracker own font rows are built, not passed; plugin rows stay
missing/deferred to F6. Native radio/AT/a11y and hardware/OS IME are not claimed.

## How to continue captures

Use the existing verified export, unless source actually changes. Capture
scripts are in `/tmp/work/gk-k-themes/f5`, with:

```sh
F5_CAPTURE_APP=/tmp/work/gk-k-themes/f5/capture-p25-235309ac/binaries/vessel-app
F5_CAPTURE_COMMIT=235309ac4dadeac7f93bac0bef1f9483d227e5f0
F5_CAPTURE_ROOT=/tmp/work/gk-k-themes/f5/captures-p25
F5_CAPTURE_FLEET=/tmp/work/gk-k-themes/f5/fleet-p25
```

Always use the supported `capture-export.py` verifier; never edit or rebind an
immutable manifest yourself. The new explicit base12 binding is mandatory.
`contact_matrix.py` uses installed ImageMagick; Pillow is absent, no install
is needed. Actual screenshots must be inspected before reporting.

Before rerunning any driver, read its logs/exit file and existing output.
The interrupted behavior driver has unique directory names and initial saved
value assumptions. **Do not blindly rerun it:** preserve its existing proof,
continue missing states into a new directory, and explicitly seed any required
initial values through UI. The actual private saved file can hold nondefaults.
The helpers' numeric focus test moves through another actual field first;
clicking an already-focused field does not cause a new focus/select-all event.
The input engine, not the capture script, owns raw text and selection.

The separate fake fleet was verified from the approved root export and setup
passed (2 nodes/18 fake sessions). Recreate/restart only private fixture
processes if cleanup stopped it, never the live preview or real providers.
Do not print node grants, pairing links or tokens. Source scripts have no
changes from the base12 fleet scripts (except unrelated quality data).

## Owned processes and cleanup

- Old `fleet-final` is stopped: two verified nodes stopped,23 recorded children
  already gone after termination,0 identity refusals. Original cleanup JSON kept.
- New `fleet-p25` ownership/descendants/permissions are in `fleet-p25-*.json`.
  UID/GID1000; directories0700, book/token files0600. No production permission
  change was made.
- A quota guard (PID1018214, verify identity before any action) is scheduled for
  **03:54 UTC**. It stops only recorded capture controllers and descendants,
  then refreshes and stops the owned p25 fleet using pidfds. Evidence:
  `quota-capture-cleanup.json`, `quota-fleet-cleanup.log`, `quota-cleanup-done`,
  `fleet-p25-cleanup.json`. Final manual verification/results will be appended.
- Controller identities are in `quota-owned-capture-roots.json`; no PID alone
  is authority. No `pkill`, process-name kill or `fleet.sh stop` (blind PID file).
  Verify cleanup, including any process started during the final census, rather
  than trusting a planned deadline or stale PID list.

## Other tracks and exact seams

- Syntax portable **45557713de5335c5c3fd433c737ffb195c297a4b**, followed by
  static-only **a786347bff13c63abbdb2f9c9923315e66fb7cf8** (code.rs allowance18→6).
  API `app::syntax_kind::of(TokenKind)` + `CodeStyles::of(Scheme).highlight`.
  Sent W10/G1/W13. One lexer, palette table and resolver; receiving union checks
  are theirs. Preserve their newer `syntax_for`/editor callbacks.
- Field notifications portable **92ad0451611fb3009c27db37a070766cc6740953**.
  `TextField::with_events` + `field::FieldEvents` on the same canonical one
  listen/Callbacks::submit; Windowed joins Plain/Identified. Sent E2/W12/W13/G2/C2.
  Producer source passed the full gate; portable receiving assemblies are not
  independently claimed green or V1-approved. Do not restore an older root over
  newer secure-field/PlainBox/SubmitPress changes.
- Explicit FieldId/engine-owned IME commit payload is now **G2's sole ownership**,
  after secure-field lands (coordinator b5b84d04). K provides Window-aware Change,
  current-text blur and existing `composing(window,cx)`, **not** a commit reason
  or originating-FieldId payload. No second listener, IME cache or string-diff
  inference. C2 was told this exact boundary.
- W13 source-only **85fea32a6** uses existing `ContentWidth::points()` for a
  centred, optionally capped Markdown document column. Its receiving strict/UI
  is pending; no C5 file is copied here. F5 keeps that already-existing API;
  default becomes820 and numeric Pixels600..4000 extend it. Shared Markdown
  typography follows kit on integration.
- Contributed themes remain a stated F6 dependency by coordinator ruling;
  typed prep retained at2c0ec0594 on `backend/m12-f6-appearance-prep`. No synthetic
  registry/unwired preference is exposed. F6 Hub calls the future
  `Shell::appearance_catalog_changed` outside paint; validate that path in F6.
- C2's later paint-cache/snapshot-face work is not adopted. Existing af31 union
  is200f4ba6c; no physical equal-metrics or performance improvement claim.

Next: finish only missing captures/proofs, inspect actual originals, create a
stable combined final manifest, then **one** V1 package submission with source,
gate and capture assignments. Coordinator opens captures, lead integrates and
owns the later combined gate. Do not announce full F5/Paseo completion before
that bounded review and integration disposition.
