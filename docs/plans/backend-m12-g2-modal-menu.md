# A menu inside the existing dialog

[Decision Log]
- Purpose: the saved-entry dialog can show its own action menu above the card.
- Constraint: pinned GPUI globally sorts deferred paint; nesting a priority1
  menu under a priority2 dialog does not lift it. Raising every menu would
  change the published screen contract.
- Alternatives: an area renderer, temporarily closing/reopening the dialog,
  changing dialog priority, or changing all menu defaults.
- Choice: typed Layer::Screen (default1) / AboveDialog (3), through Content's
  additive builder and the SAME panel, placement, motion and input handlers.
  Only AboveDialog has a transparent viewport input boundary, no second scrim.
- Why: outside dismissal is a single action. A menu press must not also dismiss
  its host dialog or activate a sibling action after the menu unmounts.
- Costs: the caller chooses its real containing surface; no automatic ancestor
  lookup or modal stack is invented. Existing Content constructor/defaults stay.

Reference pinned1c00b365 menu/menu-root.tsx read whole; menu/menu-overlay.tsx
395–455 handles Escape with preventDefault/stopPropagation and restoreFocusRef;
334–352 gives a backdrop its own close Pressable. GPUI's named ordering and
transparent modal input boundary implement that layering on this backend.
No new painted values, alternate menu rows or dialog renderer changes.

Public Content literals were searched across the native tree before adding
Layer: all product consumers use constructors, no literal requires a new field.
menu/menu_right keep Screen; menu_with(Content::new(...).with_layer(...)) opts in.
MenuState uses the already reviewed G4 ReturnFocus helper. Selection and Escape
stop in the same handlers; outside closes/restores the menu and consumes the
press only in AboveDialog. Pending/stale item policy stays the caller's.

The modal boundary reads Window.viewport_size in RenderOnce. Using size_full
inside the dialog would cover only its parent's body, so the explicit viewport
read is necessary; the outer anchored origin is the window origin. Screen
menus retain their original surface without that wrapper.

New both-theme real-window nested-dialog cases exercise pointer selection,
Escape, outside followed by a second outside press, original field-handle key
routing and no underlying action. A literal default/layer regression pins1/3.
Existing menu tests are unchanged. Source is committed before narrow checks;
no compile/runtime/UI/consumer proof is claimed yet. Actual focused TextField,
long/scroll/clip/overlap and G1 saved-entry first-use captures remain owed.

Exact1bb priority Kit Clippy reached the changed Kit and stopped before tests:
the new Content builder was incorrectly const despite ElementId destruction,
the literal-priority helper was private to draw instead of its parent test
module, and two imported paths were redundantly qualified. The correction
uses an ordinary builder, parent-only priority visibility and short imported
names. No geometry/input/expected value or old test changes; old run stays red.
Corrected narrow/source/actual first-use proof remains owed.

The corrected388b Clippy then found a terminal-unit semicolon in the shared
trigger callback and operator arithmetic in the new test's measured click
coordinate. Add the semicolon and express the identical coordinate through
Pixels-to-f32/mul_add (no cast or lint exception). Original expectations and
all input geometry stay unchanged;388b is red and no tests ran there.

Exact isolated6ece4a76 changed-Kit suite is now green: Clippy0 and164/0/0 tests,
fonts/cargo both0/no skip. Runs schedules-menu-20261115T105950Z-6ece4a76 and
schedules-menu-20261115T110007Z-6ece4a76. Changed Kit checking/compiling verified.
This is headless GPUI coverage, not an actual Linux/OS/consumer UI or full App/F1
pass; old native F5, focused TextField, hit/clip/neighbor and first-use evidence
remain owed. V1 source disposition is pending. Coordinator assigned S2 the
additive hierarchy writer AFTER this delta's checked source handoff; no second
engine or concurrent draw writer. The temporary label retires after handoff.
