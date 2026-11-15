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
  the app's numeric usage input and changing-plan fixture. Neither named
  usage modes nor plan behaviour is removed.

## Validation

Per-commit checks target affected crates. Final root and native gates run
on the identical committed head. Remote gate logs, run identifiers and
measured totals are recorded in the handover, outside repository content.

## Resume point

The node merge is committed. Client/settings and terminal approval
adaptations pass focused link/view clippy and library tests. Next: integrate
the upstream declared-port constructor/view contract when its accepted
commit is selected, then run both complete gates on the final head.

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
