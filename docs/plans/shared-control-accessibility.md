# Control roles, names and the pending button (TRACKER 781, 782)

Kit leaf on package base 14.20, merged forward to the current base (visual lane,
2026-11-16).

## TRACKER 781: the pending button

Finding: the Kit already had Paseo's loading state. `button_control` with
`ButtonConfig::loading()` (kit `widgets/button/control.rs`, since 2026-11-15)
draws the source Small20 activity indicator in place of the leading glyph,
keeps the words, fades to 0.5 and refuses the handler, as Paseo's
`Button loading` (`components/ui/button.tsx` 44-51, 188, 218, 238). The label
manager's Save used `button_with_kind`, which has no loading entry, and the
manager's notes wrongly said the Kit had none. No Kit change is needed; the
consumer moves Save to `button_control` (label manager, a separate commit on
its own branch).

Known limit: Paseo also sets `accessibilityState { disabled, busy }`. The
patched GPUI exposes no disabled or busy setter (`StatefulInteractiveElement`
offers role, label, description, selected, expanded, toggled, values), so a
pending or disabled kit button announces its role and name only; without a
click listener it offers no Click action. Adding the states is a gpui-pre
patch and needs the lead's patch slot.

## TRACKER 782: roles and names

[Decision Log]
- Purpose: Paseo names every pressable for assistive technology: `Button` is
  always `accessibilityRole="button"`, the sheet's close button is "Close"
  (`common.actions.close`) and its back arrow is its own label or "Back"
  (`adaptive-modal-sheet.tsx` 411-434). Vessel's kit buttons had no role
  unless a caller named an icon-only one, and the dialog's close button and
  back arrow were unnamed shapes.
- Constraint: the kit has no words of its own. `DialogContent` is built by
  struct literal in every dialog, so a required name field would change every
  caller.
- Alternatives: a name on each dialog (every caller changes, a forgotten one
  stays unnamed); English fixed in the kit (wrong in Korean).
- Choice:
  - Every kit button drawn by the shared renderer (`button_with_kind`,
    `button_with_content`, `button_control`, `outline_button`,
    `outline_button_with_glyph`) is `Role::Button`, named by its words. An
    icon-only one is named by its caller (`Button::aria_label`, or
    `aria_label` on the returned element).
  - `ControlNames { close, back }` is a `Global` the app sets with
    `set_control_names` at startup and at each language change; the dialog's
    close button and the header back arrow read it when drawn (English until
    set). `HeaderBack::with_label` gives an owner's own name, as Paseo's
    `back.accessibilityLabel ?? back.label`.
- Impact: the app owns one call per language change. Not covered here: the
  toast's dismiss cross (Vessel's own; Paseo's toast has none) and the
  label manager's pencil ("Edit {name}") and swatches (radio, the colour
  name, checked), which are the consumer's.

Tests: `dialog::accessibility_tests` (close and back are Button nodes named
"Close"/"Back", then in the app's words after `set_control_names`, then the
owner's label) and `widgets::button::accessibility_tests` (a label button, a
pending `button_control`, an outline button and a named icon-only button are
Button nodes with their names; an unnamed icon-only one is a Button with no
name). Both read GPUI's real accessibility snapshot after activating the
adapter (gpui-pre change 11).
