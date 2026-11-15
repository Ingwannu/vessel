# F1: schedules and recurring prompts

## Scope and dependencies

F1 is the schedules and heartbeats part of survey `.area6-parts/A.md`, through
its Heartbeats section. Saved prompts are beyond Paseo and follow F1. The existing
node schedule/store/CLI contract remains the base; a catalog entry is not a claim
that its native screen is built. Native View decides, Link calls, App draws, and
Kit owns shared controls. The screen must never contain fixture schedules.

Assigned app base8d5513e4 is merged at3b00f225. Separate node work starts from
announced Quality B29eebb8b (protocol23/store18); it is not yet merged into the
app line. The independent pending field uses one Option<ProtocolVersion> constant
set to None until the integrator assigns its number, not a capability RPC.

The list uses the existing pure Page model and correlated, bounded Link hub.
PreparedRow/Clock format authoritative node facts outside painting. UI state
owns popup focus/anchors but never infers a schedule's ended state from a partial
session directory. Creation, cadence preview, editing and run history use typed
requests with their own correlation; a saved editor uses the revision it opened.

[Decision Log]
- Purpose: ship the complete schedule flow rather than a polished list with
  unavailable forms. Preserve one owner for every shared control and node fact.
- Constraints: offline/reconnected machines and delayed writes must not produce
  invented success, lose a newer edit, or silently remove an unknown field.
- Alternatives: optimistic status, local cron projection, fixture-driven screen,
  per-area copies of selectors or notices, or a separate heartbeat subsystem.
- Choice: node-only states and next_due; existing schedule target distinguishes
  recurring prompts; shared selectors, sheet, menu, Alert and ActivityIndicator.
- Why: one schedule contract controls list, form, runs and CLI. Portable fixtures
  exercise pure rules without introducing any installable fixture path.
- Cost: an unavailable/older node requires a visible typed refusal; unsupported
  options remain hidden until their node implementation exists.

## Source map

Pinned Paseo 0.15.4 reference: schedules-screen.tsx (screen states, filters,
HostErrors), schedules-table.tsx (one card and confirmation), schedule-row.tsx
(title/target/metadata/badge/menu); schedule-form-sheet.tsx and cadence-editor.tsx
(form and adaptive footer), schedule-form-model.ts, schedule-cadence-options.ts,
schedule-cadence-policy.ts and their tests (form rules). Exact line ranges and
first-use evidence are recorded with the final screen packet. Reference paths
are relative to packages/app/src. Node execution follows
packages/server/src/server/schedule/service.ts::executeSchedule and its workspace
creation/archive path. Existing Vessel ADRs 0033/0034 remain authoritative until
an explicitly assigned extension changes them.

## Work remaining and honest states

- Existing source, final package verification still owed: View cadence/authority/
  history/editor rules; aggregate list model;
  bounded correlated list/pause/resume/run/delete calls; reconnect notice.
- Prepared but not yet a product screen: relative/calendar metadata and target
  presentation, list drawing and route/sidebar delegation.
- Next frame: editor/preview/history states and typed requests, fixture cases
  only in tests, then actual calls and a screen for every supported state.
- Node gap assigned to F1: per-run local/worktree isolation, using W11 workspace
  identity/provision and W21 archive contracts. App option is hidden until that
  node implementation exists. Quality B's field requirement gates it with one
  pending feature constant; old edits may not erase newer stored isolation.
- Shared pending: corrected HostErrors radius8 producer; HostFilter presentation
  and content-sized picker are committed6b431470 but narrow/runtime checks are
  still owed; menu close focus restoration remains open. Existing status
  dot6 is the explicitly allowed temporary value in a14-point slot pending E2.
- Saved prompts and webhooks are not part of this parity package.

## Deliberate differences requiring final verification

Node-authoritative state/unknown variants, exact interval remainders, typed
revision conflicts and nonoptimistic writes retain accepted Vessel contracts.
The recurring prompt is queued, never steered into an existing turn. Viewer and
rule zones are explicitly named; older nodes cannot promise preview support.
Node cron uses its existing OR rule/eight-year search and bounded history;
these established node contracts are not silently replaced by the client's
reference formatter. Run history is an explicit requested addition where Paseo
has no app screen; use the kit's existing rows and session navigation.

No deliberate-difference entry may excuse missing per-run worktree isolation.
Official agent marks use the reviewed shared producer only; fallback remains
until that dependency passes. Desktop sidebar toggle stays in the titlebar,
owned by the lead. Shared sidebar nav geometry is a staged seam and does not
claim full Sidebar parity.

## Verification and handover

Per-commit changed-crate clippy/tests only; one final whole native A13 gate and
node A12 gate for the complete package. Narrow green is never UI approval.
Capture actual node-backed empty/loading/failed/partial errors/read-only/long
names/many rows, creation/edit/preview/refusal and run links in EN/KO and both
themes at1280x900/390x844 (800x600 where layout matters). Inspect every original.
Fixture-only newer-variant probes, if used, remain an external uncommitted patch
identified in the evidence. Report works / does not / unverified, then V1 →
coordinator → lead. Tracker changes only F1/G2 rows and only when earned.

## Current immutable checkpoints

View presentation d282cf49, package-base integration3b00f225, shared selector
6b431470. Local exact-source retry never reached Cargo and was cancelled under
the explicit remote-migration ruling; it proves nothing. Remote changed-crate
checks are pending. No package state is advanced to built/passed from these
source checkpoints. Old narrow and whole results remain tied to their old
commits; see the separate list-time, registration and package-base plans.

The inherited generated web wire file contains trailing whitespace on the
assigned base. Its bytes are left as the lead's; diff-check for G2's native/docs
delta against8d is clean. This corrects the broad unqualified diff-check wording
in the integration checkpoint; no generated TypeScript was manually rewritten.

A minimal shared reconnect producer1af642c6 directly on8d passed V1 SOURCE
precheck only. It has an empty typed Event array after the unchanged kept-list
method. W20 may provisionally consume it without a menu dependency and adds its
own typed notice/tests. Its timing is before sessions query/client retention/
Fleet publication; the notice alone proves neither a retained connection nor
Read authority and does not retire stale tickets by itself. No compile/runtime
PASS is claimed for that isolated producer. Removal/rename/offline invalidation
is still an area's responsibility.

## Effective grants on the list

[Decision Log]
- Purpose: a device with an explicitly read-only grant must not first attempt a
  write merely to learn its scope; a device unable to inspect devices must still
  retain its valid operations.
- Existing seam: `device.self` reports this authenticated connection's effective
  scopes and does not require the unrelated device-list permission.
- Choice: retain its independently fallible result beside the schedule list,
  accept it only with the list ticket, and keep known evidence for the connection.
  Failure on an older node leaves authority unknown. A real per-row Forbidden
  outranks a later Operate report until a fresh connection resets the evidence.
- Alternative rejected: treating failure as an empty grant, or matching device
  labels in the administrative device list. Both can wrongly remove actions.
- Cost: an auxiliary self-query per list read. Narrow checks for this new delta
  are owed; the queued cb96 run is immutable earlier-source evidence only.
