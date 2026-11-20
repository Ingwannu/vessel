# Sidebar project drag order (TRACKER 811)

Paseo lets a person drag a project's header to put the sidebar's projects in their own order and
keeps that order. The native sidebar lists projects by name only (TRACKER 682 stated this as a
difference until this item).

## Paseo, as read (packages/app/src, @dnd-kit/core 6.3.1)

- What moves: the project blocks of the Project grouping (`sidebar-workspace-list.tsx:2480`,
  `DraggableList` of the unpinned projects). The whole header row is the handle (no grip,
  `:989-1018`); an open project moves with its rows, a folded one as its header. The Machine
  and Status groupings have no project headers and no project drag. Workspaces inside a project
  are dragged only under the manual workspace sort (`:1774`).
- Start: a pointer press that moves 6 (`pointer-activation.ts`, mouse `distance: 6`, no delay);
  a press that does not move stays a press (it folds or unfolds the project). Touch: a 180 ms
  hold. Not turned off in the compact layout or while filtered.
- While dragging (`draggable-list.web.tsx:135-155`, `:2672-2679`):
  - The dragged block follows the pointer on the vertical axis only (a modifier sets x to 0).
    It is drawn above the rest at opacity 0.9, scaled 1.02. Its header takes `surface2`, a 1
    `border` edge and shadow `md` (0/4/8, black 20%).
  - The other blocks move aside to open a gap where it would land (sortable,
    `closestCenter`, a 200 ms ease). There is no drop line and no placeholder drawing.
  - Near the list's top or bottom edge (the outer 20% of the scroll area) the list scrolls,
    faster the closer the pointer is to the edge (dnd-kit's default auto-scroll, acceleration
    10).
  - A workspace row's hover card is off while a drag is under way (`workspace-hover-card.tsx:67`).
- Drop (`reorder-items.ts`, `:2271-2292`): the block moves to the index it is over
  (`arrayMove`). If the visible order changed, the stored order becomes the visible order
  followed by the stored keys that are not shown (`mergeWithRemainder`), so a project hidden by
  a filter keeps its place. Escape during a pointer drag cancels it, and nothing changes. Folds
  are kept as they were.
- Kept (`stores/sidebar-order-store.ts`): one list of project keys on the device, not per
  server, kept in local storage. Every project seen is appended to the end, in the baseline
  order (`appendMissingOrderKeys`, written by an effect), so a new project shows last. Keys of
  projects that are gone are skipped when read and never pruned.
- Assistive technology: the header stays a button and the block a group named by the project.
  dnd-kit's own `role`/`tabIndex` are stripped, so there is no keyboard drag in practice; there
  are no custom announcements.

## Plan

- view `prefs/order.rs` (new): `ProjectOrder`, a list of (machine name, project id) as folds
  use.
  - Place: the stored projects in their order, then the rest by name (the tree's current order).
  - After a drop: the shown order, then the stored keys not shown.
  - Kept tidy as folds are: a whole project list drops its machine's gone projects, a renamed
    machine keeps its places, a forgotten machine's go.
  - The document holds it only once a project has been dragged (`skip_serializing_if` empty),
    as `folded_projects` and `layout.sidebar_width`, so the document the page and the bridge
    share is unchanged until then. Native only (the page has no project tree until TRACKER
    812).
- view `sidebar/tree.rs`: `TreeState` takes the order, and `sidebar_tree` places projects by it,
  with name as the fallback.
- app `areas/sidebar/project_drag.rs` (new):
  - The header row starts a GPUI drag of a `ProjectDrag` key with `drag_threshold(6)`
    (`kit::drag::LIST_DRAG_DISTANCE`).
  - At the start the shell keeps a snapshot of the shown order (Paseo's `dragItems`), so lists
    that arrive mid-drag do not reshuffle it.
  - While the pointer moves (`on_drag_move`), the index it is over is the block whose centre is
    nearest (`closestCenter`). The list is drawn in that provisional order, with the dragged
    block's place left as an empty gap of its height.
  - The dragged block itself (header and open rows, drawn by the same code) is drawn over the
    list at the pointer's height minus where it was grabbed, at the list's left, at 0.9, with
    the dragging header style. GPUI's own drag view is empty.
  - Drop writes the order; Escape (an app-wide interceptor, as the hover card's) cancels.
  - The hover card closes at the start and stays off until the drop (810's `HoverCards::close()`
    and `enabled`).
  - A press that does not move still folds the project. After a drag, the release does not
    fold it.
  - Auto-scroll at the list's edges as dnd-kit's (20% zones, speed scaled by depth).
- Tests:
  - view: placing, merging and tidying the order.
  - app, in a real window:
    - a press under 6 folds and does not drag;
    - a drag past 6 moves the block to the index under the pointer, and the order is written
      after the drop;
    - Escape restores the order;
    - the hover card is closed while dragging;
    - a11y names are unchanged.
- Frames (1280 en-dark, ko-light; 390 too, since Paseo drags in the compact layout as well):
  mid-drag (the gap, the lifted block), after the drop, and an open project mid-drag.

## Differences to state

- No scale: GPUI has no transform for a laid-out block, so the lifted block is drawn at its own
  size (Paseo scales it about 1.04, the item's 1.02 times the row's 1.02).
- The other blocks move to the gap at once, without a 200 ms slide, unless the kit's motion
  covers it (the software renderer holds motion still in any case).
- Projects never dragged are placed by name after the dragged ones. Paseo freezes each new
  project at the end in the order it first appeared, which is the same order when projects
  arrive one list at a time.
- Gone projects' places are pruned (as folds are); Paseo keeps them.
- Pointer only: no keyboard reorder (Paseo has none in practice), no announcements.
- Workspace (session) order inside a project stays as every grouping lists sessions: Paseo
  drags workspaces only under its manual workspace sort, which the native app does not have.

## Questions for the coordinator

- Q1 The lifted block. (a) As Paseo's: drawn over the list at the pointer (vertical only), with
  a gap opening where it would land. (b) Simpler: the block jumps from slot to slot in the list
  with the dragging style, and does not follow the pointer between slots. I propose (a).
- Q2 When the order is written. (a) Only on a drop that changes the order; never-dragged
  projects by name after the stored ones. (b) Paseo's: every project seen is appended to the
  stored list as soon as it is listed (the document changes without a drag). I propose (a).
- Q3 Auto-scroll at the edges during a drag: now, as dnd-kit's default, or a stated gap. I
  propose now.
- Q4 The compact layout (390, the phone sheet): Paseo drags there too (with a mouse after 6, on
  touch after a 180 ms hold). I propose the same pointer drag there; the native app has no
  touch input.
