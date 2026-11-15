# F1 node extension: per-run schedule isolation

Current assigned node base: 275024369 (gate54, protocol25/MIN2/schema21),
merged at926f14c54. The Ended-edit source was checked on9c150c6e; those
results remain pinned to that older producer. The older gate40 section below is
historical. No protocol/schema number is allocated or changed by this slice.
The isolated known-Ended update change below has bounded V3 SOURCE/NARROW
PASS (row260); its combined integration gate remains pending. Per-run
isolation is still a separate unimplemented dependency on
canonical provision/receipt/archive runtime APIs. The app stays on its own
announced package line and does not merge this whole node branch.

## Existing assets and constraints

ADR0033 already owns scheduling/catch-up/authority/overlap/history. Its explicit
old gap is a new worktree per run. `service/scheduled.rs::make_run` reconstructs
the definer, checks the existing place, creates through create_as, then queues
the prompt. `service/followed.rs` follows its first turn and uses put_away for
the default archive policy. `automation/schedules` records making before
spawning, serializes each schedule and recovers interrupted runs. Those paths
must remain the authority; this extension does not bypass checks or create a
second scheduler/cron evaluator.

Paseo `server/schedule/service.ts::executeSchedule`927–1060 and
`createScheduleRunWorkspace`1062–1077 create a local directory workspace by
default or a new worktree workspace on every run, record workspace identity
before creating the agent, then archive on finish by default. A workspace
created without a successful agent is archived even if Keep was selected.
This behaviour is in scope, not an excused difference.

[Decision Log]
- Purpose: give each automatic run the isolation the person selected, with
  durable cleanup ownership across failures/restarts.
- Constraints: one workspace identity contract (W11 ADR0082); existing worktree
  creation/trust/path checks and W21 archive path; definer authority rebuilt at
  execution; one typed wire field requirement; no guessed protocol/schema.
- Alternatives: infer workspace ownership from cwd, a schedule-local workspace
  ledger, creating Git worktrees in the app, or silently ignore a newer option.
- Intended choice: optional typed isolation on NewSession, legacy absence keeps
  existing behaviour; explicit local/worktree uses the shared provision path.
  Store the resulting WorkspaceId before agent creation and use shared archive
  on finish/failure/recovery. No implementation is frozen until those producer
  APIs are reviewed and available.
- Why and cost: workspace/session membership is explicit rather than inferred,
  and cleanup cannot delete another run's work. Additional durable state may
  require a real SQL migration, assigned/repointed by the node integrator.

## Wire admission and old clients

Use the existing Call::since target field rule in rpc/since.rs and ONE named
Option<ProtocolVersion> constant, None until the wiring merge. Calls containing
the field are refused before sending and dispatch while pending; ordinary
schedule.create/update retain11. Forward/Relay/Deliver inherit the inner field
requirement. No capability call or second method table is added.

Old replies/listings omit the field using the existing older-peer projection.
An older connection updating a stored schedule carrying the field must get a
typed Conflict asking for a newer client, rather than erasing isolation. The
stored definition/revision and any pending run must be unchanged on refusal.
The exact typed detail and storage column depend on the final shared producer
contracts; this document does not manufacture either one.

## Dependencies and tests still owed

WorkspaceId and WorkspaceOperationId identity values are now in the assigned
node base. W11's registry/provision c717bad1, W3's durable creation receipt
storage0b112311 and W21's archive port afdcb766 were reported as staged, not
reviewed product/runtime adapters. Their passing storage/fake tests do not
provide a driver. Local/worktree provision is W11, creation receipts/driver W3,
archive W21; none of those moving branches is merged merely for staged types.
The app isolation option remains hidden until the implementation is admitted.

Required regressions: field absent vs pending admission and each envelope;
old-update erasure refusal; old projection; local/worktree per-run identities;
run workspace recorded before agent start; creation failure and prompt failure;
Archive default and Keep; cleanup/restart at each durable boundary; no other
workspace/session affected; revoked definer and untrusted project refusal.
Use existing fake harnesses and worktree fixtures, never real agents. Test the
real actor/client and platform-specific refusal paths. Changed-crate narrow
checks precede the final one node package gate and V3 review; none has run for
an isolation implementation at this checkpoint.

## Gate40 registration boundary

RPC rows now live in rpc/table.rs (new methods append at its end), Request and
UnexpectedReply in rpc/contract.rs, boot wiring in node/setup.rs. Isolation
extends existing schedule.create/update payloads, so its admission belongs in
the existing target_since field rule; no extra RPC row/capability is invented.
New project/worktree/git/file methods owned by other packages use the explicit
WORKSPACE_FAMILIES_LATER test inventory. Schema18 is inherited and unchanged;
request a real migration step from the integrator only after durable run IDs
and shared driver storage are fixed. The known c5da5305 root e2e race fix is
already inherited, so no duplicate patch or test edit is needed.

This merge/design note contains no isolation product implementation or test
claim. Shared native button/menu producer handoffs remain the current priority;
app isolation stays hidden until the node runtime and its review are available.


## Known Ended definition edits (coordinator ba9f75ad)

[Decision Log]
- Purpose: pinned Paseo schedule-row.tsx209–250 opens editing for completed
  records; schedule-form-model.ts545–565 has no completed veto; server
  schedule/service.ts458–509 updates definitions without reactivating completed.
- Existing constraint: Actor::update already preserves Ended in its state
  match, but the earlier shared ended_refused call prevented reaching it.
  That helper remains the correct pause/resume/run-now admission.
- Choice: separate known-state update admission, allowing Active/Paused/Ended
  and refusing Unknown. All original Asker ownership/forwarded-call/revision
  checks remain before changing data; the actor still serializes each update.
- Why: no second update path, no new wire/schema/method/number. Existing
  ended reason/time must also survive edits of max_runs below the past count,
  so only non-Ended states undergo the existing max-run completion transition.
- Impact: the source comment is corrected. Tests seed the REAL scheduler/store
  and cover Ended edit with exact end reason/time/count/store receipt preserved,
  foreign-agent and forwarded update refusal, stale-revision replay refusal;
  Unknown unchanged; Active unchanged cadence preserves next_due. No fake job
  may be queued. Existing bounded harness guard and cancellation-on-drop apply.
- Proof: authored tests/static format/size only at this checkpoint; actual
  narrow commands/V3/integrator merge are owed. App Ended admission stays
  unchanged until this node behavior is reviewed/available. Old nodes may
  still refuse with their real typed Conflict; writes are never auto-replayed.


Known-state admission also refuses Ended{reason:Unknown}: re-encoding an
unrecognized future reason could discard its meaning. The same real actor/
store refusal test covers both unknown outer state and unknown end reason.
Only the four existing known reasons become editable, all preserved exactly.
The queued25e source was superseded only if queued under standing WORKER
approval; retain its143 record and never call it a compiler result.


### Original-state guard precision

Before an actual check, source audit found the initial guard read the state
AFTER Active's advance/expiry transition. That could change the pre-existing
max-runs reason precedence for a formerly Active record. It now captures
was_ended BEFORE the state transition. Only originally completed records retain
their old reason/time; the original Active update and max-runs ordering stays
unchanged. A bounded real scheduler/store regression uses unchanged cadence,
an earlier new expiry and an already reached run cap to assert the original
MaxRuns reason. This is a source-found correction, not an executed failure.
The older265 queued proof must not be relabelled as this source.


### Completed expiry validation

A second source audit traced settle's future-run requirement: a genuinely
expired definition has ends_at before now, so admitting its Ended state alone
still refused editing. The new typed CompletedEdit validation uses the SAME
settle_for implementation and keeps cadence/zone/unknown-target/policy checks;
only its requirement that expiry follows a new future run is inapplicable to an
already completed definition. All ordinary create/Active/Paused validation stays
FutureRun. State never reactivates. The real scheduler fixture now carries its
past expiry and asserts it is retained, rather than using an Expired marker
without an expiry. No wire/schema/number change. Source-found before execution;
prior queued runs must stay pinned to their older source.

The same validator test explicitly contrasts ordinary future-window rejection
with completed-edit acceptance of the identical expiry, then refuses unknown
catch-up, unknown target and unavailable zone under CompletedEdit. Existing
ordinary validation cases remain unchanged. All are authored, not executed.


## V3 bounded source disposition (2026-11-15)

V3 ca067547 reviewed exact9c150c6e..d26b0b775, all eight changed paths, the
full actor/validator/state changes, original Asker rules and pinned Paseo
service458–509/row209–250/form-model545–565: SOURCE PASSED, GATE OWED.
The four real scheduler/Store and one validator cases are authored source,
not executed proof. Exact ordinary SHORT schedules-node-20261115T165717Z-d26b0b77
strict plus automation::schedules tests remain queued/pending. Earlier265/6e
queued-only143 records remain non-compiler evidence. No unchanged whole was
requested or inferred. Integrator/lead adoption and App Ended permission
changes wait for an actually checked producer. Per-run isolation stays excluded
from this bounded correction and remains an unimplemented runtime dependency.
No service/permission or scope/method/schema/protocol changes were made.


## Exact corrected narrow outcomes

Source d26b0b775 on assigned9c150c6e (V3 ca067547 SOURCE PASSED):
- schedules-node-20261115T165717Z-d26b0b77: --locked vessel-node all-targets
  Clippy -D warnings0, actual changed Node compilation, cargo step0/no skip.
- schedules-node-20261115T172743Z-d26b0b77: --locked vessel-node --lib filter
  automation::schedules,41 passed/0 failed/0 ignored (266 filtered). All four
  new actor/Store and the completed-expiry validator case explicitly ran ok.
  Required web-ci/nested-fakes/nested-vesseld/cargo steps all0/no skip.
- Exact specs/checkout and all step digest prefixes were checked, immutable
  run files pinned in the private evidence packet. This is author-verified
  narrow evidence submitted to the SAME V3 row, not yet a V3 gate disposition.

Earlier265/6e cancellations remain queued-only143, not failures or passes.
This docs-only receipt changes no d26 Rust and requires no unchanged rerun.
No whole root/native, App/consumer UI, new RPC authority, service-account or
per-run isolation/runtime adoption is claimed. Integrator handoff/App Ended
permission still await the actual qualified producer disposition.


## Qualified producer and integration status

V3 same-row final disposition (2026-11-15 17:56 UTC, row260) closes the narrow
condition for exact d26b0b775: strict Clippy0 plus41/0/0 scheduler tests, all five
new cases executed, all27 pinned run-file hashes independently matched. The
docs-only receipt059f1dab changes no product source. This is bounded dependency
PASS, not a whole-node, service, App or per-run-isolation verdict.

The node integrator merged059f1dab exactly into batch17 trial b68ae267b without
conflicts. Batch17 waits for batch16/gate54 and then its own gate. The trial is
not an announced green node/package base: the App retains its current Ended
refusal until the actual combined head is announced. No unchanged producer
gate was requested or repeated.


Node integrator announced275024369 after gate54 (Rust2635/0/12, web464,
e2e80, forge4; wire diff0). It merged conflict-free into this separate node
worktree. Ended-edit product bytes under automation/schedules still match d26;
that byte match is not a new combined gate. Batch17 now gates on e0eb0a55 and
contains the Ended producer, W19 fence, G4 lifetime and W3 schema-sequence move.
No App node-head merge or Ended permission change was made. Future pending
method rows append after skills.import_selection; no feature number is guessed.
