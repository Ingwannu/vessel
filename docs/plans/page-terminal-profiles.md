# Page terminal profiles (ADR 0117, P5w)

Paseo's terminal profiles on the page:
- the "Terminal profiles" group in New tab;
- the host's Settings › Terminals list.

The node keeps the list and launches by id (ADR 0117, P5a). The page reads the list, saves it whole, and names a profile in `terminal.create`.

## What the page draws

- **New tab** (`features/workspace-tabs/profile-rows.tsx`):
  - In the "+" menu (wide and compact), after the built-ins and a separator: a "Terminal profiles" group, in the list's order. Each row shows the profile's mark and name. The group heading ends with an "Edit profiles" pencil that opens Settings › Terminals for the workspace's machine.
  - On an empty pane: the same group on a line of its own under the built-ins.
  - Rows are disabled exactly when Terminal is.
  - Paseo lines: `workspace-tabs/launcher/index.tsx` 222-252, `workspace-new-tab-menu.tsx` 37-130, `panels/new-tab-panel.tsx` 74-110, 247-262.
- **Marks** (`data/terminal-profiles.ts` `profileMark`): `icon ?? guess(command base name)`.
  - Claude, Codex and OpenCode draw their vendor marks (`ProviderMark`).
  - A known name without artwork here (Pi, Cursor, …) draws the terminal glyph, as does a profile with no name.
  - A set name outside the catalogue draws the generic agent mark (new kit icon `bot`, Paseo's Bot).
- **Settings › Terminals** (`features/settings/terminals-section.tsx`), Paseo's `host-page.tsx` 1311-1665:
  - one row per profile with its mark, name, and command line;
  - Move up and Move down (disabled at the ends), Edit, and Remove (asks first);
  - Add beside the machine picker.
  - Every change saves the whole list. A refused move or removal shows "Unable to save" with the node's reason.
  - States: disconnected ("Connect to this host to manage terminal profiles"), an empty list (Paseo's empty state), and a node too old (a warning; Paseo shows nothing).
- **Edit dialog** (`features/settings/profile-dialogs.tsx`), Paseo's `terminal-profile-edit-modal.tsx`:
  - fields Name, Command and Arguments;
  - errors after Save, cleared as the person types;
  - Enter moves to the next field, and saves from Arguments;
  - the Save button reads "Saving...";
  - a refusal is shown under the fields.

## Kit changes

- `MenuItem.leading`: a mark that is not a kit icon.
- `MenuGroup.accessory`: a control at the heading's end. `MenuGroup` moved to `menu-group.tsx` to keep `menu.tsx` within the 300-line budget.
- Icons `arrow-down` (Move down) and `bot` (the generic agent mark).

## Stated differences

- The kit's medium dialog is 520 wide (Paseo's modal is 480).
- Settings › Terminals is in English, as the page's other Settings sections are. The tab words follow the browser language (Paseo's Korean strings).
- Paseo's "Terminal agents" card (ADR 0089's hooks switch) is not drawn; it belongs to P6.
- The list is read when a workspace opens and again whenever the window regains focus. Paseo's config query refetches on focus too.

## Tests

- `data/terminal-profiles.test.ts`: guesses and base names, case for case with the Rust tests; marks; the shipped list; argument splitting; Paseo-shaped ids; the protocol gate.
- `e2e/terminal-profiles.spec.ts`:
  - New tab lists the shipped four under "Terminal profiles", with Edit profiles. "Claude Code" runs the fleet's `claude` (fake-tui) in a terminal named after it, with no prompt argument (`args []`).
  - Settings › Terminals:
    - the required-field errors and Enter between fields;
    - adding with whitespace-split arguments;
    - moving, with the first row's Move up disabled;
    - editing;
    - the list kept across a reload;
    - removing with Paseo's confirmation.
