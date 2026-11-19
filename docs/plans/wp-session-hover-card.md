# A session row's hover card, as Paseo's (TRACKER 810)

Status: plan, for the coordinator's rulings.

## Paseo

`components/workspace-hover-card.tsx` on `components/ui/hover-card.tsx`, wrapped around every
workspace row (`sidebar-workspace-row-content.tsx`):

- Only on a wide window (`isCompact` draws the row alone) and only with a pointer (web).
- It opens at once when the pointer enters the row. It stays open while the pointer crosses onto
  the card (a hover safe zone). It closes 100 ms after the pointer has left both
  (`CLOSE_GRACE_MS`). Escape closes it. Keyboard focus inside keeps it open. It stays closed while
  the row's context menu is open or the row is dragged.
- Placement `right` of the row, offset 4 (`HoverCardContent` default), kept in the window.
- The card: width 260, `surface1`, a 1 `borderAccent` edge, radius 8 (`lg`), 8 above its first
  row, shadow 0/4/8 at 20% black. Role `menu`, named "Workspace scripts".
- Rows, each 12 at the sides with 8 under it, glyphs 12, words `sm` muted on one line:
  1. the workspace's name, `base`, in the foreground;
  2. its pull request badge (`PrBadge`, a link to the pull request), when it has one;
  3. its diff stat (`FileDiff`, then +a in success and −d in danger), when known;
  4. its host (`Server` and the host's label);
  5. its branch (`GitBranch`), copyable: on hover the glyph becomes `Copy` and the words the
     foreground; a press copies the name and shows `Check` for 2 s (named "Copy branch name");
  6. its folder (`Folder`, the short label), copyable the same way, copying the whole path (named
     "Copy path");
  7. under a 1 `border` rule, when the pull request has checks: a 28-high link row (the forge's
     mark, `ExternalLink` on hover, "Checks", then a count per state in the state's colour) that
     opens the checks page.

## Vessel today

The sidebar's session rows (`areas/sidebar/session_row.rs`, TRACKER 663/813) have no card. The
`Card` holds the title, its machine, `detail.branch`, `detail.cwd` and `detail.pull_request` (the
meta line already draws the pull request and its checks from `CardPull` and `CardChecks`). It holds
no diff stat; the Changes tab reads one per workspace when opened.

The kit has the pieces: `Tooltips` (a hover surface shown at once, `Side::Right`, kept in the
window, but words only and not enterable); `popup::Surfaces` (Paseo's menu surface:
`surface1`, `borderAccent`, radius 8, the medium shadow); `popup::placement` (whole-trigger
placement); `copy` (copy and the check for a moment); `external_link`.

## Plan

- kit `hover_card`: one card per screen, like `Tooltips`.
  - The trigger records its bounds and, on hover, makes itself the card shown.
  - The screen draws the card over everything, right of the trigger at 4, kept in the window.
  - The pointer over the trigger, the card or the gap between them keeps it open. Leaving all of
    them closes it after 100 ms, on one owned timer, as the tooltip's pending timer does.
  - Escape closes it. A press on the trigger closes it, so the row's click and its menu win.
  - Paseo's card values go through `popup::Surfaces`.
- app `areas/sidebar/hover_card.rs`: the card's rows from the `Card`, in Paseo's order and values:
  - the title;
  - the pull request badge (its link opens the pull request, as the meta line's item does);
  - the machine (`Server`);
  - the branch, copyable, when it has one;
  - the folder, copyable, its label as the meta line shortens it;
  - the checks link row under a rule, when there are checks.
  It is wired from `session_row` on wide windows only, not on touch, and not while the row's
  menu is open. Its words are Paseo's: "Copy branch name", "Copy path", "Checks"; ko
  "브랜치 이름 복사", "경로 복사", "검사".
- Tests:
  - kit: open at once, kept while crossing to the card, closed 100 ms after leaving, Escape closes.
  - app: the rows a card draws for a session with and without a branch, a pull request and checks;
    copy puts the branch or path on the clipboard; no card on touch or with the menu open.
- Frames at 1280, en-dark and ko-light:
  - the card on a plain session;
  - on a session with a branch, a pull request and checks;
  - a copy row hovered and just copied;
  - the checks row hovered;
  - kept while the pointer is on the card.

## Questions for the coordinator

- Q1 The diff stat. The card has none per session. Options: (a) leave the row out, a stated
  difference, the row coming with a later item that reads workspace diff summaries; or (b) read
  the workspace's diff summary when the card opens (one git call per hover, kept a short while).
  I propose (a).
- Q2 The card's name to assistive technology. Paseo's is role `menu`, "Workspace scripts", which
  looks like a leftover. Options: keep Paseo's words with role menu, or a group named
  "Session details" (ko "세션 정보") holding its buttons and links. I propose the group.
- Q3 Keyboard. Paseo keeps the card open while keyboard focus is inside it. The native rows are not
  tab stops yet (813 named them for assistive technology only). I propose pointer only in this
  item and keyboard with the rows' focus item.

## Rulings (coordinator, 2026-11-19)

- Q1 (a): the diff stat is left out, a stated difference. In Paseo it is a server field (the
  workspace descriptor's `diffStat`), not something the card reads, and the node has no such field
  yet: TRACKER 819 (node descriptor field and live update, then the card and the page). The card's
  row order keeps its place, after the pull request badge, so 819 only adds a row.
- Q2: the card is a group named "Session details" / "세션 정보" (Paseo's role `menu` named
  "Workspace scripts" is left over: its card has no scripts), a stated difference.
- Q3: pointer only, as Paseo's (a hover card, wide window only). Keyboard is not a Paseo gap; no
  item.
- As Paseo's: no card while a drag is under way (TRACKER 811 will need it), none in the narrow
  layout.
- Frames: the list above, plus a card on a row near the window's bottom (it stays on screen).

## As built

- kit `hover_card` (`HoverCards`): one card per screen. The pointer's place is kept by trigger key
  and for the card, so a late leave from another row changes nothing. The 100 ms close only
  fires while the pointer is on neither. A press on the trigger, `close()` and Escape (a listener
  for the frame the card is drawn in) close it. Paseo's surface is used, with its own 0/4/8 20%
  shadow. kit `copy` gains `copied_within` and `copy_for`, so the card's copy rows and the copy
  button share the one "copied last".
- view `sidebar::hover` (`Card::hover_details`, `Card::checks_url`, `short_path`): the card's
  facts, the folder label as Paseo's (a worktree's folder name, else `~` for the home folder),
  and the checks page (GitHub's `/checks`, else the pull request).
- app `areas/sidebar/hover_card.rs`: the content (title, pull request link, the diff stat's slot,
  machine, branch and folder copy rows, checks link row under a rule) and `on_row`. It is wired
  from `session_row` when not touch and the row's menu is closed. `row_menu_open` closes the card.
- Differences beyond the rulings: the checks row leads with the pull request's mark (the kit has no
  forge brand mark) and says the checks' one state; the outward arrow is the kit's `ArrowUpRight`.
