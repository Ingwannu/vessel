# Milestone 12, track E: settings and machines in the native app

Branch `backend/m12-app-settings`, off `backend/m12-spike`. The app's
Settings: its frame, General, Appearance and About (with the updater's
"Check for updates" row), the preference model, and Machines, Devices,
Sync, Agents and Models; then what the page never drew (adding and
removing an agent, the registry's catalogue, installing one).

## State

| Slice | State |
| --- | --- |
| 1. The preference model and the view logic of settings and machines, with the page's cases | done |
| 2. Preferences kept by `link` (one interface, the lead's `app-prefs.json`) | done |
| 3. The settings frame (Paseo's layout) with a section registry; General, Appearance, About | next |
| 4. Machines, Devices, Sync, Agents, Models; their node calls in `link` | |
| 5. Agents beyond the page: add, remove, catalogue, install | |

## Slice 1: the view logic

In `spikes/native/view/src`, without GPUI, ported from the page's
`web/src/data`:

| Page | App |
| --- | --- |
| `prefs.ts`, `prefs-change.ts`, `theme.ts`, the bridge's `wire/prefs*.rs` | `prefs/{mod,appearance,keys,change}.rs` |
| `compose-keys.ts` | `prefs/send.rs` |
| `settings.ts`, `settings-place.ts` | `settings.rs` |
| `machines.ts`, `nodes.ts`, `book.ts` | `machines.rs` |
| `devices.ts` | `devices.rs` (and `LinkShown`, the 30 s showing) |
| `sync.ts` | `sync.rs` |
| `agent-status.ts` | `agent_status.rs` |
| `model-rows.ts` | `models.rs` |
| the page's "3m ago" | `ago.rs` |
| words | `text/settings.rs`, `text/machines.rs` (English and Korean) |

Tests: `view/src/tests/{prefs,settings,machines,devices,sync,agent_status,models}.rs`,
each naming the page's test it follows, the same inputs and outputs
(the page's JSON fixtures are used as they are where the types read
JSON). `prefs.rs` also carries the bridge's own tests of the same
document: the default document byte for byte, the tagged changes and
the refused ones, the bounds (the 65th action and the 33rd machine
refused; a change that does not grow always applies), and the key
binding grammar's good and bad lists.

Run: `cargo test -p vessel-app-view`: 145 passed.

### Decisions

- **One document, the bridge's.** `Preferences` is the bridge's
  `wire::prefs::Preferences` field for field (the same JSON, the same
  defaults, the same bounds), so a node-kept store later (ADR 0036) can
  take the app's document unchanged. Each field is read leniently: a
  value out of range or one a newer Vessel wrote is that field's default,
  and a map (shortcuts, a machine's defaults) is read entry by entry.
- **Kept in the lead's `app-prefs.json`**, as one more field (`settings`),
  not a second file. `link` will apply a `PreferenceChange` with
  `Preferences::changed` (the bridge's rule), save, and answer with the
  values and where they are kept (`Kept`): one interface, so where they
  are kept can change.
- **What a node-kept store needs** (not in this pass): a revision the
  node gives each document, a push of every change to every client of
  the person, a change sent while the answer is outstanding re-applied
  on the newer document (the page's `prefs-change.ts` does this), and the
  defaults keyed by the node's id (they are already).
- **The agent's key is the wire form** (`open-code`), as the page's
  `agentKey`; the CLI's name (`opencode`) is for people typing.

### Differences from the page, and why

- **No "here" machine.** The page's bridge runs on a node, which is
  never renamed or forgotten; the app reaches every machine through its
  book, so rename and forget are offered on all (`machine_actions`).
- **Not ported: the page's revisions and `keepSections`.** The app's
  document has no revision (one writer: the app itself), and Rust
  compares the sections by value, so nothing needs to keep an object's
  identity for React.
- **Words.** "Vessel" where the page says "vessel ui" or "this page":
  "Kept until Vessel quits: …".
- **Unknown fields are dropped on save.** The bridge keeps the fields a
  newer Vessel wrote; the app's file is the app's own, and a newer app
  reads its own defaults for them.

## Slice 2: where the settings are kept

`link/src/prefs.rs` (the lead's file) holds them as one more field,
`settings`, of `app-prefs.json`, beside the window's own (`machine`,
the sidebar's width, the update's state). Where they were read from is
`kept` (not written: it is the answer to "where are my settings").

- **One path for a change.** `Prefs::prefer(&PreferenceChange)` applies
  it with `Preferences::changed` (the bridge's rule: a refused change
  leaves the document as it was and says why); the app then saves with
  the `SavePrefs` command it already has. Moving the settings onto a
  node (ADR 0036) replaces this one function and `load`: the screens
  only send changes and read `settings` and `kept`.
- **A file that cannot be read is not written over.** The lead's file
  said a broken file is replaced on the next save; with a person's
  settings in it, that would lose them for a typo. Now: no file, or a
  file read, is `Kept::File` (saved as before); a file that cannot be
  read or parsed is `Kept::ThisLaunch { Unreadable }`: the defaults are
  used, General says so with the reason, and `save` writes nothing
  until the file is mended or moved. `Kept::default()` is the same
  (no place), so a value made in a test never writes a file.

Tests (`link`): `settings_are_kept_in_the_same_file_and_read_back`
(a change applied, saved, read back equal; a file of the lead's fields
only reads the defaults), `a_file_that_cannot_be_read_is_not_written_over`.

Gate: `cargo fmt --all --check` clean; `cargo clippy --all-targets --
-D warnings` clean; `cargo test`: view 145, app 69, link 10 passed.
