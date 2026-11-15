# F1 node extension: per-run schedule isolation

Assigned base: Quality B29eebb8b (protocol23, store18). This is the separately
owned node worktree; the app remains on its assigned package base. No product
change, pending feature registration, schema allocation or working isolation
implementation is asserted by this design checkpoint.

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

W11's staged702277bd is not yet a reviewed consumer-ready contract. It defines
WorkspaceId/registry shapes and explicit membership, not cwd ownership. The
local/worktree provision+store producer is W11, creation receipts W3, archive
W21. None is merged here solely because its types exist on a staged branch.
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
