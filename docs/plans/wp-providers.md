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
Next: typed values and skeleton, pure manifest/resolution/launch/registry,
catalogue owner and fake checks. Wiring waits for the shared method-table head.
