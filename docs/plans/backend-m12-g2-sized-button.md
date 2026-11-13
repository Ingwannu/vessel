# One pointer/touch button renderer

## Decision Log

- Purpose: match Paseo's default MD button in schedules and settings without area overrides.
- Constraint: existing button and button_with_kind callers retain their signatures and pointer geometry.
- Alternatives: paint/pad each consumer, or silently resize all legacy callers.
- Choice: one ControlSizing type and button_with_kind_at entry; legacy entries delegate with Pointer. TriggerSizing remains an alias to this same type.
- Why: button.tsx delegates geometry to createControlGeometry: pointer minimum32/padding16 and touch minimum44/padding24. The app selects sizing at its compact-layout edge.
- Impact: shared touch target44 has one definition; popup's public name remains available. Each caller migrates explicitly; colours, phases and labels are unchanged.

Tests inspect the actual returned button's min-height and horizontal padding, both sizing cases, and alias compatibility. Runtime approval needs actual Ghost phases in both themes and widths. No area handler or navigation is owned here.
