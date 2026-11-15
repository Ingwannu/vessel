# ADR 0085: Composer command catalogs and retained editing

Status: proposed

## Purpose and intent

Give the native composer the reference application's client, plugin and provider command behavior, including inline skills and command discovery before creating a session. Keep one text engine across docked and fullscreen presentations.

## Existing implementation and constraints

At proposal time, `AgentCommand` carried only name, description and argument hint. Existing sessions received commands through their event log; harness probes discarded command reports. The native composer already has ranking, recall, draft persistence and a shared TextField. Plugin invocation, pane lifecycle, file navigation and hardware audio have separate owners.

The reference is Paseo 0.15.4: `packages/app/src/client-slash-commands/index.ts`, `utils/agent-command-autocomplete.ts`, `composer/input-mode.ts`, `composer/input/state.ts`, `composer/input/input.tsx:1205-1224`, and `packages/server/src/server/agent/providers/claude/agent.ts:445-450`. Claude reports one flat command list; the reference classifies known session controls as commands and the rest as skills. Unknown metadata must never be guessed from a suggestive name by the UI.

## Alternatives considered

1. Infer skills in the UI and reuse a stale catalog for drafts. This cannot distinguish provider controls reliably or account for draft configuration.
2. Start a full session whenever the menu opens. This creates durable sessions and can start model work without a send.
3. Extend the existing bounded, prompt-free probe and typed command reporting. Share the existing editor, ranking and link infrastructure.

## Decision

Choose the third option. Add a typed optional command kind to provider command reports, parsed at the provider edge. Add a typed draft command query carrying the selected harness, profile, registered workspace root, validated relative directory and option choices; the node uses the same launch validation and prompt-free handshake path as its probes. Bound all waits, retain command order, coalesce equivalent concurrent requests and invalidate configuration changes. Existing-session catalogs continue to use their actor's facts.

The new protocol feature has one `Option<ProtocolVersion>` constant, initially `None`; the integrator assigns its number. No new call is registered until the shared method table supports pending rows. While unassigned or for an older peer, command-kind metadata is removed from direct, nested and forwarded conversation snapshots and events. Absent kind decodes as unknown and is never offered inline. Old requests and event command lists retain their meaning. Tests cover old-to-new decoding, new-to-old projection and pending refusal.

Draft probing starts no model turn and advertises no questions or MCP servers. It uses the node's selected profile credentials, never caller-provided environment variables. The authorized root, relative directory, option shape, count and duplicate identifiers are checked before spawning; the provider validates offered choices during its prompt-free handshake before Ready; it must not bypass workspace trust or creation checks. Failures are typed, displayed beside the command menu and do not become a fabricated empty success. Windows uses the existing cross-platform process abstraction and typed refusal of unavailable drivers.

Built-in commands are exact, attachment-free tokens. Built-in names and aliases win over plugins, then plugin names win over providers. Inline completion offers provider skills only. Client commands route through the lifecycle owner's correlated archive path. The same draft and editor remain mounted across fullscreen; covering voice surfaces, send and Escape/back exit fullscreen. Preserve-and-lock disables editing until submission settles. Attachment and paste hooks remain installed.

## Why this instead of the alternatives

The provider edge has the only trustworthy command metadata. A bounded probe shares authentication, option validation and shutdown behavior without creating a conversation or model call. One editor preserves selection, undo and composition behavior, and one attachment and lifecycle path prevents drift.

## Advantages, costs and effects

No new external dependency or control is introduced. The wire changes are additive and dormant until assigned. Provider implementations without command discovery return a typed unsupported result. Draft discovery may start a provider process and leave its normal empty-session trace, as existing probes do; it cannot send prompts. Catalog caching must include the full draft configuration. Package integration depends on the shared method table and the base native control split.

## Verification and rollout

Use fake harnesses only. Verify command conversion, invalid configuration refusal before spawn, concurrent probe coalescing, old-peer projections, start/end deadlines, cancellation cleanup, and native IME/send/menu/fullscreen transitions. Run narrow checks per commit; complete root and native gates and inspected captures once at package close. No gate or runtime result is implied by this proposed document.

The draft catalog uses the existing `WorkspaceRoot` and `RelPath` contracts. An unregistered arbitrary absolute/home path is refused until registered through the workspace flow. This is a stated authority floor compared with the reference's raw cwd; the Files and draft discovery paths must not disagree about which roots they may read. Native drivers without an implemented prompt-free catalog produce Unsupported rather than inferred catalogs.

Draft discovery requires Operate scope and refuses agent principals. It shares the existing bounded agent-discovery semaphore with history listing. The node caches the latest draft configuration per provider/profile, keyed by every draft field and provider version; identical concurrent requests coalesce, a different in-flight configuration returns Unavailable. A scope guard retires routed model credentials even when the query future is cancelled.

Claude's flat command list follows the reference's explicit fallback in `claude/agent.ts:420-450`: ten known root/session controls are Command, remaining reported names are Skill. This provider fact lives behind `vessel-harness`'s sole descriptor dispatch and applies identically to live actor reports and draft probes. It fills absent metadata only; explicit Command, Skill or Unknown values are preserved. Other providers and added agents have no such fallback. No command is synthesized if the provider did not report it.

Codex app-server discovery uses `skills/list` after thread open without a turn. Enabled, valid, first-reported unique skills retain their provider path and become typed Skill commands. A whole text-only `/name args` invokes that cached native skill block plus `$name args`; ordinary, unknown, inline and attached prompts retain the existing mapper. Both prompt and steer use the same conversion. The reference additionally lists custom prompt files, scans disk when skills/list fails, and implements native compact/goal operations. Those native-driver capabilities are not implemented here: no fabricated entries or arbitrary home reads stand in for them. A failed/malformed skills/list leaves the catalog unreported, so bounded draft discovery returns a typed discovery timeout rather than a false empty success. ClaudeStream/OpenCodeHttp catalog discovery still refuses before spawn because those drivers are unavailable on this base.

The Codex schema generator was run with an empty environment except PATH and isolated scratch HOME/CODEX_HOME. Installed Codex 0.169.0 added only optional fields/variants to five previously pinned definitions; the driver ignores those additions. The subset now also records SkillsListParams, SkillsListResponse, SkillsListEntry and SkillMetadata. No real session, network or model request is used by this contract step.

Native command preparation is a separate dormant value module shared with the receipt and transport owners. `AgentCommand.native_command` optionally carries a closed kind and is masked together with command kind while COMPOSER_COMMANDS is unassigned. It is not authority from persisted history or the UI catalog: actor admission queries the actual current run and validates the same original-text parser before reservation. The explicit `root_name` accessor avoids a collision with the generated TS trait. Goal availability additionally requires the transport owner’s launch flag and autonomous root-turn proof. No provider currently advertises these values in this package.

Catalog cache identity additionally retains the actual immutable NodeProfiles Arc. The catalog resolves from that set and verifies it before probe dispatch and after completion, including cache hits. Context and provider version checks precede completed-while-waiting coalescing. Profile reloads return Unavailable rather than publishing the retired result. Independently mutable model routes and fleet-secret-backed added definitions currently return Unsupported for draft command discovery until a checked launch-context proof exists; their names/version strings are insufficient authority. This restriction affects discovery, not normal session creation/options. No secret value is hashed into catalog identity.
