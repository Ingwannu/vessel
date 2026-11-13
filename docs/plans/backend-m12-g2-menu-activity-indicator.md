# Menu pending explicitly uses the source ActivityIndicator

## Decision Log

- Purpose: a pending menu item must show Paseo MenuItem's numeric16 indicator.
- Constraint: the new shared ActivityIndicator is additive; a legacy spinner call cannot silently change every existing consumer.
- Alternatives: an area-owned pending spinner, or retaining the non-equivalent old arc.
- Choice: migrate only the shared menu pending renderer to `Size::Numeric(Points::SIXTEEN)` in a separate control-only commit after the additive entry.
- Why: `components/ui/menu/menu-item.tsx::resolveLeadingContent` explicitly asks LoadingSpinner for16. The label, leading slot16, disabled opacity0.5, guards, width and dismissal policy remain unchanged.
- Impact: G1 and schedules share the correction; old `slider::spinner` callers elsewhere retain their original appearance and800ms. No area paints loading/result states.

Source/profile tests and whole-native gate cover the shared dependency. Runtime pending/stale-keyboard and direct-write KeepOpen evidence is required separately; no actual Git result is inferred from the fixture probe.
