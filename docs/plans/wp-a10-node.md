# A10: named launch profiles, node/client package

## Scope and status

Specification: the Paseo 0.15.4 survey's Host agent profiles, Applying
profiles to drafts/existing agents, and Legacy favorites migration
sections, checked against the original files named in ADR 0076.

The initial CRUD contract passed review. The expanded contract adds live
application and migration and passed contract-only review in `d616e8f9`.
Core value source `9375732b` has TS-enabled proto Clippy and 275/0 library
tests, including nine new value cases. That is a checked partial stage,
not this package's skeleton, completion, product review or whole gate.
Results are in `backend-m12-launch-profiles.md` and checkpoint `82776280`.
The original failed Clippy log remains retained.

The isolated node package contains the five pending rows, validated values,
store CAS, retained node jobs/probes, shared agent mutation/read owner,
session adapters, typed client/materialization/migration helpers, CLI and
shared fake inputs. New runtime tests are written but have not yet executed
successfully. Selected compilation and lint correction remain in progress;
generated TypeScript, the complete package gate and implementation review
remain outstanding. App screens and the MCP facade belong to their consumer
packages. These source additions are not a completion or runtime claim.

The announced node baseline `d432ef0f` is the package's direct base and
contains Quality B's canonical table/accessor. The store migration is the
allocated schema 19, expressed relative to title-origin schema 18. All five
rows use the pending named feature; its protocol remains None until the
integrator assigns it. Pending refusal and assigned happy-wire evidence
are deliberately separate obligations.

## Pending consumer contract

All types below live in `vessel_proto::launch_profiles`; methods live on
`vessel_client::NodeClient`. The single feature remains None, so every call
currently refuses locally and on the server. This table describes source
shape, not assigned availability or runtime approval.

| Call | Request | Result | Scope / additional authority | Client method |
| --- | --- | --- | --- | --- |
| `launch_profile.list` | `ListLaunchProfiles {}` | `LaunchProfilesSnapshot` | Read; public notes are readable by agents | `launch_profiles()` |
| `launch_profile.replace` | `ReplaceLaunchProfiles { expected, profiles }` | `LaunchProfilesSnapshot` | Admin and human origin, independently | `replace_launch_profiles(expected, profiles)` |
| `launch_profile.options` | `GetLaunchProfileOptions { selection }` | `LaunchCatalogue` | Read and human origin, independently | `launch_profile_options(selection)` |
| `launch_profile.applicable` | `GetApplicableLaunchProfiles { session }` | `ApplicableLaunchProfiles` | Read; filtered metadata grants no write/apply authority | `applicable_launch_profiles(session)` |
| `session.apply_options` | `ApplySessionOptions { session, options }` | `ApplySessionOptionsResult` | Operate and normal session ownership | `apply_session_options(session, options)` |

Snapshots carry `revision` and ordered `profiles`. Replacement is whole-list
CAS; omission clears optional record fields, and stale revision is checked
before identical-content no-op. A catalogue echoes the full `selection`
key and reports canonical public `account`, current `options` and
`probed_at`; it contains no credential values. Consumers discard stale keys.
Live results carry `applied`, optional first `notice` and optional first
`failure { choice, error }`. Caller timeout or disconnect never establishes
rollback or permits blind replay. Typed launch refusal details use
`ErrorDetail::LaunchProfile { problem }`; ordinary scope/ownership and
provider failures retain their existing codes and causes.

The internal `AgentInput::options_fenced()` and
`SessionHandle::options_fenced()` APIs order a native current-options read
after prior mutations. They add no RPC and do not provide an atomic
session/account/run snapshot. Consumers must independently bind the actual
process lifetime before and after the fence. The app owns migration marker
persistence; the MCP package owns its facade. Neither may substitute the
existing account-profile calls for these named launch presets.

## Existing owners and structure

| Owner | Reuse/extension |
| --- | --- |
| `vessel-proto::launch_profiles` | Validated core values, catalogue descriptors, typed state/outcome/notice/legacy inputs; request/reply shapes only when table registration is admitted |
| `vessel-store` | One ordered-list row and revision through its current reader, per-operation savepoint and commit-before-answer writer; no second file/cache |
| `vessel-node` | Scope/origin/ownership checks, finite metadata/probe/apply admission, retained account snapshot and routing |
| `vessel-session` | Existing wake/move/ended/terminal boundary and one captured process input handle |
| `vessel-agent` common input boundary | Shared bounded owner for all single and bundled configuration mutations; actual driver completion, partial prefix, notice and process closure |
| `vessel-client` | Typed calls, pure draft/live materialization and migration planning; ordinary host CAS, no local credential/preference access |
| CLI / MCP / testkit | Same typed calls and fixtures; Read-agent list notes, owned-session apply rules, no real agent prompt |
| App preference owner / G3 | Validated local compatibility input, per-node marker/in-flight state, remembered concrete choices and screens; separate app package |

Ownership of the shared agent/session boundary is explicitly coordinated.
Completion adapters belong to this package; driver parsers, vendor policy,
accounts and offered choices remain with their existing owner.
No client-loop approximation of server-side mutation serialization.

## Skeleton states, then implementation

The node skeleton consists of compiled typed state/result definitions and
staged private service/owner seams, tested through the actual owners. It
is not a visible control returning a placeholder success. Proposed states:

- Profile read: unassigned/unsupported, unread, unavailable, complete empty
  or ordered snapshot. Editing: loading, draft, saving, stale revision,
  invalid values, retryable storage failure, committed current state.
- Catalogue: no request, resolving, resolved, explicit unavailable/error,
  configuration changed. Stale selection results never replace a newer key.
- Live preparation: awaiting live options, inapplicable provider/account,
  ready ordered choices with omitted stale modes. Creation remains strict.
- Apply owner: queued, running one actual step, completed, first refusal
  with applied prefix, acknowledgement unknown while fenced, process closed.
  A notice does not short-circuit following steps. No rollback claim.
- Migration: unsupported, marker complete, no favorites, resolving catalogue,
  ready append-only plan, applying, retryable/conflict/unresolved failure,
  complete. Marker storage is app-owned and follows host success.

After expanded contract review and ownership coordination, implement the
state/type skeleton, then storage/probe, shared mutation owner and service,
then table/accessor/client/CLI/MCP/fixture wiring after Quality B. Use one
commit per sub-feature with narrow checks. No public/dead-code/lint escape
for a staged service. The whole root gate and review occur once at the end
of the complete package; no whole gate per type/contract commit.

## Reference behavior and differences to keep explicit

Keep stable ids, exact whole-list replacement/clears/reorder, public notes,
registry glyph/color fallback, host-wide catalogue/defaults, delayed/stale
request isolation, copy-to-draft with no live id binding, immutable live
provider/account, model/mode/thinking/features order, first notice and
partial failure with no mutation interleaving. Invalid saved mode is
omitted only for live profile reconciliation; a deliberate create/explicit
option request is still validated by its normal driver rule.

Keep legacy per-host in-flight/complete behavior, pair/id/selection dedup,
existing-first append, catalogue label/disambiguation/suffix naming, bounded
loading retries and marker only after success. There is currently no
Vessel favorite schema to reinterpret: existing remembered choices are not
favorites. The app owns compatibility data and markers; the node cannot
scan another application's home. UUID migration identity, strict bounds,
CAS reconciliation and typed uncertainty are the recorded differences.

## Verification and handoff

Tests must fail without each invariant: private values/escaped byte cap,
maximum revision, strict invalid later record; two writers/no-op/stale and
failed transaction/restart; scopes and delegated/agent origins; real fake
ACP/direct probe choices, account reload, cancellation and cleanup; shared
single/bundle mutation fence, first error/prefix, first notice, hung reply,
process replacement and retained external handles; source draft/live and
legacy migration cases, no-favorites/no-catalog, failure/retry/dedup/marker
ordering and virtual backoff. Safety waits use within and named phases;
written executable fixtures use the shared executable helper.

Pending-feature refusal is distinct from assigned happy-wire evidence.
No placeholder version is invented for a test. Every old/direct/nested
transport path, generated TypeScript, actual CLI/MCP and fake-fleet calls
are verified at package close. Report that gate by exact run/commit and
all return codes/totals, and list anything only a real provider/OS can
prove. G3 receives only the reviewed, assigned exact API after integration.
No native/page screen, real account/agent, production service, preview
pairing, push or permission mutation belongs to this node package.

## Storage and common-owner implementation checkpoint

The next source stage adds `vessel-store::launch_profiles` and its async
Store API, one predecessor-relative schema append, and the existing writer's
typed commit-delayed answer. Tests cover stale-before-equal, revision
exhaustion/no-op, strict whole decoding, a savepoint rollback, two real
writer CAS requests, clear/order/restart and a deferred-FK outer-commit
failure. The store's new testkit dev edge supplies the existing finite
safety clock and introduces no product dependency.

The common agent owner routes both the old setter and the new bounded
bundle. Driver changes only adapt the completion carrier; no provider
parser, offered choice, account selection or launch argument changes.
Barrier tests cover completion order with an old setter, first notice,
partial prefix, caller cancellation and closure with retained handles.
The session handle captures one process after its existing wake boundary.
These source additions have format, size and crate-boundary checks only
at this checkpoint; selected compile/runtime checks and package wiring
remain outstanding. There is still no callable launch-profile method.

The next in-progress stage adds bounded catalogue values/category hints,
all five real pending table rows through Quality B, retained metadata and
apply jobs, and service routing to the actual store/runtime. Editor probes
apply their draft choices in the normal handshake and stop/reap on node
shutdown before returning. A retained account snapshot is checked again
before publication. Eligibility reads metadata only and never start a
process. Applicable rows with a removed/mismatched saved account are not
returned; the editable host list retains them unchanged.

The first selected check of `06e8e8a5` failed on unused SQL/pop results,
document formatting and channel-closure mapping style. Those diagnostics
are retained and corrected in source; no tests ran in that Clippy check.
The shared node assembly and RPC binding definitions were divided by their
existing responsibilities to preserve the size ratchet, without adding a
second method registry. Current generated TypeScript, consumer wiring and
new-stage compile/runtime verification remain outstanding.

The client stage adds five thin typed methods, an explicit migration-support
projection from the CRUD/apply table requirements, pure strict draft and
live reconciliation helpers, and the pinned reference's separate live-call
budget. Its catalogue includes nonsecret canonical account/default facts so
migration never guesses default-name aliases. Tests cover stale mode
omission versus unknown deferral, actual category order, untouched model/
feature refusals, draft copying and finite caller-budget behavior. The
client's testkit dev edge reuses the required safety-net clock.

Selected Clippy on `53680637` stopped on five proto diagnostics (two public
constructor error docs, one qualification, one boolean simplification and
a repeated fixture arm). Those are corrected; no lint/deadline/assertion is
relaxed. Actual selected runtime tests, client/migration/CLI/MCP completion,
generated wire and the one final package gate remain outstanding.

MCP facade ownership is coordinated with its package owner: this node/client
package does not edit that crate concurrently. MCP wrappers receive only
the reviewed, assigned integration handoff; their execution evidence is a
separate consumer obligation. Existing account-profile/harness-options
wrappers never impersonate named launch presets.

Legacy input values and deterministic identity are now staged in source:
exact bounded provider/model keys, a count-bounded input list retaining
source duplicates, and a fixed-namespace canonical typed-pair UUID id.
The existing `uuid` dependency gains `v5`; lockfile addition `sha1_smol`
is confined to metadata identity, with a golden-id fixture and explicit
rationale in ADRs 0001/0076. No other identifier or credential path changes.
The first client-stage selected Clippy stopped on the canonical account
getter colliding with the TS trait's `name`; it is renamed `account_name`.
The raw failure remains preserved. Pure migration planning/application,
actual runtime checks and the package gate remain incomplete.

The shared legacy helper is implemented in source with explicit provider-to-
node catalogue mappings, actual node identity checks, pair/id/model-selection
dedup, source name/suffix order, and existing-first whole-list CAS. Only
explicit loading/busy states use the six source delays plus final read.
Stale conflicts rebuild from a fresh list within a finite budget. Unknown
write acknowledgements retain their cause and can complete only after a
read proves all imports present; they are never blindly replayed. A typed
receipt identifies the actual node for the app-owned completion marker.

Planner and driven-clock callback fixtures cover existing records, explicit
default account equivalence, names, unknown model ids, wrong-machine data,
conflicts/manual edits, failed/unknown acknowledgements, wrong reply state,
and exact loading/backoff behavior. They do not prove assigned wire traffic
or native marker persistence. Runtime execution remains outstanding.

Selected type check on `e071c09d2` reached the consumer crates and found a
missing client-test CallError import and a fixture using String instead of
the existing ModelId type; those and the agent Future qualification warning
are corrected. The new node head is consumed next at this clean boundary;
its canonical method table and startup extraction take precedence over the
working branch's earlier mechanical file divisions.

## CLI and real-node fixture source checkpoint

The announced gate-40 node baseline is merged. Incoming rpc/contract.rs and
rpc/table.rs remain canonical; the five pending rows are at the table end.
Client materialization and legacy planning/reconciliation source, plus all
five CLI operations, are present but selected compilation/runtime checks
remain in progress. CLI input uses the protocol whole-body byte limit and
typed request decoding; CAS/lost-ack retry is never hidden.

Private generated-guard fixtures use real nodes, actual handshake contexts
and fake ACP processes to cover Read/Admin/origin/ownership, public notes,
missing-account preservation, selected probe choices, failed live prefixes,
reload before publication and caller cancellation through actual node stop.
A loopback-only finite fake startup barrier fixes those orderings. Raw real
server fixtures exercise all five calls directly and under Forward/Relay/
Deliver at pending/older admission. They are not assigned happy-wire proof.
MCP facade and app marker persistence remain consumer-owner obligations.
No new runtime result is claimed by this source checkpoint.

Shared testkit authority fixtures now include public typed whole-list and
selected-probe JSON files for the fake ACP ids. Real-node tests decode and
use the same files, including applicability against the running account.
TS primitive aliases are explicitly classified before regeneration; this
source edit is not generated-TypeScript evidence.

An actual CLI binary fixture covers each typed command against a disposable
fake node while the feature is pending. It owns each child, limits captured
output, and bounds completion, kill/reap and reader drain separately. The
pending-only assertion deliberately does not substitute for numbered
happy-path integration evidence.

A shutdown audit closes the synchronous preflight-to-spawn gap: admission
now takes a task-tracker token before checking cancellation. A deterministic
fixture retains that exact production admission while closing the tracker,
proves drain cannot complete, then releases it. Completion-channel permits
remain independent so finished replies stay bounded without waiting for a
caller to poll before considering actual work drained. The next baseline
merge retains ProcessLifetime and transient SystemPromptOverride with None
in ordinary probe setups. Shared JSON fixtures use canonical wire harness
names rather than CLI aliases.

The common owner now also implements the coordinated internal read fence
requested by the provider-create consumer. AgentInput::options_fenced and
SessionHandle::options_fenced issue a real Command::Options after earlier
admitted mutations; normal options scheduling remains unchanged. Barrier
fixtures distinguish the real read answer from empty setter replies and
cover cancellation/driver closure. The session-actor consumer separately
owns authenticated parent context/lifetime binding; no atomic session
snapshot, revision or inheritance authorization is claimed here.

The fake-fleet helper copies the same two testkit JSON inputs into its
private scratch fixture directory and checks all five pending CLI refusals.
It labels this as negative availability proof only, with no populated UI,
numbered happy-path or permission claim. At assignment those negative
expectations must be replaced by integrated positive fixtures, alongside
the pending-only Rust/CLI tests. Shared native changes are confined to the
existing wire-gap fixture helper and its README explanation, with no screen
or app-control source changes.

CLI decode failures retain the original typed parser cause but expose only
line/column in output and Debug. The command boundary stops cause-chain
rendering for that one input error because serde diagnostics can quote
untrusted values or field names; a synthetic-marker regression covers the
actual CLI formatter. Other command errors keep their existing rendering.

The raw admission fixture now sends a nonempty shared CAS payload, then
checks the actual store remains at revision zero. An accidentally dispatched
write cannot pass merely because reply adaptation also refuses it. This
strengthens the pre-dispatch assertion without changing any deadline.

Current-thread barrier regressions also fix ready-ack versus driver-close
ordering. An available setter completion/first notice remains in the
prefix, the next step refuses Closed, and a ready native read is consumed
before concomitant closure. No acknowledgement is fabricated or work
rolled back; unanswered replies still close and drain as before.

## Integration package isolation

This review package is rooted at the announced node baseline `d432ef0f`.
It carries only the named launch-profile implementation and its shared
fixture delta. The separately staged project-pull service and unrelated
native ancestry are excluded. Source-stage histories remain preserved;
this packaging does not authorize that separate service for the node head.
The assigned-protocol integration and consumer screens remain separate.

The isolated package additionally checks the retained NodeProfiles identity
before publishing probe failures as well as successful catalogues. The
real startup-barrier reload fixture covers both default choices and a fake
agent's actual rejected locked mode; both must return configuration-changed
and a fresh probe must recover. Existing phase/outer deadlines are retained.
