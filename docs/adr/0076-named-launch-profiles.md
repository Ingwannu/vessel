# ADR 0076: Named launch profiles

Status: contract accepted, 2026-11-15; implementation is not approved.
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

This package does not add a running-session bundle-apply call. Existing
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
is independently useful; running bundle apply remains unsupported. These
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
