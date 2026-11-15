# The published menu: module boundaries before feedback

[Decision Log]
- Purpose and intent: preserve the published menu while making room for its
  additive, separately reviewed feedback and trigger APIs.
- Existing implementation and constraints: the published file combines item
  values, caller-owned state, placement and drawing. Adding every extension in
  that file would exceed the product file budget.
- Alternatives considered: duplicate the renderer for extended callers, or
  split the existing renderer along its responsibilities first.
- Choice: keep state/callbacks in `menu.rs`, move item values to `menu/item.rs`
  and drawing to `menu/draw.rs`, re-exporting the same public names.
- Why: existing callers continue using one renderer and identical signatures;
  neither placement nor animation becomes a parallel implementation.
- Costs and effects: two private child modules; no new dependency or behaviour.

## Preserved implementation

All menu/item/state/function bodies are taken unchanged from integration
`29a9834b`. Imports and module headers are adjusted only for their new scopes.
No test expectation or application caller changes. `menu_right` still passes
TopRight to the same private renderer as `menu`; its published point positioning,
`popup::appear(QUICK)`, `fill_in` and keyboard/outside behaviour remain intact.
The original pending/success/muted/leading/KeepOpen extensions are not in this
commit. Their return is a behaviour change in a separate control commit.

Reference provenance stays the published menu's `components/ui/menu/` chain;
this is a source refactor, not a new reference-parity or visual claim.

## Verification and resume point

The external source audit compares the moved item/constants/state/draw bodies
against the immutable integration parent. Local fmt, size, boundaries and token
checks are the noncompiling checks. Compilation and the whole native gate are
owed on the designated build machine. No new capture is submitted for a
source-only split, and no runtime approval is inferred from it.

Next: reapply typed feedback, leading glyph size, dismissal and actual-bounds
placement without removing the published renderer's motion or existing entry.

The first narrow Kit Clippy run compiled the cold dependency tree and failed
on unused imports: `Window` in the split drawing module and the legacy
Animation/AnimationExt/Duration imports left after spinner delegation. The
follow-up removes those imports, with no lint allowance or behaviour change.
That first failure remains in its log; the changed tree needs a new check.
The changed tree's short-lane `cargo clippy -p vessel-app-kit --all-targets --
-D warnings` then passed, with Kit actually checked. This is compilation/lint
evidence for Kit only, not the still-owed whole native test gate or UI proof.

## Completed native gate and review

The whole native gate subsequently passed on immutable
`f6674d0e651ba2975552da01e58fc588bb4f9ef7` in run
`schedules-20261115T022318Z-f6674d0e`: fonts, fmt, quality, Clippy, app build
and tests each returned zero, with no skipped step. There were 1,060 ordinary
test passes and two doc-test passes, zero failures and one inherited ignored
test. All four native crates were actually checked/compiled. The run waited
882 seconds before its slot; execution took about three minutes, with load
18.05 at the start and 17.76 at the end.

The reviewer independently passed the seven-path pure split/import correction
scope only, including moved-body/constant identity and the gate logs. This is
neither review of the earlier published-base merge conflicts nor approval of
the later typed restoration, runtime, Automation screens or other profiles.
No unchanged Rust gate is repeated for this result note.

Resume: the separate typed restoration is `ef91b954`; its own whole gate and
fresh first-use evidence are tracked in the restoration note.
