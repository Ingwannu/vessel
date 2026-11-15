# Composer command discovery on the node

Status (2026-11-15 18:25 UTC): separate node development source with bounded checks, not adopted App runtime. Official App roots exclude this private discovery/RPC ancestry. The coordinator-approved minimal kind-only carrier on node93970 is a separate producer; native consumes it through an announced base. Protocol assignment, current whole-package review and native acceptance remain pending. ADR0085 records reference paths and capability limits.

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
- Command-focused unit suite14/0; fake Codex/options integration29/0 (17 Codex,12 options). The retained command-unit and agent-integration logs name the exact selected source.
- Base3 merged at a25e73d83. Gate41 canonical913f05d8 is now being merged; its combined source checks are pending. Cache tests were waiting, cancelled by verified owned PID before this merge, and must be requeued on the merged head.
- Codex schema generation used installed0.169 with isolated HOME/CODEX_HOME and an otherwise empty environment except PATH. It sent no session, network or model request. The full generated primary schema is retained with the command survey evidence. The pinned schema-only contract on67c later passed1/0 in `tools-codex-contract-20261115T124102Z-67c5eb77`; that proves only the pinned schema, not current command transport.
- Node52 wire equality on680a passed3 proto and2 web tests with zero generated-file changes in `wp-composer-shell-typescript-20261115T172034Z-680a966a`. No whole root/native gate, capture or final V3/V4 package approval has passed.

Native custom prompt files, filesystem fallback and compact/goal execution remain explicit capability gaps. ClaudeStream/OpenCodeHttp native drivers remain unavailable. W22 owns model/config/provider coverage; no duplicate registry or guessed provider behavior is introduced here.


Current execution limits: exact e319 eight-crate TS-enabled strict passed. Its full Node library execution passed311 and failed one diagnostics socket fixture under the runner's pre-fix long temporary directory; the runner owner confirmed that infrastructure defect. The original failure is retained, with no diagnostics guard/assertion edit or automatic unchanged rerun. Catalog Serde diagnostic privacy and physical blocking-preparation ownership have separate W22 corrections pending their actual strict/runtime handoff. A dropped probe waiter alone is not evidence that its blocking launch lookup released capacity. Native compact/goal offers remain unavailable until checked live provider transport and keyed W23 acceptance.
