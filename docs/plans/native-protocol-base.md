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

Merge resolved; checking the combined fake harness before committing the
merge. Next: adapt exhaustive client errors and workspace approval data,
then run both complete gates. The merged ScriptEntry currently has public
kind and no port; the constructor/view change requires its upstream commit.
