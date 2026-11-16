# Shared locale number formatting

State: contract/skeleton; ADR 0101 precedes code. No new dependency or
formatter implementation has been adopted.

## Scope and structure

Follow Paseo 0.15.4 `usage/format.ts:185–199` for full-finite amounts with
typed units and provide a separate exact unsigned-count path. ADR 0101 owns
the range, rounding, data and measurement requirements. The shared view
kernel prepares display text; app consumers cache it at events and borrow it
while rendering. Locale data remains in translation folders.

The existing `Number`, `NUMBER`, grouping rules and `FiniteNumber` are the
starting assets. Their current float/`u32` limits do not close this package.
The backend remains undecided until measured; candidate experiments are not
an approved public API or consumer implementation.

## Sequence

1. Commit ADR/contract and reference findings.
2. Prove exact scalar/rounding/selector feasibility with maintained APIs.
3. Measure linked release/wasm cost for ICU and a narrower data-driven kernel;
   distinguish probe results from actual installed-app deltas.
4. Append the measured choice to ADR 0101, then implement one shared producer.
5. Check strict/view/wasm and give the checked interface to the Usage owner.
6. Record consumer integration, language-event preparation and visual evidence
   separately. No existing generator or General pass supplies those results.

## Resume

The generator package is V4 row 248 source/changed-View PASS at `c8290e56e`
with documentation leaf `bfc9f6f19`; the lead merged it at `f2169502` and it
is present in green package base 13 `2ebeedf9`. Its 1086 passing View tests
are historical generator evidence. No numeric-extension test has run yet.

This package starts on the base-13 tree. The reference oracle now retains
34 finite-amount and 13 exact unsigned cases; the stock-Fluent carrier
characterization tests are source ready, VM check owed. ADR 0101 records
the exact candidate dependency requests and the scratch measurement contract.
The coordinator requires its yes/no before lock changes; the tool owner is
preparing the missing offline experiment interface. No dependency, runtime
patch or product behavior has been adopted. The next action is that narrow
characterization check and scratch-only runtime adapter comparison, not
another run of the finished generator checks. All Rust compilation stays
on the build VM.
Stage 1 is now a controlled offline scratch graph, pinned at `d633cdd4d` with
code snapshot `bc948736b`. Run `i18n-number-experiment-20261116T014841Z-d633cdd4`
is queued; it resolves/audits the graph before strict and eight narrow probe
tests. The ICU comparison may follow only after that label is terminal.
It is not an installed-app measurement or product lock update.

The stock-carrier characterization's exact `08076af53` strict run
`i18n-narrow-20261116T004824Z-08076af5` passed, with actual View recompilation.
Its three runtime tests remain queued in
`i18n-narrow-20261116T014848Z-08076af5`; no execution result is borrowed.

Later unlinked sources prepare folder-based placement, whole amount patterns,
a single-locale narrow kernel, signed-zero keys and whole numeric-pattern
English fallback. They are outside the frozen stage-1 inputs and remain
uncompiled. Actual app callback overlays, the agreed native-workspace View
example `number_format_probe`, generated-message unsigned binding/fallback,
dependency size/time artifacts and coordinator adoption approval remain owed.

No own fleet/display is active. About dependency rows must preserve the voice
package's coupled audio entries when that accepted package reaches the line.

### 2026-11-16 carrier preparation and first controlled graph

Stock carrier controls exact08076af53: strict zero, runtime three passed,
zero failed/ignored; actual View recompiled. Narrow D633 feasibility: offline
metadata zero, graph-admission red, compile_attempted=false. Existing package
versions/checksums unchanged but four packages gained feature-unified edges;
T is examining the exact allowlist rather than weakening the guard. Both
receipts are preserved. No ICU retry yet and all release/wasm delta cells
remain unmeasured.

New unlinked option/decimal preparation and custom carrier controls retain
source/English numeric results, exact unsigned input and compact magnitude
through nested NUMBER. These remain source-only; the actual native callback,
generated binding, numeric-pattern failure boundary and fixed wasm example
overlays are still owed. Product source/manifests/locks are unchanged.

### Phase-2 overlay preparation

Prepared baseline/narrow/ICU overlays in experiment data only. Temporary-index
apply checks and rustfmt succeeded; product runtime/manifests/locks are still
byte-identical to base13. Normal candidate sources share numeric arithmetic,
options, typed source/English carrier and fallback, with backend-specific
placement/data renderers. The candidate standalone example exercises actual
generated Number arguments, all three full-finite unit policies and exact
unsigned values. No black-box-only engine or unused dependency is presented
as an installed-size measurement. Neither candidate has compiled yet; source
review, strict/runtime, full release/wasm costs and startup remain owed.

T's fixed six-edge lock admission correction preserves the original D633
compile0 red. One authorized same-source replacement was submitted:
`i18n-number-experiment-20261116T032004Z-d633cdd4`, ordinary, currently queued.
This is not a second live duplicate. Serde runtime activation is not inferred
from those weak/optional lock edges. Stock080 tests remain3/0/0, separately.

## Resume here — 2026-11-16 quota stop

Read this before any code change or VM submission. This package is **WIP,
not reviewed/gated/adopted**. Worktree `/home/dev/vessel-m12-i18n`, branch
`backend/m12-i18n-fluent-experiment`. Product root/native manifests/locks and
View/App/Kit/Link runtime source remain identical to experiment base13
`2ebeedf964ead0b490f9a0dc4af1f29d88a2252e`. Do not merge newer app bases into
this controlled cost comparison: its pinned T contract requires base13.
Port the eventual approved producer onto the then-announced app line afterward
(the last brief read announced67637eb6); this is not latest-combined evidence.

Committed source checkpoints:

- `fa9858287`: unlinked custom numeric carrier and checked options; records
  stock controls and original scratch graph refusal.
- `62e00d20252b45053a7c4448b76b8d97e5d53a61`: baseline/narrow/ICU scratch
  overlays and actual typed amount/UInt -> generated getter -> NUMBER sources.
  No product runtime files were installed. Temporary-index apply/rustfmt only.
- `001420839c227d8af45d150522986743acef0b51`: READY **baseline-only** measurement
  packet. `measurement-inputs.json` still describes existing/runtime unchanged,
  not either candidate. Never blindly submit HEAD as a narrow packet.
- `ae7e3cfb0`: v2 oracle52 finite+13 unsigned rows, with prior outputs retained;
  both-memoizer API controls registered in candidate View overlay. Default Node
  oracle reproduces v1 byte for byte. Actual-boundary tests use v2, frozen
  D633 source uses v1. No candidate Rust test has executed.
- `5a130de20`: WIP records the uncompiled numeric-literal normalization edit
  and the exact stop boundary. Do not treat source-only preflight as Clippy.

Verified, separate evidence:

- Stock rejected Fluent carrier exact08076af53: strict0, runtime3/0/0 with real
  View compilation. Logs/verified JSON under
  `/tmp/work/m12-i18n/number-carrier-08076af53-20261116/`.
- Original `i18n-number-experiment-20261116T014841Z-d633cdd4`: offline metadata0,
  graph-admissionRED, compile_attempted=false. Six exact weak/optional lock
  edges were subsequently admitted by T, without enabling serde runtime or
  accepting version/checksum changes. Preserve original logs/locks/red receipt.
- Only Node oracle/reference checks, rustfmt, source preflight, temporary-index
  overlay apply checks, size/tokens checks passed locally. No local Cargo or
  rustc was run. New numeric runtime/gates/markers are all unverified.

Already registered VM runs — query status and attach; **do not resubmit**:

1. `i18n-number-experiment-20261116T032004Z-d633cdd4`, exactD633 narrow test,
   the ONE authorized replacement after T's fixed graph audit. Out
   `/tmp/work/m12-i18n/number-feasibility-narrow-d633-policy2-20261116`.
2. `i18n-number-measure-20261116T032623Z-00142083`, exact001420 existing/native
   shipping release measurement. Out
   `/tmp/work/m12-i18n/number-measure-existing-001420839-20261116/native`.

Both were still queued in the03:39 VM status snapshot. No wasm, ICU,
candidate-validation or candidate release run has been submitted. The local
controllers/attach groups615839 and716523 were verified owned UID1000 and
terminated, with no non-zombie remainder; detached VM runs were not cancelled.
The baseline controller will NOT submit wasm later. Do not restart either
controller script: it would resubmit. Use direct attach to each existing ID.
Cleanup/status evidence:
`/tmp/work/m12-i18n/quota-stop-processes-20261116.json` and
`quota-stop-vm-status-20261116.txt`. No own fake fleet/app/virtual display exists.

Next, only after quota permits work:

1. Read finished results/receipts/SHA/actual compiler/test names for the two
   existing IDs. A queued/running/done run is never a reason to resubmit it.
2. Prepare an immutable CANDIDATE measurement-inputs packet (variant narrow,
   five-file hook selector, all exact source/overlay/fixture SHA and authored
   reachability evidence). Existing baseline packet is not a candidate packet.
3. T228fa193 installed bundle68bbd5bb under coordinator64f97 approval:
   `vessel-remote-gate run --tree /home/dev/vessel-m12-i18n --commit FULL_READY_CANDIDATE_PACKET --label i18n-number-validate --out NEW --no-wait --number-validate narrow`.
   Fixed ordinary View lib/tests Clippy-Dwarnings then unfiltered View lib
   tests; no release/size claim. Submit once; ICU sequentially only after the
   previous variant is terminal. It was NOT run before this stop. Read T's
   managed number-measure.README.md first. No priority/arbitrary Cargo/lock.
4. Fix actual strict/runtime findings on a new meaningful source. Potential
   source concerns still require execution: ICU non-selected placement helper
   liveness, numeric literal/option equality, both memoizers, source/fallback
   compact exponent handling, actual generated argument/whole sentence goldens.
5. After qualification, complete actual existing wasm and both candidate
   native/wasm measurements in the controlled sequential label. Account for
   warm/cold reuse and release compiler/link CPU vs wall; unmatched conditions
   are not an honest compile-time delta. No size/first-frame values exist yet.
6. Send ADR0101 diff + exact dependency/license/features and measured artifact
   deltas/compile costs + Fluent patch vs cheapest exact alternative to the
   coordinator for yes/no BEFORE product manifest/Cargo.lock changes. Then
   port approved source, inventory/dependency checks, all required gates/review
   and W5 consumer/cache integration. No producer hash is ready for W5.

Waiting peers: W5 needs full-finite amount+unit and exact u64 producer; it keeps
raw values, never clamps/casts/drops or adds an area formatter. T awaits the
candidate packet if work resumes; it does not submit for L. Coordinator/lead
have no adoption/PASS to merge. No numeric candidate package is in review.
Older generator row248/General row247 PASS remains separate and unchanged.
W10 added image wording exactea070ac65 received WORD/KEY/CONTEXT acceptance only
(private evidence JSON), never runtime/image-acquisition/UI approval.
