# A session row to assistive technology (TRACKER 813)

The app reviewer's P3 on TRACKER 662/663: the session row, its kebab and its status slot had no
role or name. The coordinator's ruling (2026-11-19): the row is a button named by its title; the
kebab is a named button, and an assistive technology's Click opens its menu; the status slot has
role status and is named by Paseo's bucket label (`getStatusBucketLabel`).

## Paseo

- `sidebar-workspace-row.tsx`: the row's pressable is `accessibilityRole="button"` with
  `aria-selected` while the workspace is open; its name is its content (the title).
- `sidebar-workspace-menu.tsx`: the kebab is named `sidebar.workspace.actions.menu`
  ("Workspace actions"), also the sheet's title.
- `project-leading-visual.tsx`: a project's badge is named by `getStatusBucketLabel`.

## As built

- `kit::status_mark::status_slot` takes the bucket's label and is a status node (done included,
  so every row says what it is doing); the project badge already was one (TRACKER 662).
- `areas/sidebar/session_row.rs`: the row is a button named by its title, `aria_selected` while
  open; the kebab a button named by `row_actions` ("Session actions", the menu's sheet title),
  its click calling `Shell::row_menu_activate` (opens unless already open for the row: a pointer's
  press opened it on the way down), as the project kebab's `project_menu_activate`.
- Difference, named: "Session actions" for Paseo's "Workspace actions" (a Vessel row is a session).

## Tests

- kit `each_badge_and_each_slot_says_its_bucket_as_a_status`.
- app `session_row_tests`: rows named by their titles, kebabs named and taking a Click, slots and
  badges said as statuses; a Click alone opens a row's menu once.
- app `actions_tests` badge test counts the session slots too.
