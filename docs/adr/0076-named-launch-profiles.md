# ADR 0076: Named launch profiles

Status: initial contract accepted, 2026-11-15; implementation is not approved.
The A10 expansion below is proposed for contract/security review before code.
Core value definitions are being built; catalogue/storage/wiring remain
unbuilt. The integrator allocated the store migration; no migration or
protocol assignment is made here. No method in this document is available.

## Context and reference

A person repeatedly chooses the same agent, account, model, mode and
other options. A named launch profile keeps that selection once. It is
not an account profile: ADR 0011's `profiles.toml`, `profile.list` and
`session.move_profile` continue to mean account/environment selection.

The reference is Paseo 0.15.4, release source
`1c00b365c9e0b74c1580eba33d905d693125ae21`:

- `packages/protocol/src/agent-profile.ts`: required id, name and
  provider; optional icon, color, model, modeId, thinkingOptionId,
  featureValues and **notes**. No system prompt, deliberately: a
  creation-only prompt would silently fail when applied to a running
  agent. Its schema permits extra fields and unbounded strings; feature
  values are an arbitrary record.
- `packages/app/src/agent-profiles/settings/agent-profiles-section.tsx`
  and `internal/use-agent-profiles.ts`: an ordered whole list in daemon
  configuration. An edit replaces the record at its stable id, including
  removing cleared fields; deletion removes it; reordering moves array
  entries. There is no per-record RPC. Unread/disconnected/unsupported
  differs from a successfully read empty list.
- `packages/server/src/server/daemon-config-store.ts` and
  `authorization/operation-permissions.ts`: reads need `daemon.read`,
  writes `daemon.manage`; mutations are serialized, persisted before
  live replacement, and returned as current configuration. The persisted
  `daemon.agentProfiles` array replaces the old array. There is no
  caller-supplied list revision in these requests.
- `internal/use-profile-form-inputs.ts`, `profile-form-model.ts` and its
  tests: host-wide provider discovery, feature requests keyed by the
  provider/model/mode/thinking tuple, concrete reported defaults, stale
  reply rejection, dependent selections cleared after a provider change,
  and unavailable distinct from an empty feature list.
- `internal/materialize-profile.ts` and its tests: trim/drop blank
  optional fields; copy values into a creation request. Running apply
  omits provider and absent fields. The server's
  `session/agent-config/agent-config-session.ts` applies model, mode,
  thinking, then features. Rejection stops later steps; earlier changes
  remain. This is explicitly not a transaction.

Vessel already has `HarnessRef`, account `ProfileName`, agent-reported
`SessionOption`/`OptionChoice`, structured creation choices, a driver-neutral
probe, process-group ownership and a bounded SQLite writer. These are the
assets to extend. No external dependency is needed.

## Options and decision

[Decision Log]
- Purpose: let every client read and edit the same named launch bundles,
  without granting new execution or credential authority.
- Constraints: accounts remain node-owned; agent capabilities vary with
  the selected account and options; several devices edit concurrently;
  invalid or later data must never become an apparently empty list.
- Alternatives: reuse account profiles (conflates credentials and launch
  choices); store client-local presets (devices disagree); add fixed
  provider-specific option fields or raw JSON (duplicates the drivers);
  put another mutable file beside the database (another writer, revision
  and recovery path).
- Choice: extend the existing typed options/probe and store writer, with
  a separate launch-profile domain, whole-list compare-and-replace, and
  an account selector by name only.
- Why: the existing owners already enforce account resolution, launch
  validation, transaction acknowledgement and process cleanup. Reusing
  them preserves those boundaries rather than restating them.
- Consequences: a store migration and new calls are required. Stale
  edits need an explicit re-read. CRUD does not imply a running-agent
  bundle-apply capability.

## Values and bounds

`LaunchProfile` has private invariant-bearing fields:

| Field | Type and meaning |
| --- | --- |
| id | `LaunchProfileId`, a UUID stable across edits and reorder |
| name | `LaunchProfileName`, trimmed, nonblank, one line |
| launch | `LaunchSelection { harness: HarnessRef, account_profile: Option<ProfileName>, options: LaunchChoices }` |
| icon / color | Optional distinct `LaunchIconKey` / `LaunchColorKey`, registry keys, never drawn glyphs |
| notes | Optional `LaunchProfileNotes`, trimmed multiline public guidance |

`LaunchChoices` owns a bounded ordered list of existing `OptionChoice`
values. Its constructor and decoder check nonempty bounded option/value
ids, control characters, kinds and duplicate option ids. It exposes
borrowed choices, never a mutable vector that can break its invariant.
Model, mode, thinking and additional features are all choices named by
the agent; `OptionCategory` places the controls and does not infer what
an id means. There is no second fixed model/mode/thinking representation.

Built definitions live only in the validating types. Messages, tests and
documentation refer to those names. Catalogue/admission definitions below
remain proposals until those parts are built.

| Named bound | Proposed definition |
| --- | --- |
| `LaunchProfiles::MAX_PROFILES` | Defined in `launch_profiles/list.rs` |
| `LaunchChoices::MAX_CHOICES` | Defined in `launch_profiles/choices.rs` |
| `LaunchChoice::MAX_ID_BYTES` | Defined in `launch_profiles/choices.rs` |
| `LaunchProfileName::MAX_CHARS` | Defined in `launch_profiles/text.rs` |
| `LaunchProfileNotes::MAX_BYTES` | Defined in `launch_profiles/text.rs` |
| `LaunchAppearanceKey::MAX_BYTES` | Defined in `launch_profiles/text.rs` |
| `LaunchCatalogue::MAX_OPTIONS` | 64 descriptors |
| `LaunchCatalogueOption::MAX_VALUES` | 128 select values |
| `LaunchCatalogueLabel::MAX_BYTES` | 512 UTF-8 bytes for labels/group labels |
| `LaunchCatalogueDescription::MAX_BYTES` | 4096 UTF-8 bytes |
| `LaunchPayload::MAX_BYTES` | Defined in `launch_profiles/codec.rs` |
| `LaunchProfileJobs::MAX_ACTIVE` | 4 admitted jobs per node |

Optional text is omitted to clear it; an explicitly blank optional value
is invalid after trimming. Notes allow LF/CR line breaks and tabs, but no other
control characters. Opaque agent ids are not case-folded or rewritten.
The account selector reuses `ProfileName`'s existing rule. The list rejects
duplicate ids; duplicate display names remain allowed, as in the reference.

The payload bound covers canonical request/reply bodies, including JSON
escaping and metadata. List construction budgets the maximum serialized
revision, so incrementing it cannot make an accepted list unreadable.
Array decoders stop at the named count; they do not construct another
domain item beyond it. A single oversize/invalid record fails the whole
list or catalogue: no truncation, skipped record or partial success.

`LaunchProfileRevision` is an exactly JSON-representable unsigned counter
with the range of `u32`, including the initial empty state's zero. Checked
increment never wraps. `LaunchProfileRevision::MAX` comes from that
representation;
SQL decoding rejects negative or out-of-range values.

Unknown fields in a replacement are rejected, not discarded. Persisted
data is decoded as one complete typed snapshot. A later representation
must explicitly adapt/refuse editing at the negotiated protocol boundary;
it must not round-trip unknown fields through a client that loses them.

## Calls, admission and failures

These are proposed call names, not registrations:

| Call / request | Scope | Response |
| --- | --- | --- |
| `launch_profile.list` / `ListLaunchProfiles {}` | Read | `LaunchProfilesSnapshot { revision, profiles }` |
| `launch_profile.replace` / `ReplaceLaunchProfiles { revision, profiles }` | Admin | The committed current snapshot |
| `launch_profile.options` / `GetLaunchProfileOptions { selection }` | Read | `LaunchCatalogue { selection, options, probed_at }` |

Owner, direct paired-device and delegated human calls use the usual scope
intersection. Reading the list is also available to a Read agent, with
notes intentionally visible as in Paseo's `list_profiles`. Agents,
including delegated agent origins, cannot replace the list or run this
editor's probe. Existing session option reads and all ownership rules
remain unchanged. No Read grant authorizes starting a conversation or
applying a profile.

A replacement is the complete ordered next list. Creating appends a new
id; editing keeps its id and replaces the record; deleting omits it;
reordering changes array order. A matching revision with identical data
returns the existing snapshot without increment. A stale revision fails
before any change, even for identical content. Exhaustion also fails
without a change. A client re-reads and lets the person reconcile a stale
edit; it never silently resubmits against the newer revision.

Typed constructor errors drive local field validation. Malformed wire
bodies use `InvalidRequest`; consumers never parse message text. Service
errors carry a typed launch-profile detail: stale revision,
revision exhausted, configuration changed, account unavailable, harness
mismatch, unsupported agent options, invalid choice, payload too large,
busy, persistence failed or corrupt stored snapshot, plus Unknown
for a future reason. Existing authentication/scope and process-launch
error codes retain their meaning. Stale/configuration/exhaustion is
`Conflict`, admission/shutdown is `Unavailable`, and unsupported
structured options is `Unsupported`. No failure returns an empty list.
Mutation/probe tracing records method, outcome, revision and count, not
submitted notes or raw payloads. Validation messages never quote their contents.

`LaunchProfileJobs` uses immediate admission, not an unbounded queue.
Admission stays with the owned operation until its store acknowledgement
or probe/process cleanup completes, including caller cancellation and a
completion waiting for its caller. On handoff to the RPC machinery, that
machinery's existing in-flight/outbound bounds own the bounded response;
there is no second unbounded output queue. A disconnected writer may still commit;
after a lost acknowledgement the caller reads the snapshot before any
retry. Admission is released only when that owned job actually ends.
Shutdown stops admission and drains owned work through the existing store
and process lifecycle; it does not claim to cancel an in-flight disk write.

## Storage and account boundary

The existing `vessel-store` database owns one singleton row containing the
revision and ordered typed JSON list. A forward migration initializes an
empty list. The node integrator assigns that migration's number; no entry
or schema constant is added before allocation. There is no second config
file and no change to `profiles.toml`, `agents.toml` or `settings.json`.

The writer performs revision comparison, whole replacement and checked
increment in its existing per-operation savepoint. Its typed answer is
sent only after the enclosing batch commits. Commit failure cannot publish
a successful revision; another failed operation cannot partially replace
this list. Reads use one complete database snapshot, not a second memory
cache. Stored byte length is checked before fetching/decoding the body;
corruption or a later unreadable record fails the whole read and leaves
the original data unchanged. SQLite's existing crash/power-loss policy is
unchanged; acknowledgement is not a stronger power-loss guarantee.

The database and its journal use the existing private data-directory/file
helpers and service UID/ACL. No extra write permission is granted. Tests
cover resulting ownership/modes, failure and restart under the actual test
user, including Windows' supported file/ACL behavior where available.

Only the account's name is stored and exposed. Environment values, home
paths, tokens, commands and secret names/values are not launch-profile
fields. A saved selector may be temporarily unavailable: it remains
readable/editable instead of preventing deletion of unrelated records.
Actual probing/launch resolves the selector on that node and refuses an
unknown account or a harness mismatch; it never switches accounts as a
fallback. Reading a profile grants none of that account's authority.

## Agent-reported form options

Extend the existing driver-neutral probe, not a provider lookup table.
Resolve harness/account from one retained `NodeProfiles::current()`
snapshot, use the node's host-wide probe directory and existing isolated
launch setup, apply the requested choices in order during the handshake,
read the resulting options, then stop the process group. No prompt, model
request, MCP server or persistent Vessel session is created by the probe.
The existing handshake and stop deadlines apply.

The new selection-dependent operation does not populate the default
`harness.options` cache with draft results and introduces no draft-key
cache. Bounded admitted jobs own their probe through cleanup. If the
profiles snapshot is replaced before publication, return configuration
changed rather than publish that old account selection as current. The
retained allocation prevents identity reuse while the job exists. This
does not claim knowledge of external login edits the node has not observed.

The catalogue echoes its exact typed selection. Clients isolate requests
by machine, harness, account and ordered draft choices and reject stale
replies. Provider/account changes clear dependent draft choices; each
subsequent choice uses the catalogue reported after the choices before it.
Reported current select ids/toggle states are defaults, never synthetic
unset values. Unknown option kinds are shown as unavailable controls.
Unsupported structured sessions is an error; a structured agent that
successfully reports no controls has a genuinely empty catalogue.

Descriptors retain the agent's ordering, labels, groups and categories;
all descriptor/value ids and text have the bounds above. No arbitrary
JSON feature values or implicit null/reset choice is accepted. A driver
that does not report a feature does not acquire it from a preset.

## Launch and running-apply boundary

As in Paseo's materializer, choosing a profile copies its launch selection
into a normal structured `SessionSpec`: harness, optional account profile
and ordered options. Omitted choices follow that agent's defaults. A
shared typed materializer is used by the client/CLI, not reimplemented in
each screen. The node still applies the existing create scope, ownership,
workspace and option validation. An unoffered choice fails creation before
Ready; it is never silently dropped, particularly for permission modes.

No launch-profile id is recorded as a live binding: later edits/deletion
never mutate or restart existing sessions. The new profile contains no
system prompt, environment or process override.

The initial CRUD contract did not add a running-session bundle-apply call. Existing
`session.set_option` applies one choice at a time, Operate/ownership checked;
several such calls are partial and may interleave. CRUD support must not
be advertised as Paseo's `agentConfigApply` support, nor enable a pretend
atomic Apply button. A future bundle operation must explicitly report its
applied prefix and first refusal and define interleaving; rollback and
provider/process replacement are not inferred from this ADR.

## Differences, implementation and acceptance

Necessary differences from Paseo are explicit: typed bounded values and
strict replacement input replace arbitrary passthrough; SQLite/writer and
revision CAS replace daemon-config array patching; harness plus named
account replaces a provider string; agent-reported ordered choices replace
fixed model/mode/thinking fields and arbitrary feature values. Unavailable
choices are refused at launch rather than silently removing a mode. CRUD
is independently useful; running bundle apply is unbuilt and proposed in
the A10 expansion below. These
follow Vessel's existing owners and prevent credential exposure, lost
updates and an unrequested permission-mode change.

`vessel-proto` owns values/requests, `vessel-store` owns the row/transaction,
`vessel-node` owns admission/resolution/probe routing, and existing
`vessel-session`/`vessel-agent` own launch/driver behavior. Client, CLI, MCP
read, generated TypeScript and fake-fleet fixtures consume that contract;
there is no new crate, dependency edge, native screen or page work.

After the announced Quality B head is consumed, register through its one
method table and shared accessor using the pending launch-profile feature,
`NAMED_LAUNCH_PROFILES`, whose `Option<ProtocolVersion>` stays None until
the integrator assigns it.
No dummy/current version or second method list. Until then, register no
calls. All direct and nested fleet hops must refuse unassigned/older calls
before dispatch; existing replies/events gain no fields and MIN stays as
it was. Pending and assigned happy-path evidence are reported separately.

Acceptance covers boundary/UTF-8/escaped-byte/duplicate/clear/order fixtures;
two editors and failed transaction/restart/corruption; owner, device,
no-Read, agent and delegated authority; fake ACP and direct-driver probes,
dependent options, reload, cancellation/admission/shutdown; pure
materialization and existing creation refusals; every unassigned/older
transport hop; CLI/MCP and seeded fake-fleet reads/mutations. Tests use
existing fake binaries, `within` and named finite phases, including owned
child kill/reap. No real agent is prompted. Per-commit narrow checks precede
the package's one complete root gate and implementation review.

## A10 expansion: draft/live application and legacy favorites (proposed)

The expanded parity assignment requires the complete profile workflow,
including running application and legacy migration. This section replaces
only the initial package's deliberate omission of running apply. It does
not weaken creation's existing fail-on-unoffered-choice rule or authorize
an account/process change. It is not implementation approval.

Additional source read: Paseo's complete
`agent-profiles/internal/use-agent-profile-picker.ts`,
`agent-profiles/migration/index.ts` and all its tests; client
`daemon-client.ts::applyAgentConfig` and server
`session/agent-config/agent-config-session.ts::applyBundle`. The caller's
config request uses the session RPC timeout. Migration is client-local,
with one in-flight job per host and a completion marker only after success.
No-favorites completes without a catalogue read. A catalogue failure or
failed host write leaves the migration retryable.

### Roles and materialization

A stored choice gains a typed reported `OptionCategory` hint, defaulting
only to Other when no category is known. This is placement/order metadata,
not execution authority or a replacement for the driver's validation.
The existing opaque id and select/toggle payload remain unchanged. Keeping
this hint is necessary to recognize a retired saved mode even when its id
has disappeared from the live catalogue. There is no provider-id string
switch, fixed mode-id table or arbitrary feature JSON.

Draft materialization is one pure transition returning harness, account
selector and concrete choices for ordinary creation. It retains the
existing strict launch behavior. Live materialization is separate:
await an authoritative live option snapshot; keep the running agent and
account immutable; omit only a saved mode absent from the known live mode
choices, leaving the live mode alone. Unknown/not-yet-read modes defer the
whole application. Other unsupported choices are not silently removed.
The prepared result records omitted modes for the caller. Retired model
or feature values still reach their normal validation/refusal path.

Proposed eligibility read: `launch_profile.applicable`,
`GetApplicableLaunchProfiles { session }`, Read. It uses the existing
readable-session boundary and the node's account-name canonicalization,
not a client-side harness/default-account table, to select profiles for
that running agent/account. Terminal/ended sessions are inapplicable;
parked structured sessions remain eligible. No credential file is read
and no process is started. A distinct `ApplicableLaunchProfiles` result
exposes filtered entries and list revision, not an editable whole-host
snapshot. Its body keeps the same revision/profiles budgeted representation.
Request correlation identifies the session; the app discards stale results.
Agents meet the same Read rule as listing public profile notes. Applying
still requires Operate and session ownership independently of this read.

Application order is the reported model, mode, thinking category, then
remaining features, stable within a category. An explicit direct API choice
is not a request to silently omit an invalid mode: the profile materializer
performs that reconciliation before submitting the bundle. This matches
Paseo's client reconciliation and server refusal boundary, while creation
continues to reject an unoffered permission mode.

### One live mutation boundary

Proposed additional call: `session.apply_options`,
`ApplySessionOptions { session, options: LaunchChoices }`, Operate,
targeted by the normal session ownership guard. It accepts no profile id,
provider, account selector, environment or process override. Agent and
fleet-origin callers meet exactly the same scope/ownership rules as
`session.set_option`; terminal/ended/moving sessions keep their existing
refusals. A parked session uses the existing wake path and captures one
resulting agent-process input handle for the entire bundle.

The current ACP setter starts an asynchronous provider request and returns
to its driver loop. Merely looping the public setter in a node RPC would
therefore allow other clients' changes between steps. Extend the shared
`AgentInput` configuration boundary with one bounded owner per process.
Both existing single-option mutations and a bundle pass through it. It
waits for each actual provider completion before the next step; provider
outputs, permission answers and stop/cancel processing continue normally.
A model request/prompt must not be synthesized by this owner.

The owner returns a typed completed prefix and either complete success or
the first refused choice/reason. Earlier changes remain applied and their
normal option events continue; later choices are not sent. A typed optional
provider notice accompanies completion: retain the first notice without
skipping subsequent setters. Only a notice the driver reports is carried;
no notice is invented from an opaque option id or raw stderr. A driver
without a notice reports none. The internal completion seam must carry this
fact rather than claiming a generic Vec-of-options already contains it.

No caller-owned mutex or dropped RPC future may release the mutation fence
while the provider still has an in-flight setter. A lost acknowledgement,
client timeout or disconnect is an uncertain result, not permission to
retry the bundle or to claim rollback. The owned job remains fenced until
actual completion or closure of that same process. A hung provider can
hold its own lane; stopping the process closes it and drains replies. New
work is bounded and other sessions/metadata reads remain independent.
The client uses the source session-RPC budget for this new operation;
existing call/test deadlines are not raised. This promises finite admission
and lifecycle cleanup, not a fabricated provider acknowledgement at timeout.

Shared configuration queue/admission bounds are named beside the owner;
apply admission is separate from metadata/probe admission so hung setters
cannot consume every profile-list slot. Queued/running jobs and completion
handoffs retain ownership through cancellation. No second unbounded queue,
process switch, hidden restart, retry or central session-map mutex is added.
The configuration queue reuses the existing agent-command capacity's named
definition; separate metadata/probe and apply admissions each use the
accepted `LaunchProfileJobs::MAX_ACTIVE` bound, rather than duplicate a
numeric definition. The new client budget follows the pinned source's
`DEFAULT_SESSION_RPC_TIMEOUT_MS`; it is not applied to older methods.
The owner must exit when its driver's command receiver closes even when
an external clone of the input handle is retained.
Tests drive the real owner with fake provider completions, barriers and
finite safety clocks, including a competing old single-field call, partial
failure, first-notice continuation, dropped callers, delayed acknowledgement,
process replacement/closure and cleanup with held external input handles.

### Legacy migration ownership and stable identity

The checked native preference schema has remembered agent choices, not a
favorite-model list. No existing Vessel favorite key or completion marker
was found; remembered choices must not be reinterpreted as favorites.
The app owns validated device-local legacy input, a per-NodeId completion
marker and one in-flight migration per host. N3 supplies a shared typed
client planning/application helper over the ordinary host list/CAS calls.
It never opens another application's data or credential directories. The
app's existing preference writer remains its owner's, not a node file.

Legacy input is a bounded ordered set of opaque provider/model keys, mapped
through the relevant reported catalogue into a real harness/account/model
choice. Unsupported mappings remain retryable; no account is guessed.
Deduplicate legacy pairs, existing equivalent provider/model selections
(including profiles with extra mode choices), and previously imported ids.
Existing profiles stay first and untouched. New labels follow the source's
model label, provider/model disambiguation and suffix order. A label or
whole list beyond a named bound produces a typed unresolved result; it is
not truncated or falsely marked complete.

The profile id remains a UUID. For imported favorites use deterministic
namespaced UUIDs over the canonical typed provider/model pair and a fixed
versioned namespace. Enabling the existing `uuid` crate's v5 feature is the
proposed dependency-feature change: it replaces a custom hash/escaping
scheme and preserves retry-stable ids while keeping the accepted UUID
representation. This id is public metadata, never a credential or bearer
authority. The source uses reversible `legacy_favorite:...` ids; this UUID
representation is a deliberate difference. No credential value is hashed,
logged, copied or associated with the migration.

Migration checks both CRUD and live-apply protocol availability before
consuming a local completion marker. With no favorites, completion needs
neither catalogue nor host mutation. Otherwise catalogue resolution must
succeed before appending. The source loading retry schedule is preserved
as one named client constant for explicit transient pending/busy catalogue
resolution; ordinary authentication/validation/provider errors stop the
attempt, leaving completion unset. The node probe normally awaits a final
answer, so it does not manufacture a Loading catalogue or silently treat
all Unavailable errors as retryable. Timed backoff is tested with a driven
clock, not wall sleeps in a test.

Host replacement uses the current revision. A stale migration plan may
re-read and rebuild its append-only additions, never overwrite a manual
edit; it is distinct from blindly resubmitting a stale editor replacement.
Conflict/failure, unresolved mapping, unknown acknowledgement and failed
local marker persistence leave migration retryable. A lost host reply is
resolved by reading and deduplicating, not repeating an unconfirmed write.
Only successful append/already-present/no-favorites writes completion;
later user deletion does not recreate favorites once that marker exists.
The bounded retry/terminal states and source migration fixtures are shared
with the app rather than copied into its screens.

The new live call uses the same pending feature/table/accessor after the
announced Quality B head is consumed. The store allocation remains the
integrator's; this amendment adds no migration entry or protocol number.
All new state, outcome, notice, legacy-key and result types need the same
constructor/decoder/serialized-budget discipline as the accepted values.
Implementation and the complete A10 package gate remain outstanding.

### Canonical account facts for materialization and migration

A successful selected catalogue also carries the actual retained account's
canonical name and whether it is the default or a named account. The
request selection is still echoed exactly. These nonsecret node facts let
the shared migration helper compare an omitted account selector with an
explicit default-name selector without a client harness/default table.
They do not expose account settings, read another account's files, or
allow an apply request to override the running account. Unknown future
account kinds never authorize default inference. The complete catalogue
byte budget includes these facts. This is an implementation refinement of
the accepted canonical account boundary, subject to package review.
