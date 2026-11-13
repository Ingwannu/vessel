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
| 3. The settings frame (Paseo's layout) with a section registry; General, Appearance, About | done |
| 4. Machines, Devices, Sync, Agents, Models; their node calls in `link` | next |
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

## Slice 3: the settings frame, General, Appearance, About

`app/src/areas/settings/`: `mod.rs` (the state, the Shell's verbs, the
section registry `content`), `nav.rs` (the section list and the machine
picker), `page.rs` (a section's page), `general.rs`, `appearance.rs`,
`choices.rs` (each select's rows and the change it sends). `kit`
gained `select.rs` (a trigger and its popover; a sheet on a phone) and
`settings::row_at` (a row with an error line and a phone's stacking).
Words: `view/src/text/settings.rs`, `view/src/text/about.rs`.

**The frame, as Paseo's app.** `Route::Settings` replaces the lead's
`Route::About`. On a wide window the section list takes the sidebar's
place (264 wide, on the shell's ground): "Back" first, then the groups
App (General, Appearance, Shortcuts, Notifications), Agents (the
machine picker, Agents, Models), Machines (Machines, Projects, Devices,
Sync), About. The page is a column 720 wide at most, a large title,
then titled islands of rows. On a phone the list is the root page
(rows in rounded islands, a back arrow and the title), and a section is
pushed over it with its own back arrow; the sidebar's gear opens the
list, its build label opens About.

**Another track's section** comes in by one line in `content`
(`Section::Shortcuts => …`), as a route does; until then the section
says where it is coming from (`page::elsewhere`).

**General**: the send key and the default send, each a select; "Kept":
where the settings are and how long (a file, with its path and "Copy
path"; or for this launch only, with the reason).

**Appearance**: the theme as three pictures (System, Light, Dark;
Paseo's tiles), the interface and code sizes and the conversation's
width and display as selects. A change is applied by `Prefs::prefer`
and saved; the theme is applied at once (`apply_theme`).

**About**: the lead's card (the version, "App updates" with track R's
state and commands, the log file) under "Vessel", then "Software in
this app": Vessel's own licence and every crate and face the app
ships with theirs (`view/src/about.rs`, `BUNDLED`; its test reads the
workspace's `Cargo.toml` and `kit/src/fonts.rs`, so a dependency added
without a row fails the gate).

Run: the fake fleet (`scripts/fleet.sh`, machines build-server and
laptop), the app on a virtual display at 1280x800 and 390x844, dark
and light, English and Korean. Seen: the list, each section, a select
open (a popover; a sheet on a phone), the picker, a theme change
applied at once, the kept path copied (the notice says so), the phone's
gear opening the list and Back returning to the conversation.

### Differences from the page and Paseo, and why

- **Sizes and width are selects** over the model's closed ranges
  (interface 12–20, code 10–18, width comfortable/wide/full); Paseo
  types a number. A closed list cannot hold a value the model refuses.
- **The sizes are kept but not yet applied**: the kit's type steps are
  constants. Applying them is the kit's (the lead's) change.
- **The machines' versions** (the page's About > Machines) come with
  slice 4, beside the Machines section that reads them.
- **The page's About** shows the page's own build; the app shows its
  own (the lead's card) and the software it carries.
- **Title weight through `Type::Label`**: a raw font weight lost the
  Hangul fallback faces (Korean drew thin), so the title is the Label
  step at 26/32.
- **Island ends are rounded per row** on a phone: GPUI clips
  `overflow_hidden` to a rectangle, so the first and last rows carry
  the corners.

Lead's files touched (one line each where possible): `shell.rs` (the
area's `start`; `routes::sidebar` in the wide sidebar's place),
`routes.rs` (`Route::Settings`, `sidebar()`), `areas/mod.rs`,
`areas/sidebar.rs` (the gear and the build label), `areas/about.rs`
(`render` split into `card`, `dialogs`, `content` for the section;
its route removed).

Gate: `cargo fmt --check` clean; `cargo clippy --all-targets -- -D
warnings` clean; `cargo test`: view 201, app 72, link 10, others 3 and
7, all passed.

## E2 takeover: machine book boundary

The inherited uncommitted screen was preserved outside the repository before
merging the desktop lead. It combined the list and detail on one page and
reimplemented section headers and buttons inside the area. It is not accepted
as a screen. The replacement follows Paseo's `screens/settings/host-page.tsx`
and `host-appearance-section.tsx`; a separate screen slice will supply captures.
The earlier table's device/agent scope is superseded: G3 owns Agents/Models,
G4 Devices/Sync, S2 Shortcuts/Notifications, L the language row, K theme tokens.
E2 owns the frame, General, Appearance, Machines and Diagnostics.

This first replacement slice is the file boundary, not a delivered screen:

- `view::machine_book::{Name, Edit, Failure}` validates the CLI's book-name
  grammar and changes selection only after a successful write. A failed edit
  keeps the current selection. Complete, actionable error sentences exist in
  both languages; file content and OS errors do not cross into those sentences.
- `link::machines` reads and edits with the existing CLI `BookFile`. No new
  persistence file, dependency or wire message. File work uses the blocking
  pool; actor commands serialize writes with refreshes. A successful rename
  keeps the existing client, and forgetting the watched machine cancels only
  the app's stream, leaving its remote session running.
- Actual temporary-file tests check rename/default/remove, preservation of the
  full entry and comments, external edits reread before saving, missing/taken
  failures without byte changes, malformed input without exposing its content,
  and owner-only permissions after repeated replacements on Unix. The existing
  machine tests retain the page's `data/machines.test.ts` inputs/outputs.
- Small shared seams: `link/lib.rs` module/command/event/dispatch and watched
  machine identity; browser refusal; `view/lib.rs`, text/test registrations;
  one shell event arm and a settings answer module. No unrelated area changed.
- No screen captures or real-node smoke are claimed for this boundary slice.
  Runtime UI and all offline/long/empty shapes remain the next slice.

Boundary gate: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`
and `cargo test` all exited 0. Tests: app 3, kit 19, link 20, view 243,
page parity 89; 374 passed, no failures; doc tests passed (zero cases).
The file permission probes ran as the ordinary developer account on isolated
files, including a denied directory write and repeated 0600 replacements.
No production files or permissions were changed.

Verification problems encountered: an initial run overlapped a source edit,
so its doc-test result was invalid and was discarded. Subsequent gates caught
a documentation identifier, an error type name, and missing statement
semicolons; those were fixed without changing lints. A mistakenly restarted
unchanged check was stopped rather than counted. The final gate above used a
fixed source tree. Build slots apply to subsequent build commands.

## Custom-width preparation: conversation seam only

`session::set_content_width` preserves the native default (Paseo's 820)
and delegates every explicit width to the preference's validated points.
This has the same behaviour for all three existing presets and accepts a
future custom width without adding a second width state. It is deliberately
a separate conversation-area seam; the numeric model and input follow.

Seam gate: native fmt, Clippy and tests all exited 0 through the shared
build-slot wrapper (formatting needs no slot). The same 374 tests and empty
doc-test sets passed. No screen geometry changed in this seam.

## Integrating the shared file-fault boundary

The desktop lead's file-fault change is reused rather than duplicated:
`TomlFault` never carries file content; `FileFault` supplies the app's safe,
localized notice. The merge keeps both text-module registrations and extends
the book failure match for the new project-file variant. Actor-level canary
and connection-lifetime regressions requested in review follow this merge.

Merge gate: native formatting, Clippy and tests all exited 0; app 3,
kit 19, link 21, view 243 and page parity 89 (375 passed), plus empty doc
tests. Root client/protocol changes are the lead's; no independent root
workspace gate is claimed for this integration.

## Review follow-up: the actor boundary, not only the writer

Review held `36b9139` for the legacy refresh error bypass, browser reply shape,
internal vocabulary and missing connection-lifetime evidence. The lead's safe
`FileFault` projection is reused for every actor read (tick/reload/retry/edit).
The actor resolves its book path once, then performs file reads on the blocking
pool; tests inject that path rather than changing process-wide environment.
Failed edits skip refresh and cannot alter clients or the watched stream.
Selection consumes the complete edit result in view, including failures.

Tests use actual `NodeClient` connections over the existing framed RPC on
bounded duplex streams. The peer answers and records typed protocol messages;
it implements no book or app logic. `vessel-rpc` is an existing transitive
library, now a direct dev-dependency only; no new shipped external dependency
or real agent is introduced. Tests cover rename preserving connections and
agents, failed edits preserving both clients and follow, watched forget closing
only its app connection, another forget preserving the active follow, and all
actor error events with a synthetic canary. Browser refusals preserve Read vs
Edited and the exact edit. UI aliases remain required before screen delivery.

Follow-up gate: native fmt, Clippy and tests all exited 0. App 3, kit 19,
link 29, view 245 and page parity 89 (385 passed), plus empty doc tests.
The first attempt stopped at Clippy on an unnecessary qualification and test
result/index-access lints; those were corrected before the passing full gate.
The denied-write actor test ran as uid/gid 1000 with its own parent at 0500,
restored to 0700 for cleanup. No deployed file modes or service data changed.

This is a boundary slice, with no new screen or capture claim. Open draft,
session and conversation aliases still need success-only integration. Review
also found workspace clone-follow tasks retain client clones: that owner's
cancel/rekey seam is required before whole-app disconnect is claimed. The
watched-forget regression here covers the session follow and two real RPC
clients, not every area's background work.
