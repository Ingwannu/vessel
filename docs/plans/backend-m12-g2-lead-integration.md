# Integrating the current desktop base

## Decision Log

- Purpose: schedules' shared controls must coexist with the reviewed desktop base before their first-use screens are drawn.
- Constraint: the base adds its QR/control guide, localization exports, input-method/form work, canonical touch target, Light faces and reviewed node line. The original schedules-list work remains outside this merge as a verified patch/archive.
- Alternatives: discard either side of overlapping kit files, or keep two target constants/renderers.
- Choice: preserve the lead's non-conflicting changes and both registrations; retain the reviewed Ghost/sizing/menu/indicator controls and their tests. The glyph inventory is the unique union of both sides,71 names, with a single identical PanelLeft vector.
- Why: one renderer/API for each control remains available to existing consumers. QR, dialog scrolling, multiline field documentation and localization are retained rather than overwritten by older guide text.
- Impact: breakpoint::TOUCH_TARGET is the sole44 definition; old popup/control_sizing paths forward to it. The header's two paths use the canonical module, matching the agents track's compatibility correction; no geometry or navigation policy changes.

The lead's embedded Geist Light and Pretendard Light registrations are imported unchanged. Their assets were supplied through the existing untracked-font workflow; the source weight remains300. Actual selected-face/title captures remain required, separately from a successful build.

Root node, client, testkit and web changes come from the reviewed lead line without G2 edits. The local validation report distinguishes the native merged-tree gate from imported upstream checks; it does not claim native UI or operating-system behaviour from those source checks.
