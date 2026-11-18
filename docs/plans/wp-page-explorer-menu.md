# The page Explorer's file action menu (Paseo's FileActionsContextMenuContent)

Paseo's explorer tree opens one menu per row on a right click or a long
press (`components/file-explorer-pane.tsx` 344-396, the menu
`components/file-actions-menu.tsx`), and one for the tree's empty space
(1351: New file, New folder at the root). The page's Files tab
(`features/workspace/file-tree-view.tsx`) has none: a row only opens.

## What Paseo's explorer offers, and what the page can do

Order and groups are Paseo's (create, open, reference, manage,
destructive; a hairline between groups; 220 wide; 15 icons, the
destructive rows red).

| Row                           | Paseo explorer                                                                                                                                          | Page                                                                        |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| New file, New folder          | a directory row and the empty space; an inline name row in the tree (`draft`, placeholders), Enter makes it, Escape or blur drops it; a made file opens | `file.write` (new, `expect` none) and `file.create_dir`: yes                |
| Collapse folder               | an open directory                                                                                                                                       | yes (the tree's open set)                                                   |
| Open in editor                | desktop only (a directory, the user's editor)                                                                                                           | no: a browser has no editor to open                                         |
| Open to the side              | a file, where the workspace has a second pane                                                                                                           | no: the page's Files tab has one file view                                  |
| Copy path, Copy relative path | every row; nothing said (no toast)                                                                                                                      | yes: the workspace root joined, and the relative path                       |
| Reveal in file manager        | desktop only                                                                                                                                            | no                                                                          |
| Download                      | a file                                                                                                                                                  | `file.read_bytes` pages into a Blob, saved by the browser (an `a download`) |
| Add to chat                   | a file, where a composer is open                                                                                                                        | the open session's composer gets `@path ` (its draft)                       |
| Rename                        | every row; the row becomes its name field                                                                                                               | `file.rename`: yes                                                          |
| Duplicate                     | where the node has `file.duplicate`                                                                                                                     | yes, from the file panels protocol on                                       |
| Delete                        | every row; a destructive confirm ("Delete file?" / "Delete folder?", the name)                                                                          | `file.delete` (recursive for a folder): yes                                 |

A failure says so in an error toast (Paseo's `toast.error`, the node's
words); a made, renamed or duplicated entry is selected, a made or renamed
file opens.

## Decision (ADR-light)

[Decision Log]

- Purpose: the page's Explorer acts on its files as Paseo's does, from the
  same menu in the same order.
- Existing assets and constraints: the kit `Menu` (Base UI menu, the
  popup recipe); Base UI also has `ContextMenu`, whose parts are the same
  menu parts; `data/workspace/files.ts` has write, create_dir, rename and
  delete, not duplicate or read_bytes; the tree lists folders as they open
  (`file-tree.ts`); a mutation must refresh the folders it touched.
- Alternatives: a "…" on each row (not Paseo's explorer, and a 28px row
  has no room); a dialog for names (Paseo edits in the row); Paseo's
  shape.
- Choice: a kit `ContextMenu` (right click; a touch long press opens it
  too, as Paseo's `onLongPress`) holding the kit's menu rows; one
  `explorer-menu.tsx` that builds Paseo's action list from what the row
  and the node allow; inline name rows in the tree for New and Rename;
  the kit `Dialog` for Delete's confirm; the data calls in
  `data/workspace/files.ts`.
- Costs: Open to the side, Open in editor and Reveal stay out (no second
  pane, no desktop); Download holds the file in the page's memory before
  the browser saves it (the node pages it, as the native app's).

## Packets

- E1: kit `ContextMenu`; the row and empty-space menus with Collapse
  folder, Copy path, Copy relative path, Download, Add to chat. Tests:
  unit (the action list per row: groups, order, availability), e2e (a
  right click lists Paseo's rows; Copy path copies; Download saves the
  file's bytes; Collapse).
- E2: New file, New folder, Rename (inline rows), Duplicate, Delete
  (confirm), the listing refreshed; e2e on a fleet workspace.
- Captures to the coordinator (1280 and 390, dark and light).

## Status

- E1 done (2026-11-18): the menu, Collapse, the copies, Download, Add to chat.
- E2 done (2026-11-18): New file and New folder (a row and the empty space),
  Rename in the row, Duplicate (protocol 27), Delete after the question; a new
  file is written only where none is (`expect: absent`); the open folders follow
  a rename or a delete; names Paseo refuses are refused in its words.
