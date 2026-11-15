# Native protocol base integration

The native workspace stays independent of the root Cargo workspace. Its
`link` consumes `vessel-client` and `vessel-proto`; `view` owns approval
rules and words; `app` draws those decisions with `kit`. No dependency edge
or protocol number is introduced by this integration.

## Decision log

- Purpose and intent: carry the native client onto the node's current typed
  contract so subsequent feature packages can call the same RPC methods.
- Existing implementation and constraints: the app and node developed on
  separate branches. Approval digests must never cover data the card omits.
- Alternatives considered: duplicate new wire types locally; cherry-pick
  selected node features; merge the complete node history.
- Choice: modify the existing native consumers after a two-parent merge.
  Node root definitions and generated page types remain authoritative;
  native screens retain their app parent until explicitly adapted.
- Why: the existing typed client and exhaustive matches already identify
  every consumer that needs adaptation. A second protocol implementation
  would drift. No external library or API is needed.
- Advantages, costs and effects: both histories and fake harness behaviours
  remain available; the native lockfile may resolve newly used dependencies.
  Unrendered approval fields require an explicit refusal before approval.

This is contract integration, not a new Paseo behaviour. Existing references
remain in ADRs 0035, 0036 and 0066 and the workspace plan. No real agent,
preview node or customer service is needed to validate the integration.

## Merge resolution

- Root `AGENTS.md`, quality rules, size allowances and boundary checker use
  the node parent, including its replacement of older app-root entries.
- `ARCHITECTURE.md` uses the node crate table and retains non-table prose.
- Conversation and settings plans retain both sides' conflicting text.
- Native files retain the app parent; adaptations follow separately.
- The fake ACP retains the node's named usage modes and extends them with
  the app's numeric usage input and changing-plan fixture. The numeric
  decoder retains the app parent's prefix and trimming rules exactly,
  while named node usage modes keep their existing replies. Neither named
  usage modes nor plan behaviour is removed.

## Validation

Per-commit checks target affected crates. Final root and native gates run
on the identical committed head. Remote gate logs, run identifiers and
measured totals are recorded in the handover, outside repository content.

## Resume point

The package base now includes the node's Quality B method table. The
native consumer audit found no direct protocol `since()` use or independent
RPC method list to adapt. The existing typed client owns admission and
unary method selection. Validate the combined root and native workspaces
on the same committed head; keep gate outcomes in the external handover.

### Client error and settings adaptation

New `UnassignedProtocol` outcomes follow each consumer's existing local
unsupported-call handling. The idle-release settings write leaves other
settings absent via `SetNodeSettings::default()`; fixtures explicitly keep
all drivers on ACP. No new call is registered or sent by these changes.

### Workspace approval adaptation

`Listed` exhaustively names terminal commands from the root contract.
`ApprovalRefusal::Terminals` prevents the card from attaching an Approve
button for a digest containing commands it cannot display; Cancel and the
existing skip action remain available. English and Korean explain the
refusal. Regression coverage checks the typed decision and both languages.
The existing app-parent search tests remain intact.

Reference checked: Paseo `packages/protocol/src/paseo-config-schema.ts`
lines 38–56 admits script port and worktree terminals; Vessel's deliberate
approval floor is ADR 0066. This integration retains that existing refusal
rather than extending command execution.

### Declared service ports

The reviewed workspace stack through its node integration is merged
separately because it was not yet on the selected node head. Its TOML
parser now includes declared service ports in the approval digest. The native card destructures all `ScriptEntryView`
fields without a rest pattern: name, kind, command, folder and declared
port are drawn. Adding a field breaks this display until it is handled.
Comparison uses the full borrowed view, while English and Korean name
port addition, replacement and removal with both values. Constructors in
fixtures enforce the same service/port invariant as decoded node data.
Port removal from the last approved snapshot has regression coverage.
No allocator is introduced; `PROJECT_FILE_KEYS` remains unassigned.

The existing English/Korean files are at their shrinking file budgets.
Script kind, port and change wording therefore delegates to one small
`script_words` module, reusing the existing `Lang` enum. Both languages
share the comparison, so a new approved field cannot be named in only one
language; the existing English/Korean outputs remain pinned by tests.

### Owned node settings on the current node contract

The current root settings own restart-recovery prompt text and are Clone,
not Copy. Idle release borrows cached replies instead of moving them when
painting or toggling. Test fixtures construct default recovery data without
exposing it as another app control; the idle-release write continues to
leave unrelated settings absent. Existing refresh, refusal and toggle tests
exercise repeated access to the same owned reply. No recovery feature is
assigned or enabled by this adaptation.

### Package base and Quality B

- Purpose and intent: give native feature packages the node's single method
  table, optional protocol requirements and method-shape information.
- Existing implementation and constraints: the package base has already
  split link commands, events and dispatch and the shell's event handling.
  Native calls use the typed client; UI text methods named `since` format
  relative time and are unrelated to protocol admission.
- Alternatives considered: rewrite native wrappers pre-emptively, duplicate
  admission or method lists, or retain the typed client boundary.
- Choice: merge the published Quality B node history into the package base
  and retain native source unless compilation requires a consumer change.
- Why: optional protocol admission and generated unary selection already
  belong to the root client. Native wrappers do not compare protocol
  `since()` values or maintain RPC method lists, so another adapter would
  duplicate the same contract. Existing UnassignedProtocol mappings remain.
- Advantages, costs and effects: no native behaviour, dependency edge or
  public wire number changes. Commands remain in `link/src/command.rs`,
  events in `event.rs`, dispatch in `dispatch.rs`, and shell event handling
  in `app/src/shell/events.rs`. Whole native compilation is the check that
  every request still has its generated client implementation.

The selected node commit is `29eebb8b`, the latest green head announced
when this integration began; the package parent is `8d5513e4`. Their
histories are retained by a merge commit. The node's exported table and
Option-valued admission decision are documented in `rpc.rs` and
`rpc/since.rs`; no new Paseo behaviour is introduced by this integration.

### Updated package seams

The subsequent accepted package base `6036eeb8` is retained as another
merge parent. It supplies optional node identity on the machine that
answered a book entry, extracts shared kit field events, and shares typed
retained-input editability while preserving existing field callers. These
changes stay where the package lead placed them. No protocol adapter or
second implementation is added here. The whole native gate must run again
because these accepted native sources differ from the earlier package
parent; the earlier native pass is historical evidence, not final proof.

The root gate's bootstrap-history case exposed the already documented
sidebar-before-verified-welcome race. Its owner supplies the node-head
receipt-wait test fix and slow-welcome regression. Only the coordinator-designated reviewed fix head is merged; this
integration does not edit or weaken the shared test. Both whole gates then validate the final combined commit.

### Receipt regression fix selected for final validation

The coordinator designated `c5da5305`, directly based on `29eebb8b`, rather
than waiting for the next integrator batch. Its test-only changes wait for
the verified tab-session receipt before the existing storage assertions and
add a controlled slow-welcome regression. They do not change production web
or native code. Both `6036eeb8` and this reviewed fix are ancestors of the
combined package base; later integrator work is outside this validation.
