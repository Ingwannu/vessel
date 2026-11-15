# Composer command discovery on the node

Status: implemented source with scoped checks; native integration, protocol assignment and whole-package review remain pending. ADR0085 records reference paths and capability limits.

[Decision Log]
- Purpose: preserve provider command/skill identity and list a complete draft's commands without sending a prompt.
- Existing assets and constraints: reuse AgentCommand, provider reports, the bounded probe handshake/cache, workspace authorization and the sole RPC table. Older peers cannot expose skill metadata reliably.
- Alternatives: launch a conversation with a synthetic prompt; infer skills by a suggestive name in the UI; create a second provider registry.
- Choice: classify at the existing provider edge, retain reports through prompt-free probes, and add one pending typed unary call. No dependency is added.
- Why: discovery must apply the same provider/profile/directory/model/options as creation while retaining its cancellation and credential lifetime.
- Effects: old/unassigned peers see no new metadata; unsupported native drivers refuse before spawn. No empty catalog is fabricated when a provider has not reported one.

## Ownership and data flow

`vessel-proto::composer_commands` owns optional CommandKind, DraftCommandConfig, GetDraftCommands and CommandCatalog. The pending Operate/unary agent.draft_commands row is at the end of rpc/table.rs after the gate41 merge. Only the integrator assigns COMPOSER_COMMANDS; there is no invented protocol number.

The node resolves the authorized WorkspaceRoot and existing relative directory, validates model/options, and uses the complete draft hash in the existing Probes cache. Concurrent identical callers share a probe; an active different draft for that provider/profile is refused, and idle superseded configurations retire. The existing history-discovery semaphore bounds starts. Model proxy leases retire on cancellation through their existing drop guard.

Catalog probes wait for both Ready and Commands in either order. Ordinary option probes retain their existing Ready behavior. Explicit unsupported drivers refuse before process launch; a failed or missing report is a typed failure rather than empty success.

ACP namespaced metadata is converted by vessel-agent. Claude flat-list classification goes through the sole vessel-harness descriptor, shared by live events and probes; explicit metadata, including Unknown, wins. Codex's native skills/list catalog uses typed provider paths and converts only an exact leading selected skill into the native Skill block plus text, in both prompt and steer. Inline/unknown/attached ordinary prompts keep the existing path. Its unchanged initialization/opening bodies live in codex/session/handshake.rs, shared with W22 provider work.

Old/unassigned metadata projection covers direct conversations, events and nested forwarding envelopes. SessionEvent retains the canonical optional required_protocol admission from the gate41 head while applying command metadata projection. Timeline client-message receipts remain W23's separate ADR0084 contract.

## Checks and practical limits

- Values snapshot57f: remote proto tests4/0.
- Exactbec742aa: strict all-target Clippy0 for proto, harness, agent, session and node.
- Command-focused unit suite14/0; fake Codex/options integration29/0 (17 Codex,12 options). Logs: /tmp/work/wp-composer-shell/catalog-command-unit-tests.log and catalog-agent-integration.log.
- Base3 merged at a25e73d83. Gate41 canonical913f05d8 is now being merged; its combined source checks are pending. Cache tests were waiting, cancelled by verified owned PID before this merge, and must be requeued on the merged head.
- Codex schema generation used installed0.169 with isolated HOME/CODEX_HOME and an otherwise empty environment except PATH. It sent no session, network or model request. The full generated primary schema is retained under /tmp/work/wp-composer-shell/codex-schema/generated; pinned contract rerun remains pending.
- No TypeScript refresh, whole root/native gate, capture or final V3/V4 review has passed for this package.

Native custom prompt files, filesystem fallback and compact/goal execution remain explicit capability gaps. ClaudeStream/OpenCodeHttp native drivers remain unavailable. W22 owns model/config/provider coverage; no duplicate registry or guessed provider behavior is introduced here.

Gate44 merge67c kept the shared optional AgentEvent protocol projection and B2
command metadata masking; exact older d9 strict5crate and cache4tests remain prior
source evidence. TS regeneration67c compiled the root dependencies but failed at
vessel-web's exhaustive forwardability match, before generation (wire.patch empty).
The draft command query is now explicitly forwardable beside HarnessOptions:
node Operate/workspace validation and NodeClient's pending-version gate remain
authoritative. The bridge grants no scopes or path authority. Its existing full
refused-method fixture covers the new decision. Corrected TS generation is owed.

Agreed with W22/W23: AgentCommand.native_command is additive optional typed
NativeCommandKind metadata. It is independently masked beside kind under the
unassigned COMPOSER_COMMANDS gate, including native-only/missing-kind reports,
conversation snapshots, events and forwarded envelopes. Existing ACP and skills
producers explicitly leave itNone; names/labels never infer availability. Only
W22's proven current-run transport may fill it, after W23 receipt acceptance is
checked. The composer/actor still compare canonical name and the shared parser.
Goals additionally require launchflag plus autonomous root-turn proof. Dormant
valuesdd06 are merged for NODE-side preparation; no app activation/source-pass
claim. Metadata and corrected browser policy need combined check/TS generation.

Whole-consumer audit: both existing client/RPC fake handlers now include the new draft command query in their `unserved!` lists. They retain the same typed Unsupported result and RPC call recording behavior. This closes a test-target trait requirement missed by the earlier five-crate selection; it does not add a second fake or authorize discovery. The browser unary policy already classifies GetDraftCommands beside HarnessOptions (6e893096c). Current combined consumer lint remains pending.
