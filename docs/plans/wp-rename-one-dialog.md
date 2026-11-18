# One rename dialog that waits; the tab menu from the keyboard (TRACKER 800)

Visual lane, 2026-11-18. Built on TRACKER 797's branch (head 5583af105).
Reference: pinned Paseo 0.15.4 (`1c00b365c`): `components/rename-modal.tsx`
(`AdaptiveRenameModal`), `screens/workspace/use-workspace-tab-rename.tsx`.

Status: built and gated; frames with the coordinator.

## Paseo

- One modal for every rename. While its submit waits, the field is not
  editable, Cancel and the dismiss do nothing, and Rename reads "Saving...".
  Success closes it; a failure shows its message under the field (or
  "Unable to save") and keeps the typed title.
- An empty title: "Name is required". Rename is disabled while the text is
  the title already or one it would refuse. The field stops at
  `maxLength` (200).
- The tab's label follows the new title (the agent store's title).

## Vessel before

- Two dialogs: the session's (789: Paseo's frame; it sent the rename and
  closed at once, a refusal shown under the bar's title) and the workspace
  layout's (its own frame; it waited for the node's answer, matched by a
  ticket, with the original node's alias and id).
- The tab's label kept the old title after a session rename (the open
  session's title was not updated).
- The bar had a second "…" (Session actions) beside the workspace "…".

## Rulings (coordinator, 2026-11-18)

1. One rename dialog: 789's Paseo frame with the layout rename's
   send-and-wait and original-node authority, from every surface.
2. Remove the bar's session "…" (Paseo has no second "…"), after making
   sure the tab menu opens from the keyboard on the focused tab (the
   context-menu key or Shift+F10), with a test. The compact switcher's row
   menu covers the hidden tab head.
3. The resume-unavailable notices stay Info until TRACKER 802 (impl F)
   brings Paseo's toast card (variant icon, no close, hover pause, error
   3200 ms, others 2200 ms); then they become errors as in Paseo.

## What was built

[Decision Log]
- Purpose and intent: one rename, as Paseo's, that never says done before
  the node does.
- Existing implementation and constraints: the layout's route needs a
  workspace layout key; today `layout::key` always gives the session's key,
  so that route waits for workspace layouts. The session's act already has
  an answer (`ActAnswer`, kind Rename) carrying the session's new
  description.
- Alternatives considered: keep the layout's dialog and drop 789's (not
  Paseo's frame); wait on the session's next description instead of the
  act's answer (a guess at which change was ours).
- Chosen approach: `conversation::rename` keeps the one dialog with a
  route: the workspace layout's (`layout::LayoutRoute`, by ticket,
  `layout_renamed`) or the session's act (`rename_acted`, by machine,
  session and kind). One way in, `Shell::open_rename`, from the bar's
  title and the tab menu's Rename. The layout frame draws the dialog, so it
  shows whichever tab is in front.
- Why: one frame and one waiting rule for both routes; each route keeps
  its own answer matching.
- Pros, cons and impact: an answer closes the dialog without a window, so
  focus is not handed back to the opener (as the settings page's rename).
  The session's refusal is said in the dialog only, not again under the
  bar's title. A paste past the limit is refused whole (Paseo cuts it).

- Coordinator's visual return (2026-11-18), fixed: Korean "이름 변경" for the
  tab menu's Rename and the dialog's submit (Paseo's
  `workspace.tabs.menu.rename`, `renameModal.rename`; ends 789's "이름
  바꾸기"); no placeholder in the field (Paseo's tab rename passes none);
  no caret while it waits: the field is read-only and the dialog holds the
  keys (Escape reaches it and is held), and a failure hands the keys back to
  the field at the next frame. gpui-base draws a read-only field's caret
  (only a disabled one hides it, and dims its text), and its patches are
  slotted, so the dialog moves the focus instead.
- App review return (2026-11-18), fixed:
  - R1: success closed the dialog without acknowledging it, so the keys
    stayed on its undrawn focus and the next opening kept the old opener.
    Now a close without a window (an answer, another session) sets
    `rename_closed`, and the next frame calls `DialogState::dismissed`:
    the opener has the keys again, a newer focus is kept.
  - R2: the session's act goes to the session shown when it is sent. The
    dialog sends only while that is still its own session; otherwise it
    says "Unable to save" at once and does not wait.
  - P3: a send waits at most 30 s (`RENAME_DEADLINE`), then says the
    layout rename's timeout words and lets Cancel work: the act has no
    deadline of its own. An answer after it still names the open session.
- Dialog: Paseo's rename field profile (`TextField::rename_element`),
  limited to `SessionTitle::MAX_CHARS` (the kit's new
  `TextField::set_max_chars`; TRACKER 801 brings Paseo's 200). "Name is
  required" for an empty title, the node's own words for others. While
  waiting: the field read-only, Cancel and the dismiss held, Rename
  disabled and reading "Saving...".
- Success: the dialog closes and the open session takes the new title, so
  the tab's label (`layout::tab_label`) and the cards follow.
- Keyboard: Shift+F10 or the Menu key (alone) opens the focused pane's
  focused tab's menu under the tab, aligned to its start, on a wide session
  page (`layout::keyboard_menu`). They are the platform's context-menu keys,
  checked before the shortcut registry, not shortcuts of the app (not listed
  or rebound), as a browser gives Paseo's tabs.
- The bar's session "…", its menu state and its overlay are gone; the
  actions are the tab menu's (797).
- Words (en, ko): Name is required, Saving..., Unable to save (Paseo's);
  "Session actions" removed.
- Copy resume command (app review P3 of 797, ruled in): an id of only
  `[A-Za-z0-9._:-]` goes in bare, as Paseo copies it; any other in POSIX
  single quotes (`'` as `'\''`), so a pasted command never runs part of
  the id.

## Follow-up when TRACKER 801 lands

801 (impl U, node queue) adds `SessionTitle::max_chars_for(protocol)`: 200
from protocol 30, 120 before. Once it is in a package base, the field's
limit comes from it and the connected node's protocol instead of
`SessionTitle::MAX_CHARS` (`conversation::rename::start_rename`, the
`set_max_chars` call), with a test for both protocols.

## Tests

- view `tests::resume`: an ordinary id copies bare; one with a space, a
  quote and `$( )` is one quoted word; an empty one is `''`.
- kit `field::rename::tests`: the field takes no character past its limit,
  counting characters, not bytes.
- app `conversation::rename::tests`: prefilled, "Name is required", typing
  clears it, Cancel closes; a rename waits (read-only, Cancel held, Saving),
  a refusal keeps the typed title with the node's words and not under the
  bar, success closes and renames the open session; no caret while waiting
  (the field is not focused) and the field has the keys again after a
  failure; the field stops at the node's longest title; a taken rename
  hands the keys back to its opener and the next opening's Cancel returns
  to its own; nothing is sent when the session is no longer shown (Unable
  to save, not waiting); the deadline ends the wait with the timeout words.
- app `layout::rename::tests`: a workspace rename takes only its own
  ticket's answer, names the card and closes; a refusal keeps the typed
  title with the layout's words.
- app `layout::keyboard_menu::tests`: Shift+F10 opens the focused tab's
  menu with the session's actions, with nothing focused and with the
  composer focused; F10, Ctrl+Shift+F10 and the compact width do not; the
  key rule takes Shift+F10 and the Menu key alone. The Menu key is checked
  by name only: the test context types it as the word "menu", while a
  platform's Menu key (Windows `VK_APPS`, the keysym `Menu`) arrives as the
  name `menu` with no text.

## Status

Coordinator visual PASS (2026-11-18) with three fixes (0e0e23701); re-takes
on the base14.41 fleet. App review RETURN (R1, R2; P3 optional), all three
fixed. Merged with package base 14.42 (e52e8a597) at d662f1d59; gates on
the final head are in the notes and the hand-over.
