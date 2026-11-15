# Typed feedback on the published action menu

[Decision Log]
- Purpose and intent: restore the previously reviewed schedule/Git menu
  extensions without replacing the published menu or its active callers.
- Existing implementation and constraints: the published point-opened menu
  and right-aligned entry share one renderer with surface/highlight motion.
  The earlier extended renderer lacked that integration.
- Alternatives considered: keep two renderers, remove the published motion,
  or extend the published item/state/draw modules.
- Choice: extend the published renderer; keep both entry signatures, QUICK
  arrival and fill_in. A prior, separate commit only split its modules.
- Why: existing callers keep their placement and motion while new callers
  can express leading content, feedback, exact bounds and dismissal as types.
- Costs and effects: item values gain defaulted fields; two Git literals need
  constructor completion. No new dependency or separate renderer is introduced.

## Reference and preserved behaviour

Paseo 0.15.4 `packages/app/src/components/ui/menu/menu-item.tsx`:
196–220 pending/success replacement, 277–279 nonselectable feedback,
323–359 text/leading and one-line label, 480–509 hover/pressed/disabled/muted
paint, 530–533 shrinking content. Schedule caller:
`components/schedules/schedule-row.tsx` 269–378 (leading14, pending words,
end alignment and width); Git caller: `git/action-icons.tsx` (leading16)
and `actions-split-button.tsx` (explicitly kept-open direct writes).

The published `menu_right` retains its TopRight point-opened policy. Measured
`open_end` is TopRight from the actual visual bounds for either entry, not
the point clicked inside its larger hit area. Escape/outside still dismiss.
Default Close dismisses before invoking the caller; KeepOpen retains the
same surface for caller-owned pending/result state. Disabled/pending/success
cannot be activated by pointer or a stale keyboard highlight. Normal/Muted
are independent of feedback; muted idle remains selectable. Success uses
the existing CircleCheck and the previously reviewed GREEN_500 token.

The row still uses the published `fill_in`; feedback does not prepaint a
selected fill beneath that animation. The ordinary single-line glyph/label
extensions and explicit transparent row border follow the reference's item
box. The Script/Kit file budgets and token checks are not enlarged.

The restored Schedule/GitHeader trigger values are unchanged. Their hit sizes
are computed from the named visual/glyph/padding/slop inputs rather than
independent hit literals. These are the earlier two profiles, not a newly
implemented ProviderRow, modal menu, passive spacer or button-content control.

## Caller seam

`app/src/areas/git/menu.rs`: the two complete MenuItem literals in `items`
and `pull_item` retain their existing tone, disabled and starts_group values;
`..MenuItem::new(label)` supplies only the new fields' legacy defaults.
No Git action, label, authority or pending policy is changed here. Other
existing constructors and update literals need no modification.

## Verification and resume point

The original exact paint/geometry, stale-selection, dismissal and own-SVG-ink
tests return with the extensions. Added placement coverage compares both
legacy corners and measured-end geometry through the shared calculation.
The first narrow Kit all-targets Clippy check reached this source and failed
only on the callback comment's unquoted `KeepOpen` identifier (`doc_markdown`);
the comment is corrected without a lint allowance and the failed log retained.
The corrected source still needs its own check. Whole native tests and fresh
runtime/first-use captures remain
owed; an old gate or the earlier partial24-shot probe is not evidence for
this integration. The previously observed after-close keyboard-focus issue
remains unverified in production and is not silently claimed fixed here.

The corrected `ef91b954` source passed remote all-targets native Clippy in
`schedules-narrow-20261115T022908Z-ef91b954`; font setup and Clippy both
returned zero with no skipped step. View, Kit, Link and App were actually
checked. The separate whole run `schedules-20261115T024307Z-ef91b954` is
queued, not a pass. Fresh control captures are being inspected separately;
no runtime or UI approval is claimed here.

Next: finish the designated-machine whole gate and runtime checks, send the
immutable control slice to review, then consume it in the schedules
list. The list's new labels use Fluent; accepted cadence/error words remain
in their existing table for the separate translation migration.
