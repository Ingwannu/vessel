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

## Whole-gate result and new runtime evidence

The exact `ef91b954` whole run `schedules-20261115T024307Z-ef91b954`
completed with all six native steps zero, no skip: fonts, fmt, quality,
all-targets Clippy, app build and tests. The ten raw test summaries total
1075 ordinary tests and two documentation tests passed, zero failed, one
inherited ignored. Kit and App were actually checked and compiled; the
unchanged View/Link outputs had already been checked in the preceding narrow
run. No whole native gate was run on the saturated development machine.

Fresh evidence `shots/resumed-controls-v2-ef91` uses the external, uncommitted
example `resume-controls-probe-v2.rs`, not an installable fixture route.
The build record pins its patch, binary, original product source and ignored
fonts/configuration. The previous pilot with an obsolete caption is excluded.
The first24 menu captures and additional28 Ghost/KeepOpen captures were all
personally opened: both themes, normal/pending/success, long one-line EN/KO
labels, selectable muted items, pointer and keyboard activation, and explicit
Escape dismissal. These are shared-control probes, not the Automation list
or Git/provider first-use evidence. Their full per-image manifests state the
actual dimensions and exact language/state coverage.

An unresolved routing observation remains: after pointer/Enter/Escape closes
the menu, the probe root no longer receives its F5 witness. The native window
retains the probe PID as focus, and reopening the menu restores root delivery.
This does not establish production impact or a cause, and keyboard focus
restoration is not claimed to pass. No area workaround was added.

Actual shaped `Schedules예약` glyphs requested at300 match the name-only
Light proof families byte-for-byte (bounds, dimensions and raster mask), and
differ from Regular. The proof families change no outline, metric or weight
table and are not shipped. This is Linux/these11 glyphs at14 logical pixels
and scale1 only. V1 has received the method and artifacts; independent review
and UI approval remain pending. Remaining interaction/motion evidence and
first-use screens are reported separately, without rerunning unchanged Rust.
