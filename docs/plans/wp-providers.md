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

## Resume point

Done: inspected the reference and existing harness/probe boundaries; mapped
provider behaviour above; requested the missing node-call allocation once.
Skeleton: provider identity, bounded mode catalogue, built-in manifest and
create-config policy are implemented with reference decision tests. Narrow
compile/tests are pending. Next: launch/registry/catalogue owner and fake checks.
Wiring waits for the shared method-table head. The integrator approved a
separate owner-only provider configuration file without a SQLite migration; it
must reuse the settings file preservation and atomic-write rules.

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
