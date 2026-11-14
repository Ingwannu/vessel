# Shared archive and git action glyphs

[Decision Log]
- Purpose: one registered glyph for each shared action, without local copies.
- Existing assets/constraint: Pencil and Archive already exist. The kit draws
  its own vectors on a 24-unit grid, stroke 1.5, round caps and joins.
- Alternatives: duplicate Pencil/Archive, import third-party SVGs, or let
  screens retain local drawings. None preserves one source for the action.
- Choice: add only Trash2, ArchiveRestore, Minus, GitBranchPlus, Commit and
  Undo2. Trash2 and ArchiveRestore reuse the kit's bin/archive geometry;
  branch-plus reuses its branch. Commit reuses the git track's supplied
  original path verbatim at `kit/commit.svg`, rather than a second drawing.
- Why: callers share the same weight, monochrome mask and asset lookup in
  both themes, and existing Glyph names/paths remain unchanged.
- Cost: six vectors and inventory entries, no dependencies or UI callers.

The inventory grows from 59 to 65. A rendering test loads every new asset
through AssetSource, checks its 24-unit view box, proves it paints at 24 and
leaves each outside edge clear. Pencil and Archive are deliberately not
re-added; the horizontal window-control mark is not renamed. Minus and
Undo2 use the shared line grid; neither is a text character.

## Reference provenance and necessary semantic additions

- Commit: Paseo `git/action-icons.tsx` supplies GitCommitHorizontal at 16,
  consumed by `git/actions-split-button.tsx`. GOAT's
  `components/GitActionsControl.tsx` also uses commit icons. The drawing
  here is the git track's original, not reference SVG data.
- Undo2: Paseo `components/file-actions-menu.tsx` uses it for `onRevert`,
  reached by `git/diff-pane.tsx::useDiscardChangesAction`. Trash2 in the
  same menu means file deletion, not discarding changes.
- Trash2: Paseo `review/surface.tsx` has a 14-point destructive-action
  instance. Using it to drop a Vessel saved stash is a semantic addition;
  no matching reference git-stash deletion component was found.
- GitBranchPlus: GOAT `components/GitActionsControl.tsx` uses it for
  **Initialize Git**. Using it for Vessel's create-branch capability is a
  semantic addition, not a reproduction of that reference action.
- ArchiveRestore for restore-stash and Minus for unstage name Vessel node
  capabilities. No matching first-use git component was found in either
  reference. Their consumers use Paseo's existing menu-item manner.

First use remains with the git/session owners. No screen, route, toolbar
trigger or menu consumer changes in this commit. The earlier shared menu
supports these leading glyphs without any further API change. No reference
code or third-party vector is copied. Native rendering tests do not stand
in for the consuming screen's runtime captures and interaction checks.
