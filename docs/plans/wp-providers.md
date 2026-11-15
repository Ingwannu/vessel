# Provider coverage

## Reference and existing assets

References below are relative to the Paseo source tree, version 0.15.4.
They describe behaviour read from source, not a live-agent verification.

| Paseo behaviour and source | Vessel before this work | Planned implementation |
| --- | --- | --- |
| Six built-ins, exact defaults and unattended modes; `packages/protocol/src/provider-manifest.ts:38-257` | Four HarnessKind descriptors; no Pi/OMP | Provider manifest separate from executable-driver support |
| Explicit mode, same-provider inheritance, cross-provider unattended bridge, unknown versus empty catalogue; `packages/server/src/server/agent/create-agent-mode.ts:43-111` | ACP OptionChoice at session creation | Pure typed create-config resolver |
| OpenCode legacy full-access and auto_accept; `packages/server/src/server/agent/providers/opencode-agent.ts:248-294` | ACP transport, direct HTTP driver refused | Provider policy retains unset dynamic defaults |
| ACP unattended feature and cross-provider default; `packages/server/src/server/agent/providers/acp-agent.ts:903-934` | Generic ACP client and added agents | Shared ACP create policy |
| Default/append/replace launch and parent marker removal; `packages/server/src/server/agent/provider-launch-config.ts:92-114,208-252` | launch/profile/added-command and redacted CommandSpec | Extend launch policy without shell execution |
| Built-in/derived/plugin registry, opt-in and model merge; `packages/server/src/server/agent/provider-registry.ts:371-432,741-955` | AddedAgents and profiles, no registry | Typed registry; inert plugin contribution seam |
| Catalogue cache, scope, generation, timeout and change feed; `packages/server/src/server/agent/provider-snapshot-manager.ts:403-446,464-480,895-1099` | Node probes caches HarnessOptions/readiness together | Staged catalogue owner, shared provider policy |
| Claude native SDK, Codex app-server, OpenCode HTTP, ACP and Pi/OMP RPC; survey B sections 7-11 | ACP works; Codex direct offered; Claude/OpenCode direct BUILT=false; Pi/OMP absent | Existing transports reused where valid; absent transports explicitly unavailable |
| Plugin registration and retirement; `packages/server/src/server/agent/provider-registry.ts:925-947` | Plugin runtime staged separately | Definition input; no plugin execution in harness |

Decision: **modify existing assets**. Std collections and existing serde,
thiserror, Tokio, sha2, CommandSpec and Lookup cover these responsibilities.
A provider framework dependency would duplicate the process/probe boundaries.
ADR 0081 records the reasons and tradeoffs.

## Structure and flow

Validated provider configuration enters vessel-proto. Harness resolves the
manifest plus overrides plus plugin definitions into a registry. Catalogue
refresh receives typed observations from an existing agent transport; the node
owns caching/publication. Create-config resolution consumes the ready mode
catalogue and parent configuration, before any process is launched.

## Differences and limits

- A manifest entry is not evidence of an installed or supported driver. Missing
  transports refuse execution; ACP mode identifiers are not silently renamed.
- Configured agents keep an explicit environment allowlist (ADR 0032), a
  deliberate security floor above Paseo's daemon-environment inheritance.
- No credentials are included in arguments, added environment values, errors or
  Debug output. Environment policy is tested only with synthetic values.
- The catalogue and connection indexes supplied with the survey are unfinished
  heading-only placeholders; the source references above are authoritative for
  this package's mapping.
- No new call is registered before the shared method-table head is available.
  Feature version and any future SQLite schema number are assigned at merge.
- Real agent execution, sign-in, vendor network, installed CLI compatibility,
  native provider history and approval enforcement are not verified here.

## Current scope and resume point

The metadata core now has the registry, private-file owner, bounded catalogue
producer/cache, node startup/shutdown wiring,3pending typed methods and client
wrappers. PROVIDERS remains None: the integrator assigns the next free protocol
number at the wiring merge. The earlier reservation of24 was superseded when
host diagnostics received24; no provider number is assigned here. MIN2/current23
are unchanged on this package snapshot. No pending traffic is sent.
G3 keeps modern provider metadata apart from legacy installation/account rows;
no profile, sign-in, defaults or removal identity is inferred from a provider ID.

Prior immutable selections passed provider70, agent139(with2externalSDKtests
ignored), and RPC53; these do not prove the latest combined source. Actual
strict failures and the coordinator-approved queued cancellations are retained
below. Current node46g union, encoded reply budget and strict corrections await
merged-head strict and provider regression execution before the final whole
and V3 core PASS. All new Rust work runs on the build VM through the approved
remote gate. Detailed current run IDs are in the review packet; no green gate
or callable provider head is claimed here.

The pure create resolver is not an authoritative create RPC. Native mode
capability, actor-attested binding, actual applied provider/env/sign-in proof,
transient native prompt override/restore, missing native drivers, plugin session
adapter and durable helper ledger remain separate increments. Prepared values
and isolated positive tests are not full A9 parity or runtime authority.

The following chronological checkpoints preserve what each earlier snapshot
actually implemented or checked; their pending statements are historical.

## Implementation checkpoint

The initial narrow clippy found documentation-markdown and inaccessible bound
constants; no behavioural result was obtained. Those findings are corrected in
the launch/catalogue fill-in. Provider commands now resolve default, append and
replace forms as inert data. Added-provider environment inheritance is explicit,
parent markers are removed after overlays, and denied tools require a capable
consumer before a command can be yielded. Tests consult only testkit executable
paths and temporary homes; no process is started by these policy tests.

Additional deliberate differences: launch text rejects control characters and
has local resource bounds. The command prefix is represented as program plus
arguments rather than an untyped argv array. Model metadata in this first
contract includes identity, label, default, visibility and thinking choices;
descriptions, aliases and token-window metadata remain a later extension.

## Catalogue owner checkpoint

The registry resolves built-in, configured and inert plugin definitions before
clients are built. Model merge and native-option catalogue conversion are pure
harness policies. The agent's provider probe reuses the existing prompt-free
probe, clearing model/resume/options/MCP inputs; fake ACP is the only process
used by its test. The node cache and bounded async owner are staged under
cfg(test), pending request wiring. Source tests cover passive cache misses,
host-versus-workspace coalescing, forced refresh generations, plugin replacement,
finite deadlines, shutdown and pending-call settlement; their execution is still
pending. The first local narrow compile reached harness policy and found a
similar-name lint, now corrected.

Differences at this stage: passive diagnostics report NotChecked rather than
starting background discovery; all discovery work, including admission wait,
shares one finite catalogue deadline. Typed failure classes replace raw provider
error prose so credentials cannot cross a diagnostic boundary. Configuration
override iteration is deterministic by provider ID. Native ACP/direct mode IDs
remain the ones observed by the transport, and are not silently translated to
the reference manifest's mode dialect. This is a policy/discovery stage, not
complete Claude/OpenCode/Pi/OMP native execution parity.

## Storage preparation

Quality B was merged cleanly before method wiring. The existing settings-file
preservation and recursive unknown-field comparison are extracted behind
crate-private path-taking operations without changing the settings methods or
their test expectations. The added-agent file trust check becomes crate-visible
for reuse by provider persistence; its checks are unchanged. This avoids a
second implementation of owner-only backup naming and nested-field detection.

## Provider persistence checkpoint

The provider file reuses the settings backup/unknown-field operations and the
added-agent file trust check. It bounds reads, refuses links and nonprivate
files, preserves later and unreadable bytes before replacement, and writes via
the existing owner-only atomic helper. Synthetic tests cover round trips,
Unix ownership/modes, nested future fields, malformed/oversized input, symlinks
and failed preservation leaving the original intact. These tests are pending
execution. The final broad narrow compile still stopped on an option-variable
similarity lint before node tests; the identifiers have now been separated.

## Shared thinking-choice storage

Native probes may offer the same thinking choices for every model. The checked
list and its ID index are now shared behind Arc instead of allocating a list
per model or rescanning it for every default. Model clones retain this shared
observation; serialization still emits the same ordered list. A regression
checks actual sharing and decoder parity. The storage narrow compile reached
an unused read-count result in the new bounded file read; that result is now
explicitly retained while the buffer length enforces the read bound.

Native denied-tool patterns now have their own validating type instead of
crossing the launch-policy boundary as argument strings. The wire/local-file
shape remains a string array, and an adapter still must explicitly enforce the
policy before receiving an executable command.

## Model selection preflight

The pure selection policy reuses the checked catalogue and its per-model
thinking index. Explicit unknown models and efforts refuse the whole selection;
an effort offered by a different model is not accepted. Defaults follow the
selected model, with the first selectable model as fallback. Compatibility-only
rows remain available explicitly and do not become implicit defaults. Missing
discovery leaves the native model unset rather than fabricating a choice. A
model default effort must belong to its actual offered list, including when the
list is absent. This helper grants no execution authority; RPC/create wiring
must supply the node's own current catalogue. Scoped tests are pending.

The OpenCode provider kind serializes as `opencode`, matching the manifest ID.

## Native Codex catalogue transaction

Paseo codex-app-server-agent.ts:3213-3250 and 7700-7935 initialize one native
process, read model/list and saved defaults, and build per-model thinking
choices. Reusing the existing session probe would open a throwaway thread and
only report mode choices. A dedicated transaction therefore reuses the existing
Codex wire/protocol codec, without changing session or option-completion arms.
It follows pagination under fixed page/model/message bounds, reads defaults,
and cleans up the process group and readers under the supplied finite deadline.
The fake process's request audit must contain only initialization, model pages
and configuration reads. No live Codex or account is used.

Deliberate differences: required malformed model data refuses the observation
instead of producing a successful empty catalogue. Optional config methods may
be absent only through typed method-not-found; other failures remain failures.
Native runtime mode IDs retain the driver's existing codex-acp-compatible
dialect. The catalogue metadata boundary ignores unknown additive fields and
does not introduce SDK/dependency or real-agent schema generation. Execution
of the added codec and process tests is pending.

## Provider runtime lifetime prerequisite

Tracker row 508 (persistent helper identity and reapStale for OpenCode runtime)
is owned here under A9 following the coordinator's 07:35 UTC ruling. Its process
identity and stale-process cleanup must consume the workspace track's forthcoming
vessel-proc ProcessLifetime. No second reaper or parallel process ledger is
introduced. Runtime implementation waits for that typed seam, while the provider
registry, discovery and native transport work proceeds independently.

The subsequent durable-identity ruling assigns vessel-proc's serializable
pid/kernel-start-time/boot-id/executable proof to nodectl. Its verify operation
returns Same/Gone/Reused, with explicit Unsupported outside Linux. This package
owns only the provider-helper ledger in the node data directory and restart
reap policy: signal solely after Same verification. A name or bare PID never
authorizes a signal. ProcessLifetime remains the separate in-process observer.

## Cache subscription boundary

The owner now projects passive snapshots without discovery and publishes typed
registry invalidations as well as scoped observation notifications. A lagged
subscriber rereads its own scope. Full execution capacity refuses only new
work; disabled, ready and coalesced callers still receive the corresponding
cached observation, with no transient loading slot left after refusal. Tests
cover those states and configuration changes after passive subscription.

## Configuration owner

One bounded edit mailbox owns the full saved generation and plugin registration
set. It validates the next registry before writing, saves through the existing
provider file helper, then publishes the new cache generation. Dropping a caller
after admission does not cancel that sequence. Concurrent enable edits merge
against the latest committed map; invalid registry or file failure retains the
previous runtime generation. Plugin reload uses this same serialization point
and never writes an inert plugin contribution into the owner's configuration.
The startup constructor creates both owners from the same validated file and
returns both tasks for node lifecycle tracking. Startup now applies the same
recovery rule as settings: unreadable bytes are moved aside before manifest
defaults are selected. Failed preservation and unsafe files refuse startup;
neither is silently overwritten. The typed recovery status can be logged
without disclosing content or a private path.

Synthetic tests cover concurrent changes, reopen, abandoned waiters, invalid
generation, a nonregular file target and plugin collision. Execution is pending;
node service wiring and its authorization checks remain separate unfinished work.

## Explicit runtime producer

Admitted work resolves only its immutable provider definition, explicit account
homes, launcher lookup and named secrets supplied by the existing fleet source.
Native protocol arguments follow the configured prefix. Configured environment
keeps precedence over account home defaults. Discovery does not inherit the
daemon environment implicitly; provider settings supply any additional required
variables. Unsupported driver, absent launcher and unhonorable launch policy
remain distinct states. A missing deny-policy adapter refuses before spawn.
The producer exposes the generic owner hook to a future plugin runtime without
executing an inert plugin definition. Fake Codex request audits cover this
whole lookup/launch/probe boundary. These tests and service wiring are pending.

## Startup assembly preparation

Before provider service wiring, existing blocking startup preparation and
attachment construction are moved intact into node/prepare.rs and
node/attachment_setup.rs. This is a responsibility split with no initialization
order, behaviour or assertion change. node.rs falls below the regular budget,
so its shrinking-size allowance is removed rather than increased.

## Initial-prompt output schema

CLI run.ts:638-654 passes outputSchema through createAgent, while server
create-agent/create.ts:252-258 places it in the initial prompt's AgentRunOptions.
It is not durable launch/session state. Native Codex puts it on turn/start
(codex-app-server-agent.ts:4288), and OpenCode on the prompt's format payload
(opencode-agent.ts:4025-4029). W17 retains CLI semantic validation and retries.

The bounded OutputSchema value reuses RawJson for opaque schema content,
without a new dependency or a second schema AST. The node wire representation
is JSON text (a string), distinct from the vendor's JSON object. The text bound
is checked before native JSON parsing/retention, and root scalars/boolean schemas
are refused because this creation contract takes an object. Debug/errors hide
content; the native encoding accessor is only for a tested supporting adapter.
No output-schema capability, creation field or prompt transport is enabled by
this value alone. Existing CLI refusal remains in force until native preflight
and transport are implemented and reviewed; no supplied schema is dropped.

## Pi native parser preparation

Pi rpc-types.ts and agent.ts:1122-1178,2790-2850 define one available-models
observation plus the active native model. The new test-only agent::pi catalogue
decoder preserves provider/model identity, reasoning availability and the native
thinkingLevelMap. Explicit null removes a level; xhigh/max require an advertised
mapping; the medium default clamps upward first and then downward, as Pi does.
Nonreasoning models offer no thinking choices and Pi offers no permission modes.
Unknown metadata is ignored inside the native decoder; malformed required data,
oversized input and inconsistent empty reasoning choices refuse the observation.
This last refusal replaces the reference's nonexistent `off` default when every
level has been disabled. Provider identity containing a separator is refused
instead of creating an ambiguous native key.

This stage does not add a Driver variant, mark Pi/OMP built/selectable or change
the existing not-built assertions. Native process framing, turn events, UI
requests, history and fake-harness execution remain unfinished. No live Pi or
account file is used. The parser's tests are pending execution.

## Pending service composition checkpoint

[Decision Log]
- Purpose: make one provider owner reachable from node lifecycle, typed clients
  and trusted diagnostics without a second collector or arbitrary config RPC.
- Existing assets: Quality B's optional method version and generated request,
  reply/handler table, Principal scope checks, TaskTracker, and the existing
  serialized provider owners/file helpers cover the required boundaries.
- Alternatives: a separate diagnostics collector, accepting the lenient file
  override DTO over RPC, or starting probes during startup/cache misses.
- Choice: modify those existing boundaries; no dependency or schema number.
  Append provider.list, provider.diagnostic and provider.set_enabled as pending
  rows using the single PROVIDERS=None constant. Strict wire requests differ
  from forward-compatible owner file decoding. Only targeted enabled edits are
  remotely expressible; trusted plugin contributions use a local typed handle.
- Why: absent protocol assignment refuses both direct and nested calls before
  mutation. Cached diagnostics remain usable without process/credential access.
- Effects: normal node startup validates/recover-preserves providers.json under
  its data lock, then tracks configuration/cache owners and shutdown. Cached
  requires Read, active Discover/Refresh additionally local person/Operate;
  enabled mutation requires Admin and a local person. Current delegation policy
  refuses active discovery and enabled changes rather than weakening authority.

Discover joins or reuses an observation; Refresh explicitly supersedes one.
The total snapshot budget and each admitted observation are bounded at 120s;
capacity, supersession and shutdown remain typed refusals. Safe diagnostics
contain cached entry/version only, never launch config, stderr, login output or
private error causes. Unlike Paseo's getProviderDiagnostic, this request does
not force an unrelated global catalogue refresh; that difference is explicit.
Cached miss stays NotChecked. Trusted ProviderRegistrations.cached shares this
projection with host diagnostics, and changes are scoped invalidation hints;
lagged readers reread and retained handles refuse after owner shutdown.

The required node40 merge is7a4160b2. Head model setup remains in node/setup.rs,
preparation uses that same helper and explicit tracing target; both architecture
appendices remain intact. The inherited generated TypeScript trailing spaces
were preserved at merge. New declarations will be regenerated through the
existing Rust generator, never hand-maintained. No existing test assertion or
size allowance is raised. New pending-envelope, authority, cache-no-probe,
durable-enabled and tracked-shutdown tests are drafted; execution is pending.

Still unfinished: authoritative create-config/session inheritance binding,
per-turn output-schema transport, native missing drivers/permissions, actual
plugin session adapter, and persistent provider helper ledger. The ledger must
consume nodectl's reviewed seeded identity producer and pidfd-only verified
signal; ProcessLifetime is solely an in-process observer. Root verified signal
is not checked process-group cleanup and cannot acknowledge whole-group reap.

The first immutable wiring test run626f12f79 stopped in nested-vesseld compilation:
three new reply variants were missing from agents/older.rs's exhaustive older
agent projection. No product test ran. That projection now explicitly classifies
the three pending provider replies alongside other newer methods; shared protocol
admission still refuses unassigned calls. Red evidence stays in
/tmp/work/wp-providers/provider-wiring-tests. The own waiting local626 check was
cancelled before compilation and is replaced on the changed head.

## Node42 integration checkpoint

Exact f55a2cf3 passed remote provider selection70/0/0 (run
wp-providers-20261115T095315Z-f55a2cf3; all prerequisite steps0), including
output-schema bounds/codec, startup preservation, pending envelope refusal,
service authority and tracked owner shutdown. Agent Pi/catalogue_data modules
and standalone provider_catalogue integration are not in that filter.

The required d432ef0f merge adopts reviewed ProcessLifetime and the reviewed
transient system-prompt prerequisite, plus shared lifecycle call preflight.
Both conflicting public exports are retained. Both architecture appendices
remain verbatim, designated head first. Own full AgentSetup constructors gain
only system_prompt:None: producer.rs and the two provider_catalogue integration
setups; no head assertion or size record changes. The existing no-prompt probe
normalization additionally clears the new field before dispatching either
native Codex or ACP, so catalogue observation cannot inherit transient context.
The native catalogue already ignores other launch-only selections; moving that
branch after normalization preserves the observation contract. This merge's
new combined source still needs strict compilation and TS regeneration. The
own queued f55 local job was cancelled before any compilation, avoiding a false
exact-head claim after the required base change; its red/queue logs are kept.
## Codex command-owner dependency preparation

W12b supplied checked source bec742aa1 (strict five-crate Clippy0, command unit
14/0, Codex/options fake integration29/0). Its skill discovery and typed command
projection are consumed intact in an isolated preparation tree, while the main
626f12f79 check remains frozen. No native model/config producer arm is removed.
Merge conflicts preserve both codex catalogue/commands modules, both refusal
classifications and fixtures, the head SessionEvent optional admission check
plus W12 command projection, and both documentation appendices. The head's
single table stays in rpc/table.rs; the W12 pending draft row moves to its end
without a number. No source budget is raised or existing assertion edited by
this merge. Combined compilation remains owed before this dependency reaches
main or any consumer-ready claim.

## Codex native MCP preparation

Paseo codex-app-server-agent.ts:903-925 and5387-5424 put the resolved stdio/HTTP
MCP map under thread start/resume config.mcp_servers. Reuse Vessel's existing
ResolvedMcpServers and secret wrapper in a native-only Serialize adapter; do not
resolve again or flatten into process environment. Empty snapshots omit config,
preserving native defaults. Native config Debug shows only a count. The positive
fake audit records exact synthetic start/resume payloads into a private temp
file, including literal arguments, stdio env and HTTP headers, without executing
any MCP helper or contacting an endpoint. The native capability is staged here;
old unsupported assertions stay until that positive test actually passes, then
will narrow to the still unsupported choices in the same capability commit.

The positive native MCP test actually passed locally on2026-11-15:
/tmp/work/wp-providers/native-mcp-positive.log,1/0/0, including both start/resume
with configured stdio+HTTP and empty omission. Cold compilation4m06s and the
fake preparation inside testkit's pre-deadline setup account for its94.48s test
summary. Only testkit processes ran, with synthetic environment/header values.
After that proof the old combined omission assertion is narrowed to its still
unsupported effort-choice half; its unchanged refusal assertion remains.
The capability commit explicitly names removal of the old left-out expectation.
The separate Debug unit test and combined Codex/options/catalogue checks remain
owed; this single positive test is not a package-wide gate or reviewed handoff.

The native MCP/checked command ingress merge into node42 retains both plan
appendices and the tighter723-line designated-head client allowance. It keeps
W12's reduced939-line Codex session allowance and the designated head's removed
handshake allowance; none is raised. The new MCP integration AgentSetup literal
gains only system_prompt:None. No further existing assertion changes occur in
the merge; the real capability commitf6403fa48 contains the authorized omission
expectation narrowing and positive proof. Combined checks remain pending.

The grandfathered local TS generation check stopped at a merged dependency's
missing browser classification for GetDraftCommands, before generator tests ran.
It is now classified with other ordinary unary capability queries; the node's
person/Operate validation and pending protocol gate remain authoritative. No
browser-specific grant or bypass is introduced. The original red log is retained
at /tmp/work/wp-providers/provider-wiring-ts-node42.log. All new generation and
checks are remote-only; the existing remote `ts` step exports generated bytes,
patch, commit/status and hashes even when freshness correctly reports a diff.
The pending fa4 strict/integration replacements were already registered when
this earlier local compile result was inspected; retain their terminal evidence,
then use a changed head rather than replaying an obsolete snapshot.

## Node44 integration and test-helper correction

The required ede20014 merge keeps the one shared AgentEvent.required_protocol
accessor, its inner-event projection even under an assigned enclosing event,
and W12's independent CommandsChanged kind projection. The two guards match
different variants and remain in designated-head-first order. No duplicate
accessor, number or assertion is introduced. Other architecture additions merge
intact, including exactly W16's reviewed value-only plugin boundary.

Post-resize strict fa4 also found new MCP integration helpers outside an explicit
cfg(test) module. The corrected file now marks helpers test-only and lists all
ignored AgentOutput variants without lint allowances. Assertions and native
request bytes are unchanged. The first edit script stopped before adding its
evidence paragraph; this checkpoint supplies that missing record. Existing
red outputs remain in provider-mcp-clippy-node42-resize-recovery. No new local
cargo/rustc invocation is permitted; the already waiting TS job finished with
the previously recorded browser-classification error, and all new work is remote.

## Protocol assignment and account identity boundary (2026-11-15)

Integrator correction7f9077bb supersedes the earlier protocol24 reservation:
host diagnostics received24, and PROVIDERS receives the next free number in
merge order. The constant remains None and no client may send these calls until
the reviewed, whole-gated wiring merge announces an assigned head. MIN remains2;
providers.json needs no SQLite schema number. Only the three provider rows use
this constant. The future wiring commit assigns the constant and current
protocol together and regenerates TypeScript, with the assigned version changelog.
A likely number is not authority for a client or a generated transport.

[Decision Log]
- Purpose: keep provider display identity distinct from credential/account and
  installation authority while the modern provider boundary is introduced.
- Constraint: the discovery producer resolves a ProviderDefinition's launch
  settings with explicit node HarnessEnv, without selecting a legacy profile
  or AddedAgent. Built-in entries can have command/environment overrides.
- Alternatives: join by provider ID, implementation kind, or display name; all
  can associate an alias or plugin with another installation's account.
- Choice: the current snapshot deliberately exposes no HarnessRef/profile
  binding or sign-in verdict. SnapshotSource::Custom does not imply removable
  AddedAgent; Builtin does not imply a particular legacy credential context.
- Reason: an unavailable binding must remain unknown until its actual actor
  can attest the launch/account identity. A catalogue handshake is not a
  successful authentication check.
- Effects: G3 initially displays modern provider models/diagnostic/enable apart
  from legacy installation/account controls, keyed by their own returned
  HarnessRef/profile. No account, sign-in, defaults or removal join is inferred.

Coordinator's next core increment requires an actor-owned typed binding and
actual known/unknown sign-in verdict, then one combined row per provider. It
also assigns W22 the name-overlap settlement: current AgentName reserves only
the four HarnessKind names, so copilot/pi/omp remain valid legacy Added names
in a separate registry, while the modern registry reserves its six built-ins.
This checkpoint has not changed that legacy rule or mapped the registries;
the next increment must enforce one explicit node rule with a typed refusal
or genuine mapping, before any shared-identity claim.

Exact fa4afdc4 agent selection passed139/0/2ignored on the remote VM (run
wp-providers-20261115T104558Z-fa4afdc4). The two external-installed Claude SDK
contracts remain ignored. The selected MCP start/resume, native catalogue and
options fakes use isolated homes and synthetic values only. Exact ddc587212
strict seven-crate Clippy and remote TypeScript generation are queued; the
earlier fa4 strict red and infrastructure143 results remain recorded. No latest
whole-gate or provider-core V3 pass is claimed. Native mode preparation76364ecd
passed113/0/2ignored separately; it has not been adopted into this checkpoint.

The exact ddc remote generator ran all4 TypeScript tests successfully and then
correctly failed freshness for proto.ts. The exported source commit matches
ddc; all6 SHA256SUMS entries verified before applying its exact wire.patch.
No generated declaration was edited by hand. The historical freshness-red run
wp-providers-20261115T111247Z-ddc58721 and its artifacts remain retained; the
combined whole gate will verify the now-committed generated bytes.

## Core preflight correction before the approved replacement

The first whole submission47b3 was still queued when an own source audit found
two pinned RPC policy lists missing the newly added rows. Coordinator explicitly
authorized cancelling ONLY that queued run via cancel-queued and one corrected
replacement. The helper verified no acquired slot and terminated that exact
process group; terminal state is cancelled143, not a compiler/test result.
All original records remain under provider-core-whole-node44. Existing policy
list entries stay intact: append only provider.set_enabled to Admin and
provider.list/provider.diagnostic/agent.draft_commands to FORWARDABLE. No
permission is weakened and no failed test assertion is removed.

A bounded-core audit also adopts the already positively tested native metadata
counter: charge owned strings/vector/cursor storage across model pages before
retaining the next page, with one4MiB bound. A count limit plus per-frame bound
alone allowed excessive multi-page retention. The same exact catalogue_data
unit source passed in immutable763's110-unit agent run; no native mode/version
capability is adopted here. Its paired retired-configured-model regression
preserves the actual native default when the saved ID is absent, matching
Paseo's hasConfiguredDefaultModel check. The complete corrected core still
needs its own whole gate and exact V3 review. No number is assigned locally.

## Strict all-target correction preparation

Exact ddc seven-crate strict check returned source errors: client FakeNode's
explicit unsupported handler list omitted the3provider methods and W12 draft
method; node start exceeded100function lines after wiring; an Arc clone, a
clone assignment and a doc code token violated strict lints. These are own
integration omissions, not infrastructure failures. The original run/logs remain
provider-core-strict-node44. The whole was submitted too early, before this
strict result; future whole replacement waits for the corrected strict pass.

[Decision Log]
- Purpose: preserve one shared connection/shutdown ownership context while
  keeping async node-start ordering within the existing function budget.
- Constraint: no size/line allowance may be raised to accommodate provider wiring.
- Alternatives: increase the budget, suppress the lint, or compress formatting;
  none improves the bootstrap responsibilities.
- Choice: move only synchronous frontend handle assembly into node/frontends.rs.
- Reason: RPC/secure admission, connection tracking and teardown already share
  these handles; their construction has one cohesive ownership purpose.
- Effects: the same stop tokens, trackers, service and limits are preserved.
  No new I/O/task start or await is introduced. Existing bind/serve ordering
  remains in Node::start. Other corrections change no production policy and
  the fake handlers continue explicit Unsupported instead of pretending support.

The explicit Handler inventory contains exactly the guarded node and two test
fakes. The RPC test fake receives the same4explicit Unsupported handlers as
the client fake, preserving its call-recording helper. This completes that
inventory instead of waiting for the whole workspace compile to discover the
second fixture. The already submitted seven-crate strict source remains
immutable; RPC test compilation is an additional narrow check before any new
whole submission. No existing assertion or method policy is changed.

## Required node46g merge

The node integrator announced82ae7c91 after the known-red whole was explicitly
cancelled and before any replacement whole started. Merge that head under A8,
without rebasing or dropping earlier red/cancelled evidence. It carries staged
method-row support, W2 ProviderChildRef values and StopSignals. The sole source
conflict is lib.rs public exports: preserve both provider handles and the
head's StopSignals/signal exports. Real provider rows remain pending, not staged,
and retain their destination handlers and separate named feature constant.

The announced gate46g reports Rust2389/0/12, web e2e68 and forge4, with one named
web load timeout counted by the coordinator; it is not evidence that this new
combined provider source has passed. Existing c2c3 strict/RPC34 runs keep their
immutable source and finish unchanged. The merged head needs narrow union
qualification before the one approved replacement whole. No number, schema,
assertion or size allowance is changed by this merge.

## Encoded reader projection budget

[Decision Log]
- Purpose: a bounded retained catalogue must also be readable through the actual
  protocol frame. Arc-shared thinking choices repeat for every model in JSON.
- Constraint: a valid shared observation can therefore expand beyond the16MiB
  transport frame, even when decoded metadata and model/choice counts are bounded.
- Alternatives: allocate the full frame then reject, silently truncate choices,
  or add paging during this metadata increment. The first risks large allocations;
  the second misreports Ready; paging is a separate future contract.
- Choice: use serde's existing serializer with a std::io::Write counting sink,
  bounded at half the canonical vessel_rpc::MAX_FRAME for envelope headroom.
- Reason: count actual escaping/structural overhead and repeated shared lists
  without retaining encoded text or another JSON tree/dependency.
- Effects: oversized individual observations become whole InvalidCatalogue
  errors before Ready publication. A too-large aggregate returns typed Unavailable
  with individual diagnostics still usable; no entry/catalogue is silently dropped.

New regressions cover exact escaped-byte boundary, shared-thinking expansion
refused before Ready, and individually readable catalogues whose aggregate is
too large. They are drafted, not executed. No wire shape, protocol number or
existing assertion changes. The main whole is not active; this source will enter
the planned merged-head narrow check before the approved final whole.

The c2c3 strict run actually reached two unnecessary qualifications in the
previously unit-tested metadata counter: size_of is already in the Rust2024
prelude. Remove only the std::mem qualification; no counter arithmetic or
assertion changes. Original red134s log remains provider-core-strict-fixed.
The next immutable merged-head strict selection also includes vessel-rpc, and
the newly added encoded-budget regressions must run before the final whole.
No rerun of unchanged c2c3 and no whole submission is made on a known-red head.

## Latest strict correction (2026-11-15 14:28 UTC)

Exact db553 all-target eight-crate strict failed: the frontend extraction left
an unused SecureGateway import, the encoded-byte constant used disallowed raw
integer division, and fake/test helpers discarded builder/results or cloned an
Arc implicitly. Remove the unused import, use checked integer division with a
fail-closed zero branch, and explicitly discard those existing results/use
Arc::clone. The byte limit remains exactly half the same even frame bound;
process policy, assertions and transport payloads are unchanged. Original
strict logs remain provider-core-strict-node46. The already queued db553 runtime
selection stays unchanged; this correction needs a new strict selection before
any replacement whole gate. No strict/runtime/whole PASS is inferred here.
