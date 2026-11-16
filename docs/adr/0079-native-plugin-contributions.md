# ADR 0079: Native plugin contributions and a node-owned helper contract

Status: architecture contract accepted by V3, 2026-11-15; implementation
remains unreviewed. Extends ADR 0068; no executable plugin is
enabled by this document. ADR 0068's hook machinery was accepted on its
track in commit 118a59da. Its original RPC-only hook site and conflated
timeout/refusal were returned and corrected before that acceptance.

## Purpose and intent

Carry Paseo's plugin installation, management, server extensions and client
contributions into Vessel. The native app must render contributions using
the existing kit and must never evaluate downloaded code. A process is a
fault boundary, not a sandbox. Installing a helper means trusting it with
the node account's filesystem and network access.

## Existing implementation and constraints

The native baseline has no plugin calls, settings, catalog or runtime.
ADR 0068 fixes hooks and their placement but intentionally excludes the
host and UI. Its worker has uncommitted machinery: do not copy, reset or
overwrite that work. Integrate only a reviewed commit supplied by its owner.
Existing automation hooks are HTTP deliveries and cannot host these hooks.
Existing `vessel-proc` supplies process-group/job-object ownership;
`vessel-proto::RawJson` carries opaque plugin arguments without inspecting
them; native view/link/app/kit layers already separate policy, I/O and paint.

Reference is Paseo 0.15.4, read without copying its implementation:

| Behaviour | Paseo source, relative to its repository |
| --- | --- |
| Valid installation ids, directory enable default | `packages/protocol/src/plugin-config.ts:3-16` |
| Manifest, argv build commands, confined assets | `packages/server/src/server/plugins/manifest.ts:7-46` |
| Global switch, list, installs, reviewed updates, reload/remove | `packages/server/src/server/plugins/index.ts:218-548` |
| Managed roots, source metadata, revision truth | `packages/server/src/server/plugins/managed-source.ts:27-28,58-143,146-180,398-403` |
| npm scripts disabled, pinned artifact | `packages/server/src/server/plugins/managed-source/npm.ts:49-80,87-120` |
| Registry host-bound credentials, no redirect, deadline | `packages/server/src/server/plugins/managed-source/registry.ts:20-69` |
| IPC names and registration metadata | `packages/server/src/server/plugins/plugin-process-protocol.ts:34-110` |
| Request/tool/shutdown deadlines, log bounds | `packages/server/src/server/plugins/runtime.ts:41-46` |
| Settings, RPC, provider, usage registrations | `packages/plugin/src/server/contracts.ts:13-45` |
| Hook names and waiting bits | `packages/plugin/src/server/lifecycle.ts:174-192` |
| Shared agent creation site | `packages/server/src/server/agent/agent-manager.ts:1348-1413` |
| Hook rewrite restrictions | `packages/server/src/server/plugins/lifecycle/index.ts:220-249,283-324` |
| Client extension points and panel contexts | `packages/plugin/src/client/contracts.ts:24-205,230-245` |
| Catalog reconnect and invalidation | `packages/app/src/plugins/catalog-sync-model.ts:15-91` |
| Theme validation, semantic mapping, multi-host identity | `packages/app/src/plugins/themes/index.ts:23-155` |
| Host Plugins screen, feature gates and row actions | `packages/app/src/screens/settings/plugins-page.tsx:35-108,263-386` |
| CLI command/flag declarations | `packages/cli/src/commands/plugin/index.ts:211-268` |

## Alternatives considered

1. Evaluate Paseo's React bundles in a web view: violates ADR 0035 and
   brings a second UI runtime, executable client code and styles.
2. Embed JavaScript or Wasm: neither renders React through GPUI; both add
   a runtime and adapter without solving the contribution boundary.
3. Native-only compiled Rust libraries: faults enter the node/app process,
   ABI is unstable and distributing every plugin needs platform binaries.
4. Typed declarative client data plus a supervised helper: keeps the
   native kit authoritative and preserves node-side extension points.

## Decision and reasons

Choose option 4. Reuse and extend the accepted hook machinery, process
supervision, node authorization and kit. Add plugin-specific models because
automation deliveries, provider options and core routes have different
identities and lifetimes. No new external runtime dependency is selected.
The helper is an explicitly installed executable speaking versioned JSON
frames; it may itself be a Node adapter. Vessel does not promise that an
unmodified Paseo TypeScript package runs. A Node adapter/compiler is a
separately tested compatibility layer, never an implicit download.

### Trust, credentials and permissions

One top-level node setting, `plugins`, contains the global switch (absent
means off), installations and registry configuration. Core/bundled adapters
remain distinct from user installations. Installation, update, enable,
disable, reload and remove require the node's existing local-owner/paired
Admin authorization and refuse agent/delegated callers. Reads retain caller
scope and omit private paths, source credentials and helper diagnostics
where the caller cannot manage the installation. The app shows the trust
notice before an install action and the global enable switch independently.

An unsandboxed executable can read the node account's files, including
credentials already accessible there; the UI says so. The host adds no
tokens, pairing links, harness homes or secret values to argv or environment,
does not clone the node environment, and runs as the node's existing UID/GID.
An allowlisted minimal environment and node-owned working/temp directories
are prepared explicitly. Registry credentials stay in the node fetcher,
are sent only to their configured host over HTTPS (loopback exception),
never across redirects or into an executable's added environment.

The helper's optional node API broker is a distinct plugin principal,
revoked on helper exit and checked by ordinary scope/root/session guards.
It cannot mint devices, change grants, install plugins, manage secrets or
escalate itself through configuration. Person-triggered RPCs are bounded
by caller authorization as well as plugin policy; background hooks run
with the installation's declared policy. No plugin may replace a caller's
principal or bypass post-hook validation. This restricts Paseo's broad
`context.paseo`; OS access remains unsandboxed and cannot be revoked by
the broker. Native declarations never contain credentials or executable
callbacks. A screen action cannot execute a local program or arbitrary URL.

### Sources, storage and lifecycle

Directory, git, npm and registry are distinct typed source identities.
Managed installs live under node-home `plugins/<id>/<generation>/`, with
private staging and source metadata; user directories are never deleted.
Git uses a noninteractive bounded child; npm disables lifecycle scripts;
registry resolution produces a pinned git commit or npm version, resolved
artifact and integrity. Explicit manifest build argv run without a shell
as trusted code in the candidate, before activation. Validate paths,
symlink containment, manifest, compatibility and declarations before use.
No network or package manager runs merely from viewing an install form.

Preview returns a reviewable update proposal binding installation identity,
generation/revision and target. Apply rechecks that tuple; a changed install
refuses the proposal. Candidate preparation failure preserves the active
version. Reload deliberately stops the old version first and may end Failed,
matching Paseo. Disable/global-off remove contributions, cancel pending work,
revoke the broker, and reap the process group before acknowledgement.
Remove also deletes only owned managed generations, plugin settings and
retained logs. Shutdown cancels every child and joins pipe tasks. Startup
settles configured helpers before exposing the initial authoritative catalog.

Runtime state is an enum: Disabled, Starting, Running, Stopping, Failed.
Saved individual enable intent remains separate from global enable state.
Logs retain Paseo's count/byte/line bounds and survive reload/disable/failure
until remove or node restart. Helper output is untrusted: core errors carry
typed causes and safe text; raw helper diagnostics are not leaked to scoped
callers. Never claim arbitrary helper stdout can be proven secret-free.

JSON errors at plugin decoding edges retain a concrete sealed parser cause.
Its Debug/Display expose only category and location; its source chain ends
before serde's input-bearing unknown-field/variant text. A fixed outer error
alone is insufficient because tracing and CLI reporters may walk sources.
Raw serde errors stay inside the decoding edge and must be sealed before
being returned or emitted as diagnostics.
The generic ClientDocument serde implementation seals structural errors too;
this rule applies even when it is nested inside another typed envelope. File
selectors require equality with JavaScript-edge-trimmed text rather than
normalizing it, so BOM-only or BOM-wrapped matchers refuse at direct and serde
construction boundaries.

### Shared checked process cleanup prerequisite

The current `vessel-proc::Child::wait` reaps the leader and requests a group
sweep, but logs a sweep failure and still returns the leader's status. Later
`signal` on Reaped is intentionally a no-op. Therefore F6 cannot acknowledge
disable/remove as fully stopped solely from that legacy success. No product
caller is enabled until an owned checked cleanup result exists.

Coordinator-designated proc-owned seam (portable b37a3ee8 scoped source/narrow
PASS; whole F6 integration remains owed):
`Child::wait_checked() -> Result<OwnedGroupExit, CheckedWaitError>`. Proc retains
the **first** sweep outcome on the owned Child, including when legacy wait
already ran. OwnedGroupExit has a private constructor, code accessor and original run observation;
CheckedWaitError retains concrete WaitError or an Arc of the first SignalError.
The existing wait signature/behavior stays compatible. Never retry a delayed
numeric group id after reaping: it may have been reused. The receipt proves leader reap and that the first group/job cleanup request
was accepted or already absent. It does not prove every descendant has died,
and does not cover deliberately escaped processes or all account processes;
the helper remains unsandboxed.
Linux/macOS/Windows proof belongs with this common primitive, with fake-helper
caller tests here. Restart ledgers/pidfd identity are separate and do not mint
this in-process receipt. W22 agrees on this boundary and will reuse it.

Until that seam is reviewed and consumed, the staged supervisor can prove its
framing/actor transitions and fake-child behavior but cannot stand in for a
complete node disable/remove cleanup acknowledgement. Keep that node path
Unavailable/unknown rather than promoting a leader-only receipt.

### Node-side helper protocol and contribution API

The private helper protocol has its own negotiated version, independent of
node RPC versions. A bounded length-prefixed JSON frame contains a closed
message enum and typed ids; unknown version/type is a typed refusal. Stdout
is protocol only, stderr is a bounded diagnostic stream. No unbounded line
reader, channel or pending-request table. The node owns correlation and
generation; stale replies cannot fulfill a newer runtime's request.

The first private-format value slice uses version 1, a nonzero u64 node-issued
InvocationId, a 4 MiB complete JSON-body limit and a 3 MiB opaque payload limit.
HelperPayload redacts Debug instead of exposing RawJson through incidental
diagnostics. Its owner alone decodes business output. Closed modern HookName
and HookWait carry all fourteen names and the reference wait table; subscription
replacement rejects duplicates. Shutdown is an empty struct variant so unknown
members cannot hide in serde's unit-variant behavior.

This initial typed private envelope covers initialize/ready, invoke/hook,
cancel/result/error/fatal, complete client/subscription changes and shutdown.
It does not advertise server tool/provider/usage/settings/broker capabilities:
such unknown families refuse until the corresponding owner adapters and typed
envelopes are implemented. No node RPC or executable helper is activated by
these values, and this slice does not fulfill the complete server API below.

Mirror Paseo's server API families, with typed envelopes and opaque RawJson
only for plugin-defined schema payloads: initialize/ready; invoke/result/error;
hook/cancel/subscription-changed; settings changed; tool call/cancel/detach;
usage discover/fetch; provider status/catalog-key/connect/send/close and
accepted/rejected/events; shutdown; guarded node API request/response.
Registration is atomically published on Ready and removed as a unit on exit.
Unknown declared capability fails registration rather than silently dropping
it. RPC method schemas validate input/output at the helper boundary; the
node does not interpret plugin-owned opaque payloads as domain objects.

Settings use host scope, version, migration and optimistic revision; read
can be Ready/Invalid and write Saved/Conflict/Invalid. A file and the settings
change event publish only after a successful owner-private atomic write.
Providers use the existing typed provider path, usage sources the existing
usage model and agent tools the MCP tool boundary; absent integration is
explicitly unsupported and never advertised as running capability.

Preserve all fourteen modern lifecycle names/wait bits, cancellation and
ordering from ADR 0068. `agent.create.before` fires at shared creation,
after draining refusal and before checks that read a rewritten spec, covering
RPC, scheduled, fork and new side-session creation. Import/resume and future
ephemeral internal generator sessions skip it as in ADR 0068. `after` fires
only after successful shared creation and is nonwaiting. A refusal maps to
InvalidRequest (do not retry); timeout to Unavailable (may retry). Hook errors
do not expose helper text. Workspace/worktree/open/turn/archive/close/permission
hooks attach to their own shared owner paths, never just the RPC wrapper.

Use Paseo's request, tool, git and soft-shutdown time budgets and log bounds,
each defined once as a named constant in its owning module. Add named bounds
for frame bytes, declarations, tree depth, pending work and schema size:
Paseo has no bound for Vessel's new declarative format, so these are security
limits rather than claimed Paseo defaults. Unix uses vessel-proc groups and
Windows its job objects. An unsupported executable target returns a typed
platform refusal before installation/activation, never a partial live plugin.

### Declarative native client API

The catalog contains data, not a React bundle: plugin id, generation,
requirements, translations and a declaration document. Identity is
`(node, plugin, contribution kind, contribution id)`; user preferences may
deduplicate by `(plugin, kind, id)` while retaining the selected node.
Dynamic updates replace a generation atomically, validating references first.

Declare screens, host settings screens, workspace/agent/file panels,
sidebar header/footer items, global/workspace/agent commands, slash commands,
header buttons, composer pills, attachment sources, timeline presentation
rules and themes. Targets carry typed workspace/session/file identities;
string screen parameters are bounded text, not path or program authority.
Buttons expose action/menu/popover enum variants; availability and disabled
state are explicit and can be updated/removed with their generation.

Surface nodes are a bounded tree of Text, Markdown, Stack, Section, Row,
Button, TextField, Choice, Toggle and Status using existing kit controls.
No CSS, JavaScript, SVG markup, shader, font family, arbitrary image URL or
executable string. Core controls own focus, keyboard, accessibility, compact
layout, tokens and text scale. Actions are local navigation or a registered
plugin RPC; Rpc actions carry captured host/plugin/generation and context,
revalidated at dispatch. No host fallback when the selected host is offline.
Arbitrary React animations and custom rendering require an explicit native
control decision, rather than an executable escape hatch. Audio and timeline
transformation need typed opt-in adapters; they are not performed implicitly
by evaluating a catalog. Unknown controls or missing action targets fail
the whole installation's client declarations with a visible typed error.

Themes use exactly Paseo's small semantic palette: background, foreground,
raised, control, border, optional accent, muted foreground, ring; light/dark;
trimmed name bounded as Paseo; hex accepts its three supported widths.
Theme identity is `<plugin>/theme/<id>`, deduplicated across eligible hosts
with preferred host else first target. Missing contribution uses System
while retaining the stored choice; offline retained catalogs still resolve.
Accent defaults to foreground; destructive colors stay core defaults.
Font, size, width and syntax preferences are reapplied after palette changes.
Only the theme adapter changes kit tokens: plugin declarations never supply
paint literals directly to app controls. Disabling/removing a plugin withdraws
its themes without deleting the person's preference.

### Catalog, app and CLI plumbing

Native view owns parsed declarations, registry, selection/trust/page state;
link owns existing-node calls, bounded catalog/cache I/O and reconnect;
app owns kit rendering and registration with existing feature owners. Keep
logic out of shared shell/link entry files. Do not add kit controls without
the coordinator's approval. Catalog sync is per node, serialized and
generation checked; retain offline declarations, invalidate settings on
reconnect, remove contributions on authoritative withdrawal, and never
substitute a different host. Hash-verified private cache advertises only
verified declarations. A malformed replacement preserves the previous safe
snapshot with an explicit evaluation failure, not partial registration.

Host Settings > Plugins reproduces trust notice, global enable, source
input, rows with saved enable intent, running/disabled/failed badge, logs,
reload and confirmed remove; offline/unsupported/loading/error/retry/empty
states. CLI supplies init, ls [id], logs, install --id/--ref, update reviewed
preview/apply, enable/disable/reload/remove and JSON output. Scaffold writes
the declarative manifest and helper contract example, no executable download.

No plugin methods exist on the baseline. The coordinator/integrator must
authorize the shared proto contract before registration. Proposed calls:
plugin list/catalog/logs; directory inspect/install; source install/status/
update preview/apply; enable/disable/reload/remove; invoke; typed catalog and
settings invalidations. One pending protocol constant, Option None, and the
shared required_protocol accessor follow A6. No invented request is sent.
Older peers get typed Unsupported; both client and server refuse unassigned
calls. New reply/event fields are gated on every nested/forwarded path.

Quality B subsequently landed at the reviewed node head 29eebb8b. Its
`Method::since()`/`Call::since()` return Option and method shape comes from
the single table. Pending registrations use `[Scope, (pending PLUGINS), unary]`;
this supersedes the earlier accessor spelling without adding another mechanism.
W11's V3-reviewed identity-only e45a5d89 supplies WorkspaceId for header/pill
targets; its broader registry/store work remains a separate dependency.

## Differences and consequences

| Difference from Paseo | Reason and effect |
| --- | --- |
| Declarative data instead of executable React bundles | GPUI/ADR 0035; authors port client UI; arbitrary React cannot run |
| Explicit helper executable instead of built-in TS compilation | Rust runtime stays standalone; Node compatibility needs a separately supplied adapter |
| One `plugins` settings namespace | A2; global flag, installs and registry config still distinct within it |
| Narrow broker instead of unrestricted PaseoApi | Preserve caller/root/secret/install authority; unsandboxed OS access is still trusted |
| Minimal environment | Prevent accidental credential inheritance; plugin declares its runtime dependencies |
| UTF-8 id ordering instead of process-locale localeCompare | Stable hook order across machines, as ADR 0068 already records |
| Only modern hook/registration names | No deprecated aliases on a new native contract; explicit unsupported errors |
| Bounded declarations, frames, schemas and queues | New format needs resource and cancellation guarantees |
| Validated bounded theme/contribution identifiers and control-free names | Native retained identities must be safe keys; this is stricter than the theme parser's unconstrained id string |
| Safe typed errors rather than helper exception text on scoped wire | Preserve retry class without leaking secrets or local paths |
| Attachment context URLs require explicit HTTP(S) authority and no userinfo | Retained context links have predictable client opening semantics; registration never fetches them. Paseo's general URL schema also accepts other schemes |
| Bounded attachment items/text/URLs and distinct bounded whole search lists | Bound retained draft/picker resources and avoid ambiguous selection; excessive results refuse rather than silently truncate |
| Declarative cache instead of JS bundle cache | Same host/hash/offline ownership, no client execution |
| Full plugin capability requires each owner integration | Do not silently ignore provider, tools, usage or client contributions |

Cost: more explicit schemas and a plugin author port; limited arbitrary UI.
Benefit: one native kit, typed ownership, predictable teardown and no code
evaluation on a connected app. Runtime evidence, installation safety and
cross-platform execution remain separate from acceptance of this design.

## Initial scalar limits

The staged proto scalar implementation owns its actual limits in
`plugins/identity.rs`: identifiers, method names, captions, UTF-16 theme
names, multiline content and literal file extensions. Its messages format
those constants; tests derive boundary cases from them. Other declaration
budgets land with their owning validation module rather than becoming a
second table of copied limits here. These are additive value types only:
`PLUGINS` remains None and no registered request or settings field exists.

Attachment URLs use the existing `url` dependency's real parser, with
additional scheme/authority/userinfo/control policies. Parser errors retain
the concrete input-free ParseError, not the legacy fleet HttpUrl error that
owns rejected text. Canary tests cover Display, Debug, serde errors and
the complete source chain. Theme names reuse the existing JavaScript-edge
`workspace::trim_project_text`, including BOM normalization, before their
UTF-16 bound and the separately declared control refusal.


The staged supervisor now consumes portable proc b37a3ee8 during development,
rebuilt directly on ede20014 without W15 release/desktop ancestry.
Supervisor::shutdown returns the concrete OwnedGroupExit, retaining the first
cleanup refusal as RuntimeError::Cleanup rather than flattening legacy wait.
This adoption is not runtime or node lifecycle acceptance: proc execution,
helper integration and management handler gates remain owed. No node caller
or advertised method is activated by the library foundation. Test time budgets
are named beside their fixture owners, including observation polling.

### Native package manifest admission

The explicit author-port manifest is `vessel-plugin.json`, closed format
`vessel.plugin/v1`: id, optional name/description/PNG icon/media, mandatory
requirements {node,client} protocol floors, optional client declaration path,
per-exact-target helper path/private protocol/argv, and trusted build argv.
At least a client or helper is required. There is no automatic TS compilation,
Node download or executable fallback between architecture/libc targets.

Vessel uses its typed protocol floors plus the closed manifest/private-helper
formats instead of Paseo's npm-semver requirements.paseo. This is a deliberate
native-author-port difference: it checks the actual data/API contracts without
inventing a relationship between Paseo versions and Vessel packages or adding
a second semver grammar. Unknown formats/targets/requirements refuse. This
format does not accept an unchanged Paseo package as executable native code.
The public PLUGINS constant remains None and no method number is assigned here.

Bounds are64KiB manifest,8 exact helper targets,16 media,16 build commands,
64 argv entries,16KiB per argument and32KiB aggregate per argv. Package paths
reuse RelPath but additionally forbid root, edge whitespace/BOM, colon/query/
fragment syntax; they are data, not canonical containment proof. HTTPS media
reuse the real bounded URL decoder and forbid HTTP/userinfo; this is stricter
than Paseo's https z.url and avoids private credential-bearing metadata. No
media URL is fetched or opened here. File identity, symlink containment,
permissions and candidate ownership must be proved by the future installer.
Build arguments preserve intentional leading/trailing spaces, but empty/JS-blank
and control-bearing arguments refuse, with a named aggregate argv budget.
Paseo permits more raw argv text; the stricter portable data admission avoids
ambiguous invalid launch values before trusted preparation. No shell wrapper
is inserted; explicitly authored build executables remain trusted account code.
Validated Manifest wraps a private deserializable input; callers cannot use its
Deserialize implementation to bypass whole-manifest validation because the
validated wrapper deliberately implements no Deserialize.

R resolved the helper-target boundary11:2x UTC: author labels may recognize
the eight triples, but only x86_64-unknown-linux-gnu and
x86_64-pc-windows-msvc are initially eligible for selection. Target::compiled
uses the node build.rs Cargo TARGET export VESSEL_TARGET, including libc/ABI;
no updater feed OS/arch identity or SSH discovery is reused. Adding a third
eligible target follows R's release matrix/qualification, not a manifest edit.
R's evidence is limited: Linux is built on Ubuntu24.04 without a claimed
minimum glibc; Windows is static-CRT, Windows10 1703+/11 build/system-DLL proof,
not real plugin/node execution proof. Test-only labels imply no platform
qualification, download, installation or successful helper start.

Source spelling is a staged input boundary, not an acquisition capability.
Native-compatible registry configuration is explicit; no public Vessel registry
or compatibility with unchanged Paseo published packages is invented. Bare npm
selectors keep npm's own resolution grammar with finite input bounds. Registry
transport is HTTPS except actual loopback HTTP, stricter than Paseo's
unauthenticated public HTTP option. URI userinfo/query/fragment refuse before
any future credential/fetch adapter. Directory/Git spellings remain unresolved
input and must not be passed to launch or storage without their real node-owned
canonical/pin/authorization adapter. No source install is enabled by parsing.


### Native CLI scaffold implementation choice

The local init leaf follows Paseo's required directory, optional validated id
and empty-directory admission. It emits a static native document and manifest
instead of TypeScript/React dependencies or npm scripts. Passive helper guidance
explains the separately built exact-target executable and existing framed V1
contract; no helper, runtime or callback is implicitly installed or advertised.
This makes scaffolding usable independently of the pending node plugin protocol.

W17's existing presentation parser/admission and Row/list formatter are reused.
The leaf writes only after preparing valid fixed templates, refuses nonempty or
symlink targets, and uses exclusive file creation. Existing directories retain
their permissions; newly created Unix targets/files are private to the invoking
account. The same-account pathname replacement race is not strengthened into a
filesystem capability. Failed writes may retain a partial scaffold: automatic
recursive cleanup could remove concurrent user work and is therefore excluded.
No source acquisition, settings mutation, node connection or executable launch
is part of this command. Its actual CLI/output and permission tests are separate
from installation/runtime/UI qualification.

### Private saved settings envelope refinement

The coordinator assigned the shared disk/wire separation to nodectl: the same
settings.json is to hold a private SavedNodeSettings envelope while generic
SetNodeSettings and Read replies retain only their safe typed surface. W16 will
supply the qualified private plugin schema and dedicated mutation admission,
not private fields in the generic wire patch or a second writer. The proposed
contract, concrete resource budgets and required preservation/authority tests
are in docs/plans/wp-plugin-settings.md. This is a design refinement, not an
implemented slot, available storage API or plugin activation claim. Joining it
waits for the envelope owner's reviewed source and announced containing head.

### Complete private candidate observation prerequisite

The declaration-only inspection is insufficient for executable review identity.
The proposed bounded artifact observation and its explicit Unix ownership versus
unsupported-ACL platform boundary are specified in
[wp-plugin-artifact](../plans/wp-plugin-artifact.md). This is a private source
prerequisite, not acquisition/pinning, consent, persistence or helper activation.
It introduces no second settings writer and cannot manufacture managed provenance.

The proposed canonical Literal/Message resource values, prepared presentation
and per-installation client-language lifetime are refined in
[wp-plugin-localization-wire](../plans/wp-plugin-localization-wire.md). This
proposal does not widen core Translator or approve a locale parser dependency;
its manifest/lock edge and actual client admission require their owning review.


### Staged regular-file prerequisite

The artifact plan now separates a Unix regular-file probe from the still absent
complete private-tree owner. It adds only a test-gated Unix dev edge to the
existing nix0.31.3 user/fs features for effective UID and bounded FIFO fixtures.
The probe uses nonblocking/no-follow open plus actual-descriptor verification,
sealed original I/O evidence and bounded hashing; it never supplies root/ACL,
confinement, managed provenance, installation or blocking-owner lifetime proof.
The reasoning and required real-filesystem cases are in the artifact plan.

### Prepared command diagnostic policy join

PreparedCommand consumes the reviewed proc CommandDiagnostics::Redacted policy
on the announced node line. It preserves real launch fields and the decorated
spawn/owned cleanup path. This seals actual spawn diagnostics, beyond the outer
prepared-value Debug; it does not protect OS argv or helper-authored output and
adds no Node product caller, executable/source authority or sandbox promise.

### Owned candidate boundary proposal

[wp-plugin-candidate](../plans/wp-plugin-candidate.md) separates exclusively
created, retained staging storage from observed content, acquired source facts
and human review. Its descriptor/cleanup/platform requirements are a proposed
private prerequisite, not a new producer, managed provenance or start authority.
The existing staged pathname observer and tracked operation remain separate;
their scoped tests cannot qualify the proposed root bootstrap or installation.
