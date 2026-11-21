# Sidebar hover counts and control accessibility

## Decision log

- Purpose and intent — report the same selected or expanded state visually
  and through assistive technology. Show the workspace diff in the session
  hover card even when the sidebar is configured to show a timestamp.
- Existing implementation and constraints — the controls already have element
  identities and draw from state their callers own. The sidebar already
  associates sessions with served workspace stats under ADR 0120.
- Alternatives — label each caller individually, create new controls, or
  query Git for the hovered session. A second implementation would duplicate
  state and could disagree with the row while the pointer is on it.
- Choice — extend the existing GPUI controls and reuse the sidebar's
  workspace lookup and diff-count renderer. Each overflow menu caller
  supplies its own expanded state because it owns which menu is open.
- Impact — these changes require no new dependencies or data requests.
  Older machines still have no hover diff row when they provide no count.

## Reference behavior

Paseo v0.15.4 uses a named button for the select and machine triggers
(`packages/app/src/components/ui/select-field.tsx:298`,
`packages/app/src/components/hosts/host-filter.tsx:82`). Segment buttons
report selection (`components/ui/segmented-control.tsx:135–142`), and menu
triggers report expansion (`components/ui/menu/menu-root.tsx:105–112`).

The hover card places a muted diff icon and the workspace counts below the
pull request and above the host (`components/workspace-hover-card.tsx:97–107`).
The signs and colors match the sidebar and composer
(`components/diff-stat.tsx:11–16,28–36`).

The native gate exercises the shared accessibility controls and the app.
Capture the hover row with a dirty test workspace at a pointer-sized
viewport in both themes. The touch layout has no pointer hover card.
