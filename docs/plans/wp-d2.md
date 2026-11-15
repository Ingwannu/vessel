# D2: supervised scripts, service runtime and setup progress

Status: implementation; this replaces the old B1 package scope. The existing
B1 execution/card skeleton is retained, not a claim of full D2 parity.

## Scope and shared ownership

The detailed Paseo 0.15.4 survey covers scripts configuration, supervision,
service environment/port plans, local/public proxy routes, TCP health, status
projection and workspace menu, plus setup progress/explicit trust release.
Project configuration editing is C1's responsibility; this package consumes
its approved definitions. The workspace node track owns missing runtime
contracts; native code must not invent wire calls or infer health from Running.
The browser pane belongs to its own package. Until that host exists, an explicit
preview opens the platform browser, after a validated destination and socket check.

[Decision Log]
- Purpose and intent: one end-to-end surface for a workspace's supervised work.
- Existing implementation and constraints: protocol-11 script.list/run/restart,
  terminal.kill and port.list exist; native project approval and the worktree
  start/setup log already exist. The newer declared port and service_ports fields
  are separately reviewed on the node track; allocator/env/proxy/health are absent.
- Alternatives considered: run local shell commands; fabricate service URLs or
  probe health from the desktop; reuse the node supervisor and typed observations.
- Choice: extend existing view/link/workspace areas, leave node-authoritative
  features pending their reviewed contract and keep explicit unsupported states.
- Why: the client cannot know a remote process's bind interface, port allocation,
  trust state or credentials. A running terminal is not a health probe.
- Advantages, costs and effects: early native execution is useful against existing
  nodes; D2 remains incomplete until missing runtime contracts are implemented.

## Reference mapping and implementation boundaries

Paseo sources already opened for B1 are in [wp-b1.md](wp-b1.md). Additional survey
requirements and reference paths to verify directly before implementation:

| Capability | Paseo reference | Vessel reuse / remaining work |
|---|---|---|
| Configuration | protocol/src/paseo-config-schema.ts; server/utils/paseo-config-file.ts | C1 edits definitions; this package shows every approved field or refuses approval |
| Run/stop/restart | server/worktree-bootstrap.ts:518; app/screens/workspace/scripts/use-workspace-script-actions.ts | Existing node calls and native skeleton; node-side restart deliberately owns the operation |
| Service env/ports | server/workspace-service-env.ts; workspace-service-port-registry.ts; workspace-service-port-allocator.ts | Missing reviewed runtime contract; no guessed environment or allocation |
| Proxy/route aliases | server/service-proxy.ts; script-route-branch-handler.ts | Missing reviewed contract and ownership reconciliation with preview ADR; never construct fake proxy routes |
| Health/projection | server/script-health-monitor.ts; script-status-projection.ts | Missing reviewed wire contract; extend pure state with supported/pending/healthy/unhealthy when supplied |
| Menu/routes/copy | app/screens/workspace/scripts/*; utils/workspace-script-links.ts; workspace-service-routes/store.ts | Existing execution card to gain a workspace action/menu and typed route preference; current explicit/direct observed preview remains useful |
| Setup progress | app/panels/setup-panel.tsx; stores/workspace-setup-store.ts; server/worktree-session.ts:507 | Reuse native workspace/start_card, start_header, setup_log and worktree.output; preserve full logs and explicit approval |
| Helper ledger | server/managed-processes/managed-processes.ts | Provider-runtime concern, not script/terminal ownership; audit with node owner, no second client ledger |

Paseo's TCP monitor uses a three-second single-flight poll, a five-second initial
grace, a 500 ms connect budget and two failures before unhealthy; these are node
behaviour, not permission to probe remote service ports from this desktop.
Its port allocation helper is bounded by ten seconds and 1024 stdout bytes, with
at most ten dynamic uniqueness attempts; the node implementation must own them.

## Resume point

Execution/status/observed-port/card skeleton is committed. Changed-crate clippy
and tests are in progress, not yet green. Central Step 0 is still pending; do not
merge the node line independently. Next: make the current vertical slice compile,
add workspace-scoped entry point and terminal reveal using the terminal package's
existing API, audit setup observation against the current start card, and consume
only reviewed node runtime contracts. One whole native gate and one package review
follow completion. Missing env/allocator/proxy/health remain explicitly unverified.

## Existing setup audit

The existing start card already has making, exact-command approval, setup running,
failed actions and streamed combined output. The link follows `worktree.output` and
retains the created worktree on cancellation. The old code treated `SetupState::Unknown`
as successful completion; this package refuses automatic session creation for that
state and shows a complete translated explanation. A regression enumerates all known
setup states. No new wire call is involved.

The current protocol has a combined setup log and one overall state, rather than
Paseo's per-command duration/result rows and persisted blocked-source observation.
Those facts cannot be reconstructed reliably from terminal text. Workspace registry
identity is the shared registry contract; setup's passive pane will use the layout
package's `Target::Setup`, and scripts its `Target::Scripts`. Terminal reveal uses
`Target::Terminal(TerminalId)`. No competing local workspace identifier is introduced.

Missing runtime/registry contracts are dependencies, not accepted parity differences.
The tracker must not mark service allocation, env, proxy, health, route preference or
workspace status delivery passed on the strength of the existing script/port card.


Pure `scripts::services` now defines the survey's pending/healthy/unhealthy states,
per-row and aggregate tones, validated public/proxy/direct targets and preference
fallback/deduplication policy. These are view rules tested with fixed data, not a
new wire API. No current node is reported healthy and no proxy URL is fabricated.
The native adapter will accept them only from the workspace track's reviewed report.

## Package-base merge

Merged the announced package base `8d5513e4` under A8. The only textual conflict
was the native size allow-list: the head's removal of the now-split link root wins;
this package's already-reduced project/detail records remain reduced. No node-head
merge or fake harness conflict was resolved independently. The legacy workspace
registration moved into this package's actor module, so it remains outside the
split shared link root. The root crates changed only by consuming the announced base.

Setup observation/cancellation is this package's node responsibility (ADR 0093),
on the announced node head with the shared method table and pending protocol rows.
Allocator/environment/proxy/health remain the separate workspace runtime package.

## Declared-port adapter after package-base merge

The actual package base already includes private ScriptEntry kind/port and
its complete five-field view. The native execution card now projects all
five fields explicitly, uses ScriptEntry::new in fixtures and labels the
approved port as a declaration. It never turns that value into a listener,
health claim or preview authority. Existing approval rows and changes already
consume the complete view on the package base. The later optional global
service_ports block is not present on this base and remains pending its join.

The first remote narrow submission was interrupted before registration by
the shared submit-lock incident (tool-owner inventory: no run for c350918f).
It produced no compile or test result. A local narrow check covers the updated
adapter; a fresh committed snapshot will be used for the next remote request.
