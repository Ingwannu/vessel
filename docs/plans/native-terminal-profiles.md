# Native terminal profiles (ADR 0117, P5n)

The native app's side of ADR 0117: a machine's profiles in New tab and in Settings › Terminals. Reference: pinned
Paseo 0.15.4: `screens/settings/host-page.tsx` 1311-1665 (`TerminalProfilesSection`),
`screens/settings/terminal-profile-edit-modal.tsx`, `panels/new-tab-panel.tsx`,
`screens/workspace/workspace-new-tab-menu.tsx`, `screens/workspace/workspace-header-menu.tsx` 177-300.

## Decision Log

- Purpose and intent: launch a machine's profiles from New tab, and list, add, edit, reorder and remove them in
  Settings › Terminals, as Paseo does with its daemon config's `terminalProfiles`.
- Existing implementation and constraints:
  - New tab drew the shipped four, disabled, with a disabled pencil.
  - Settings › Terminals was listed in the view but not drawn.
  - The kit menu had no group label row and no brand mark in its leading slot, so the "+" menu showed no profiles.
  - The node keeps the list in its settings and resolves a launch by id (protocol 45).
- Alternatives considered:
  - each surface asking the node on its own (two copies that disagree after a save);
  - reading on every frame;
  - the shipped four until the node answers (rows the node may not run).
- Chosen approach:
  - Link: `Command::TerminalProfiles` reads (`node.settings`) or keeps the whole list (`node.set_settings`). Its answer
    is the list in force: the kept list, else the shipped four. A node below 45 is not asked; it is `TooOld`.
    `terminals::Request::Create` carries the profile id.
  - View (`view::terminal_profiles`, tested): one `Listing` a machine, numbered by tickets, so only the latest answer
    counts. A refused save keeps the list as it was. The rules for move, remove, a draft (name and command required
    after trimming, arguments split on whitespace, the profile bounds), an edit keeping id and icon, and Paseo's id
    shape.
  - App (`areas::terminal_profiles`): one store for every machine.
    - It reads when a workspace on the machine is shown (the terminal tabs' poll), when Settings › Terminals is shown
      for it, and when the window comes back to the front (Paseo refetches its config on focus).
    - New tab's panel and its two menus list the read list. The page and its dialogs change it.
  - Kit: two small menu additions. `MenuItem::heading` is a group label: 12 muted, never highlighted or chosen.
    `MenuItem::with_mark` puts a muted agent artwork in the leading slot at the item's leading size. The button kit
    adds `ButtonIcon::destructive`, for the ghost trash.
- Why this approach over the others: one list per machine keeps New tab and Settings the same right after a save, and
  a launch never offers a row the node would refuse.
- Pros, cons and impact:
  - A change made on another device shows when the window comes back to the front or the page opens. Nothing pushes
    it.
  - The menus and the panel show no profile group until the node answers.

## Paseo, and what Vessel draws

- New tab panel: the "Terminal profiles" group after the built-ins, each row led by the profile's mark. A row with
  no mark uses the terminal glyph; an icon outside the catalogue uses the bot. A row starts its program in a new
  terminal in place of the New tab tab. The pencil sits at half opacity, takes `surface2` under the pointer, and
  opens Settings › Terminals for the workspace's machine.
- The "+" menu (wide): a separator, the label, the profiles with 14 marks, a separator, and "Edit profiles".
- The workspace "…" menu, on a compact window only (Paseo's desktop menu has none): the same group with 16 marks,
  after the workspace actions.
- Settings › Terminals:
  - "Terminal profiles" with a plus at the heading's end, and a card of rows. Each row has its mark (16, muted), its
    name, and its command and arguments as a hint. Then Move up and Move down (off at the ends), Edit, and Remove
    (trash in the destructive colour), all ghost icon buttons named for assistive technology.
  - An empty list says "No profiles yet…". A machine not connected shows Paseo's one card saying to connect.
- Edit dialog (480 wide, a sheet on a phone):
  - Name, Command and Arguments, each labelled; Arguments has a hint.
  - After Save, an empty Name or Command says so under its field, and typing in that field clears it.
  - Enter moves to the next field and saves from the last.
  - While the save waits, the fields are read-only, Cancel and the dismiss are held, and Save reads "Saving...".
  - The node's answer closes the dialog, or its refusal shows under the fields.
- Remove asks 'Remove "{name}"?' with a destructive Remove.

## Stated differences

- While a save waits, every row button and the plus wait too, so two saves never race. Paseo leaves them on.
- While the list is read, the card is not drawn. Paseo shows the empty sentence until its config arrives.
- A node too old for profiles says so. Paseo shows nothing for a missing feature.
- A refused move or removal is said over the list. Paseo uses a system alert.
- Refusals are said in the app's words for their code, never the node's text (the Devices page's rule). A draft
  that breaks a profile bound is said in the dialog before anything is sent.
- A failed profile launch says the terminal toast "Failed to create terminal". Paseo shows the daemon's message (for
  example "claude: command not found").
- The compact menu lists no profiles until the node answers. Paseo lists its defaults before its config arrives.
- Not here: Paseo's "Terminal agents" card above the list (ADR 0089, P6).

## Tests

- view `tests::terminal_profiles`: only the latest answer counts; a refused save keeps the list; moves stop at the
  ends; remove; required fields; argument splitting; an edit keeping id and icon; the id shape.
- app `terminal_profiles::tests`:
  - the page reads its machine once and lists the rows;
  - a move sends the whole list and the buttons wait for it;
  - Add says what is missing, then saves a new last row from Enter, and closes on the answer;
  - a refused save stays in the dialog with the draft;
  - a refused removal is said over the list, which stays;
  - a machine not connected is not asked.
- app `layout::new_tab::tests`:
  - the profile rows are ready (the arrows reach them);
  - a press starts the profile in place of the New tab tab;
  - the pencil opens Settings › Terminals for the workspace's machine;
  - no group before the list is read.
- app `layout::header::tests`: the "+" menu's group, label, marks and Edit profiles, and the kit's heading row.
