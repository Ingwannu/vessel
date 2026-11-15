# Shared push and pull glyphs

[Decision Log]
- Purpose: Git push/pull use transfer marks, not a plain arrow or updater icon.
- Constraint: existing ArrowUp has no tray; Update is the updater's 12-grid
  mark. Neither reproduces Paseo's Git action semantics and line family.
- Alternatives: repurpose either old mark or keep local drawings per caller.
- Choice: original Upload and Download on the kit's 24-unit, 1.5-stroke
  rounded line grid; old paths and callers remain unchanged.
- Why: Paseo git/action-icons.tsx GIT_ACTION_ICONS uses Upload for push and
  Download for pull at 16; its menu leading slot is 14. No vector is copied.
- Cost: two of this glyph-only commit's four inventory additions (65 to 69).
  Header glyphs/provenance are in backend-m12-g2-header-glyphs.md.

Tests load both paths, raster at 14 and 16, verify painted pixels and clear
outer edges. Native comparison and Git first-use captures remain required;
this note does not certify the consuming Git screen.
