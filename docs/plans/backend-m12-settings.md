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

## E3: Paseo's Settings, section by section (2026-11-14)

The owner (words 43 and 45): this version carries Paseo's features and
settings over as they are. Worker E3 rebuilds Settings on Paseo 0.15.4
(`packages/app/src/screens/settings-screen.tsx` and the section
components it names), branch `backend/m12-app-settings2`. The gap list
is outside the repository (the coordinator's survey of every Paseo
setting against Vessel's).

### Slice 1: the frame

Followed: `settings-screen.tsx` (`SIDEBAR_SECTION_ITEMS`,
`HOST_SECTION_ITEMS`, `SettingsSidebar`, `SidebarGroup`, `HostPicker`,
`sidebarStyles`), labels from `i18n/resources/en.ts` and `ko.ts`
(`settings.sections`, `settings.hostSections`, `settings.groups`).

- **Sections and order** (`view/src/settings.rs`): Paseo's 26 sections
  in its two groups. App: General, Appearance, Sidebar, Chat, Terminal,
  Browser, Editor, Files, Shortcuts, Integrations, Notifications,
  Permissions, Diagnostics, About. Machine (Paseo's Host): Overview,
  Projects, Connections, Pair device, Agents, Metadata, Voice,
  Workspaces, Providers, Usage, Terminals, Plugins; then Vessel's own
  Models and Sync. Paseo's `desktopOnly` (Browser, Shortcuts,
  Integrations, Notifications, Permissions) and `webOnly` (Editor) are
  `Section::available_on(Surface)`, and the app names its surface at
  build time (`settings/sections.rs`, `SURFACE`).
- **Where the page's sections went**: Machines is Paseo's Overview;
  Devices is Paseo's "Pair device" slot and label (the page also lists
  and revokes paired devices, which Paseo's does not); the agents page
  (installed, signed in, install, sign-in, defaults) is Paseo's
  Providers, so Paseo's host Agents section (Paseo tools, system
  prompt, restart recovery, idle release) is free for slice 3's
  idle-release card.
- **One registry** (`app/src/areas/settings/sections.rs`, `page`): a
  section is listed exactly when a renderer is registered for it, so
  the list and the pages cannot disagree; the placeholder page
  (`page::elsewhere`) and its sentence are gone.
- **The machine group** exists while the book has a machine, headed by
  the picker (Paseo's `HostPicker`); with an empty book it is one row
  under no heading, "Add a machine" (Paseo's "Add host"). Paseo's
  settings list has no search.
- **Motion** (word 39): a section's page fades in when it arrives
  (`motion::STANDARD`), the selected row's fill fades in as the
  selection moves (`motion::QUICK`); both are one-shot GPUI animations
  keyed by an arrival count, drawn at their end while motion is held.

Listed in this build: General, Appearance, About; Projects, Pair
device, Providers. Hidden, nothing behind them yet: Sidebar, Chat,
Notifications, Diagnostics, Overview, Agents (the next slices);
Shortcuts (track S3's page, one line in `page` when it lands); Terminal,
Browser, Editor, Files, Integrations, Permissions, Connections,
Metadata, Voice, Workspaces, Usage, Terminals, Plugins, Models (no
Vessel feature behind them yet); Sync (held back by the coordinator).

Differences from Paseo, stated:

- The group Paseo calls "Host" is "Machine" ("머신"), Vessel's word
  for a host everywhere else in the app ("Add a machine").
- The list keeps the workspace sidebar's width (264) and ground, not
  Paseo's 320 on `surfaceSidebar` (unchanged from slice 3).
- Glyphs: the kit gained Stethoscope (G2's, as it is), FolderGit and
  Boxes (original drawings after Paseo's FolderGit2 and Boxes) for the
  sections listed now and next; the hidden sections' glyphs the kit
  still lacks stand in, named in `nav::section_glyph`.
- The Providers page's list heading says "Providers on <machine>"
  ("<machine>의 프로바이더"), as its title, where it said "Agents on".
- The picker's name is in the foreground and it has no open fill, as
  Paseo's trigger (before: muted, with the selected fill while open).

### Slice 2: the app sections that have something behind them

Followed: `settings-screen.tsx` (`GeneralSection`, `DiagnosticsSection`,
`AboutSection`), `settings/general/sending-section.tsx`,
`settings/appearance/appearance-section.tsx`, `settings/chat/chat-section.tsx`,
`diagnostics/app-diagnostic-report.ts`, `components/app-diagnostic-sheet.tsx`;
labels from `en.ts` and `ko.ts`.

- **General**: Paseo's two sections. "General" holds Language (the row
  track L wrote, now a row of this card rather than a card of its own);
  "Sending" ("전송") holds "Send shortcut" (no hint, as Paseo's) and
  "Default send" with its hint for the value in effect, its list in
  Paseo's order (Interrupt, Steer, Queue). Paseo's default, Steer, and
  the hints' sentences are the conversation track's (C, `65cd84bc`,
  which owns what Send does); "Steer" is "지시" as Paseo's Korean.
  "Kept" (where the preferences are) stays as Vessel's own, after
  Paseo's sections.
  "Send shortcut" is not shown on a compact layout, as Paseo's (its
  `useKeyboardShortcutsAvailable`: a phone has no shortcuts).
- **Appearance**: "Theme" (the tiles), "Fonts" ("Interface size", "Code
  size") and "Layout" ("Content width") with Paseo's hints; the chat
  display moved to Chat.
- **Chat**: "Detail level" ("세부 수준") with "Chat display" ("채팅
  표시"), Paseo's hint.
- **Diagnostics**: "App diagnostic" with Run; the report
  (`view::diagnostics`, Paseo's text shape: the app, then each machine)
  opens in the kit's dialog with Copy and Refresh. Addresses, keys,
  pairing links and secret values are taken out (Paseo's
  `redactAppDiagnosticReport`), tested. The home folder shows as `~`
  (Paseo shows paths whole; Vessel's report is pasted elsewhere and an
  account's name in a path says nothing about the problem).
- **About**: the lead's card as it is (version, What's new, updates, the
  automatic-update switch, the log), then Paseo's "Connected hosts" as
  "Connected machines" ("연결된 머신", `settings/connected.rs`): each
  machine with its node's version ("v0.1.0", "Offline", or "—"). The
  version comes from the node's own `NodeInfo` at connection
  (`view::NodeVersion` on `Machine`, the lead's request: a type, not a
  string); it is also in the diagnostic. Not compared with the app's
  (Paseo marks a differing one): app builds and the node are numbered
  apart.

Hidden, with why: Sidebar (the sidebar does not read its group, sort or
items yet; S3's next slice, Paseo keeps those in the sidebar's menu) and
Notifications (nothing delivers one yet; S3's slice 4). Not drawn as
rows, nothing reads them yet: "Show setup again" (no walkthrough),
"Open location" (no side panes), "Always expand reasoning", "Chat
outline", font families, content size, syntax theme, "Test audio".

Differences, stated: three of Paseo's eight themes; sizes and width are
selects over closed ranges where Paseo types a number; two of Paseo's
three chat displays ("Collapse turns" waits on the conversation), so the
default stays Full; the diagnostic asks no machine anything new (it
reports what the app holds), so Paseo's per-host progress rows have
nothing to show; its text is English, as Paseo's.

### Slice 3: the machine's sections

Followed: `screens/settings/host-page.tsx` (`HostAgentsPage`),
`screens/settings/idle-runtime-cleanup-card.tsx`,
`components/settings/headings/settings-info-tip.tsx`; labels from
`en.ts` and `ko.ts` (`settings.host.agents`).

- **Agents** (`app/src/areas/settings/host_agents.rs`): Paseo's "Agents"
  section with "Release idle agents" ("유휴 에이전트 해제"): its title
  with the info tip (the kit's tooltip on a 16 info glyph), Paseo's hint
  and info, the kit's switch. It reads the machine's own settings
  (`node.settings`, protocol 12) when the page is shown for a machine
  and sends `node.set_settings` when switched; the switch waits while a
  change is out; a refused change says why under the hint and the switch
  shows what the node still keeps. While the machine is not connected
  the page is Paseo's one card, "Connect to this machine to manage
  agents", under no heading.
- **The model** (`view::idle_release`, tested): on is any number of idle
  minutes, off is never; turning it on restores Paseo's hour (the node's
  default, 60), and the hint names minutes other than 60 ("after 30
  minutes"); answers carry the machine and a ticket, and one for another
  machine or an older request changes nothing.
- **The call** (`link::node_settings`): one request for a read or a
  change, answered with the settings in force; failures cross as codes
  only. `link/src/lib.rs` gains six registration lines (1082 of its
  1086).
- **The kit**: `settings::row_titled`, the row with the caller's title
  element, for a title with its info tip; `row_at` delegates to the same
  body.

Differences, stated: a node older than protocol 12 says so in the row
(Paseo draws nothing; an empty section would mislead); the info tip
shows on a pointer only (the kit's tooltip has no touch mode); minutes
other than an hour are shown, and lost when the switch is turned off and
on (the command line still sets any number). Not shown, nothing behind
them in Vessel: Paseo tools, browser tools, the appended system prompt,
resume after restart, skills, agent profiles.

### Slice 4: the machine's Overview

Followed: `screens/settings/host-page.tsx` (`HostSettingsPage`,
`HostStatusBadges`, `HostConnectionError`, `RemoveHostSection`),
`screens/settings/host-appearance-section.tsx` (`HostAppearanceSection`,
`HostRenameButton`), `components/rename-modal.tsx`
(`AdaptiveRenameModal`); labels from `en.ts` and `ko.ts`
(`settings.host.appearance`, `daemon.rename`, `daemon.dangerZone`,
`daemon.remove`), "machine" where Paseo says "host".

- **Badges**: the status ("Online", or "Offline" in Paseo's warning
  tone), how the book reaches the machine (Direct, Over SSH, Paired; read
  from the book when the page is shown), the node's version ("v0.1.0",
  `NodeVersion::label`, shared with About).
- **Not answering**: Paseo's connection-error alert, in the app's own
  words (the sidebar's sentence), not the connection's raw error.
- **Appearance › Name**: the name, muted, and a pencil that opens Paseo's
  rename dialog: the field holds the name, Rename is off while the name
  is unchanged or being saved, an invalid name says why after Rename
  (Paseo clears it as the name is typed again), the dialog stays open
  until the book answers and shows its refusal inside.
- **Danger zone › Remove machine**: Paseo's row and confirm ("Remove
  laptop? This will delete its saved connection."); the confirm stays
  open until the book answers. The machine leaves the book (the CLI's
  `nodes remove`), never its node.
- **Rules** (`view::overview`, tested): what the typed name asks for
  (unchanged, a rename, or why not; a name another machine has is
  refused here first), and a dialog closing only on its own edit's
  success.
- The book's edits are the existing `link::machines` actor; the page's
  own dialog says a refusal, so `book.rs` no longer repeats it as a
  notice.

Left out, nothing behind them yet: Paseo's colour and sidebar-badge rows
(the app keeps no machine appearance), "Restart daemon" and "Update
daemon" (the node-control track brings the calls). Stand-ins until the
shared controls land: the pencil is a plain glyph button (G2's
`host_rename_button` is taken once it passes review; the call site then
changes), the name is not selected when the dialog opens (no select-all
in the kit's field yet; it waits on `kit/src/field.rs` being split in the
quality audit). The badges are the kit's (pills on `surface3`, where
Paseo's are bare outlines). Remove now has Paseo's Trash2 before its
words (`kit::outline_button_with_glyph`, 14 square, 8 to the words, both
in the destructive colour; the lead's decision for 0.0.0.0.18).

### Slice 5: the machine's Connections

Followed: `screens/settings/host-page.tsx` (`HostConnectionsPage`,
`HostConnectionError`, `ConnectionsSection`, `ConnectionRow`,
`formatHostConnectionLabel`); labels from `en.ts` and `ko.ts`
(`settings.host.connections.title`, `badges.relay`, `badges.remoteSsh`).

- **Routes** (`view::connections`, tested): every way the book reaches
  the machine, in the order the client tries them: a websocket with a
  token, an `ssh` target, or a paired machine's own addresses and then
  its relays. The link fills them from the book when it reads it
  (`link::machines::Machine::routes`).
- **Endpoints only**: a route shows its host and port (Paseo shows
  endpoints); the scheme, a `user@` part, the path, the query and the
  fragment are cut where the route is made, so nothing more of a URL
  reaches the screen. An ssh target shows as the book names it.
- **Rows**: Paseo's titles: "TCP (127.0.0.1:7077)" for any direct
  address (a paired machine's too, as Paseo names every direct
  endpoint), "Relay (relay.example:443)" / "릴레이 (…)", "Remote SSH
  (me@gpu-box)" / "원격 SSH (…)"; one line, cut with an ellipsis.
- **Not answering**: the same alert as Overview (Paseo's page puts
  `HostConnectionError` above the section).
- **One read of the book**: Overview and Connections share it
  (`settings/book.rs`): asked when either page is shown for a machine
  it was not read for, asked again after an edit succeeds, dropped
  when both are left.

Left out, nothing behind them yet: the latency probe beside each row
(Paseo's "…", "12 ms", "Timeout") and the ghost "Remove" with its
confirm: the book's actor edits a machine (rename, default, forget),
never one of its routes, so a route cannot be removed here; removing
the machine is Overview's "Danger zone", and a paired machine gets a
new set of routes by pairing it again through the CLI.

## Overview module boundary and native quality (2026-11-15)

[Decision Log]
- Purpose: restore the native size gate without changing a screen or the
  browser demo's results.
- Existing assets and constraints: Overview already separates book-edit
  rules (`view::overview`) from drawing, but its page and dialogs share a
  file. The browser demo similarly mixes its catalog with its log actor.
- Alternatives: a larger allowance would preserve both mixed
  responsibilities; a new implementation would risk changing behaviour.
- Choice (modify): move Overview's dialog drawing into `overview/dialogs.rs`
  and the demo's agent/options catalog into `demo/catalog.rs`. Keep the
  same functions, expressions, callbacks and visibility at existing callers.
  Neither responsibility needs an external library.
- Consequences: existing tests remain unchanged. The modules become small
  enough to inspect independently; the demo's old size allowance shrinks
  or disappears, and the link entry's allowance records its current size.
- Reference: this is a structural change only; the Overview reference
  remains `screens/settings/host-page.tsx` and
  `screens/settings/host-appearance-section.tsx` listed in slice 4.

### Resume point

- Saved baseline: Connections and the shared outline-with-glyph Remove
  control are committed; their inherited captures have not been re-reviewed.
- Done: extracted Overview dialog drawing and the browser demo catalog;
  the demo allowance shrank from 591 to 528 and the link entry records 1082.
- Next: run the complete native gate on this committed tree. After that,
  consume the shared rename control and verify its actual input bounds.
  No select-all change is included.

The size check previously stopped before the token check. Its next check
exposed seven existing `px` literals in Overview, host Agents and Diagnostics.
They now use the existing equal-valued spacing tokens, including the
negative info-tip inset; no measurement or behaviour changes.

### Integration resume point

The native line already extracted the same Overview dialogs. The merge
keeps that implementation and its `dialogs` entry; the existing Remove
leading-glyph call remains. Host Agents keeps the native line's named
negative inset. The size list takes the lower measured counts: link entry
988, demo 523 after combining the native line with the catalog extraction.
Conflicts were limited to these equivalent extractions, imports, equal-valued
geometry tokens and shrinking allowance records. No rule or test expectation
was changed. Next: gate this integration, then replace the temporary pencil
with the shared host rename action now present in the native line.

The extraction's first narrow Clippy found a missing test-only
`OptionCategory` import. The existing test now imports it directly; its
inputs and expectations are unchanged. The superseded whole-gate request
was cancelled while queued. Resume: finish the corrected narrow check,
commit, then submit the complete native gate before the rename consumer.

### Quality result and shared rename consumer (2026-11-15)

`caa0f632` passed the complete native gate: fonts, formatting, size,
boundaries, tokens, whole Clippy, normal app build and tests; 1,169 passed,
none failed, one ignored. The build and test logs both recompiled link,
kit and app. A missing test-only import found by the first narrow check
was repaired before that gate; no existing expectation changed. The
original entry file budgets now measure link 988 and demo 523.

[Decision Log]
- Purpose: replace Overview's temporary pencil with the existing shared
  host rename action, preserving the editor and book-edit rules.
- Reference: `screens/settings/host-appearance-section.tsx:31-71,310-318`
  uses the static Pencil action with eight points of hit slop; the name
  row is at 279-288. `components/rename-modal.tsx` owns the editor.
- Existing assets: the native line already contains `host_rename_button`
  and its token geometry (`d103ab39`); another area-owned action or a new
  dependency would duplicate it.
- Choice (modify): call the shared action unchanged. Attach the editor
  callback only to its enabled state. Invalid book names retain the
  existing absence of an edit action. Do not add hover/pressed paint.
- Verification: actual Overview input at and outside the visual frame;
  a separate kit audit of disabled cursor/handler refusal, neighbouring
  controls, parent propagation and overflow clipping. The audit must call
  the real shared control, not recreate its geometry or hit handling.
- Scope: no editor engine or select-all change. The shared action's
  accessibility and keyboard limitations remain as documented by its owner.

### Resume point

- Quality repair committed and gated; review requested before proceeding.
- Next: install the shared-action consumer and isolated input audit, gate
  them, capture both schemes/languages and layouts, and report measured
  bounds separately from screenshots and reviewer approval.
- Select-all remains waiting for the field module split; do not enlarge
  that file for this consumer change.

The shared-action consumer and audit are now installed. The baseline
Overview action's actual pointer hit measured 22 points on each axis. An
initial probe used display coordinates and therefore sampled outside the
centred window; it is invalid evidence. The corrected probe uses the
window belonging to the capture process. Next: finish the narrow check,
commit and run the native gate while collecting the new action's bounds
and the isolated audit's results. No editor rule changed.

### Structural repair review and exact gate evidence

The structural repair through `caa0f632` passed independent source and
native-gate review. Exact run: `settings2-20261115T034150Z-caa0f632`;
all six steps ran with exit zero and none skipped. Tests: 1,169 passed,
zero failed, one ignored. The step log's SHA-256 is
`694679fcbb2430907123bdace632b81ede53703117c0e29d1756f3b87ffd7f67`.

That disposition covers the equivalent module extraction, lower measured
budgets and test-only import repair. It does not newly approve the inherited
outline-with-glyph implementation, rename input bounds, disabled/adjacent/
clipped states, select-all or the inherited capture set. Only the baseline
Overview capture was opened during that review. The shared rename consumer
and its first-use evidence remain a separate slice.

Current resume: the shared rename consumer and input audit are committed
as `ae6ba8f6`; their native gate is queued. Local normal app build and new
captures are in progress. No editor engine or select-all change is included.

### Long name and release isolation follow-ups

The shared-action consumer `ae6ba8f6` passed its complete native gate
(run `settings2-20261115T040434Z-ae6ba8f6`, all six steps zero, 1,169 passed,
zero failed, one ignored). Desktop and compact input probes measured the
same 38-point hit and passed nine edge/inside/outside cases each. The
74-image matrix covers both schemes and languages, four widths and the
requested larger scales. The original scale-two display clipped two
physical pixels; those six images were excluded and recaptured on a
larger virtual display with the same requested application dimensions.

A supported maximum-length book name exposed a separate layout defect:
the trailing value squeezed the title into a letter per line. The name
was saved successfully; the first file check raced its asynchronous
completion. The capture harness now waits for the saved-name condition.
Choice (modify): use the existing custom-content row frame, keep the
name title nonshrinking and one line, and let the right-aligned value
share the remaining width and ellipsize. The reference's name row
(`host-appearance-section.tsx:279-288`) stays on one line in both layouts;
its shared frame is reused rather than copied.

The sibling-overlap audit remains an open finding: a later transparent
neighbour and rename can both receive the same press. The expected
neighbour-only result is retained, not changed to pass. At native line
`1d1f11df` there is no product call to this shared action; Overview's
existing name value has no clickable neighbour. A global button hit-test
occlusion policy requires its own kit decision. It is outside this
follow-up. The separately assigned rename-control repair will isolate
its bubble-phase release from a parent while preserving its caller's
click; it must not stop capture phase or add a second click callback.

Current resume: first fix and capture the long-name row; then add a
failing parent-release event regression and the small shared-control
repair. Select-all follows those changes and the field module split.

### Parent-release control decision

[Decision Log]
- Purpose: an enabled rename action consumes its left-button release as
  well as its press, so a parent's mouse-up action does not also run.
- Existing implementation: the shared action stops mouse-down only. Its
  caller still owns the editor callback; disabled callers attach none.
- Source: pinned `gpui-pre` 0.3.10, `elements/div.rs` registers custom
  mouse-up listeners before its generated click listener (2848 and
  3143); `window.rs:5992-6008` dispatches capture forward and bubble in
  reverse. The generated caller click therefore runs before a custom
  bubble mouse-up listener on the same element.
- Alternatives: capture-phase suppression can prevent the caller's
  generated click; another click callback would obscure callback order;
  global button occlusion is a separate kit policy.
- Choice: add a left-button bubble mouse-up stop only for Enabled, after
  the existing mouse-down wiring. Keep the API, geometry and paint.
- Validation: a real GPUI event test first demonstrates the leaked parent
  release, then verifies caller-once, no parent release, Disabled with no
  handler, and a neighbour outside the rename hit remaining reachable.
- Limit: the later-sibling overlap case keeps its neighbour-only
  expectation and remains an open finding pending the global kit policy.

The long-name fix is `ad2d4ef3`. Its local app build passed; four
scheme/language/layout cases show the maximum-length name with its label
and action preserved. The temporary book rename was restored byte for
byte, with mode 0600, as the development account. Its complete native
gate remains queued; no gate pass is claimed for it yet.

The new parent-release regression reproduced the leak (caller one, parent
one, expected zero) before the control change. With the enabled root's
bubble mouse-up stop, the same test passed, including Disabled and a
reachable neighbour outside the hit. The live audit now displays a
separate parent-release counter. It retains the later-sibling case and
its neighbour-only expectation as an open kit-policy finding.

Resume: finish narrow Clippy, commit the control-only change, run the
native gate and inspect the new live counter captures. The long-name
commit's complete gate has now passed: run
`settings2-20261115T050249Z-ad2d4ef3`, all six steps zero, 1,169 passed,
zero failed, one ignored. No unchanged native gate was rerun.

The corrected event regression passed (one test, both enabled/disabled
branches); the control's caller still runs once and an outside neighbour
still receives its click. The final kit all-targets narrow Clippy passed
after splitting the audit parent wrapper from its row composition. Next:
complete native gate and fresh live parent-release captures.

### Rename package: completed source and gate evidence (2026-11-15)

The preceding queued/resume statements are historical. The parent-release
control is committed as `990da6dc` and passed independent V1 source/gate
review. Run `settings2-20261115T052611Z-990da6dc` ran all six native steps
without a skip, all exit zero: 1,170 passed, zero failed, one ignored.
The step log SHA-256 is
`a79854cc5735ec197af1da4ea732a9f37c9bf3593bb1d1624e1f38edb6c70ecf`.
The actual before-change regression failed with caller one and parent
release one; after the change it passes with caller one and parent zero.
Disabled retains no handler and the outside neighbour remains reachable.

Fresh release captures use the completed normal build of `990da6dc`, not
the earlier executable. The `rename-release` set has 74 images: four
window sizes, English/Korean, dark/light, plus the requested 1.25/1.5/2
scales. All physical dimensions match; every idle/hover pair is equal.
One English dialog title was missed by OCR; the retained original was
opened and its title, input and footer are visible. The parent-release
audit's two theme sets agree with the event regression. These captures
and the long-name consumer evidence are submitted for visual review;
the source/gate disposition alone does not approve them.

The deliberately overlapping later sibling still receives a click along
with rename in both theme audits. Its neighbour-only expectation remains
failed. No product layout at the checked release head puts a clickable
sibling beneath this control. This is the coordinator's deferred global
button occlusion policy, not a claimed fix or a removed test. Select-all
and the inherited Remove-with-glyph implementation remain outside this
parent-isolation approval.

Next: wait for the native lead's validated package base before changing
field source. Then split field presentation and colour projection from
state/registration/events without changing behaviour; commit the move
separately before adding select-all. The new A14 package rule applies:
small commits receive narrow checks, one full gate and capture submission
at the end of the user-visible package.

Diagnostics remains explicitly local app facts. N1 has approved bounded
value types, but no public request, client call or assigned protocol yet.
Do not register a stub or duplicate the node collector. Wire it only after
the actual producer handoff, using the existing admitted identity and
request-generation mechanisms. Node update awaits nodectl's ADR/call;
Plugins belongs to its separate package owner.

### Field responsibility split on the validated package base

Merged the announced package base `8d5513e4` as `0671404b`. The field's
state factories, weak line registry, focus/IME queries and event listener
remain in `field.rs`; `field/presentation.rs` owns the existing form and
composer drawing, and `field/styles.rs` owns engine/component color
projection. Public methods and the original composer-card/theme-apply
paths are preserved. Only the internal projection helper gains sibling
visibility. All moved function bodies and existing test expectations are
unchanged; the composer hooks file is unchanged. The field allowance is
removed once its measured product size is below 500. This commit adds no
selection, readonly, composer key mode or layout configuration behavior.

The next separate behavior commit owns rename select-all/Editability.
K owns the later additive blur factory and Appearance profiles; W12 owns
the later composer layout/ButtonOnly policy. Neither is folded into this
move-only change. Narrow compilation and existing field tests are owed
before reporting the split as usable.

### Selection consumer on package base 2

The selection/profile experiment is reapplied after `6036eeb8`, using
W4's sole `Editability` and `set_editability`. No E2 editability file or
event subscription is restored. `field/selection.rs` adds only whole-value
selection; its tests also exercise the shared editability API. Rename
sets ReadOnly before dispatch, restores Editable on the matching answer
or reopen, and focuses/selects the initial value. The one book-answer
caller now passes its existing Context for the immediate state update.

A shared kit-internal `field::settings_frame(&TextField)` provides only
the I-beam cursor and padding-to-existing-editor focus bridge. Rename
and K's later family/number/width profiles reuse it; each supplies its own
chrome. The rename profile is surface0/padding12/radius6/border1, without
a hover ring, density branch or extra disabled paint. Existing native
Body14/20 is retained. Existing form/composer renderers remain unchanged.

Prepared engine regressions cover selection of ASCII/Hangul/emoji before
the first edit, keyboard/paste/IME refusal, re-enable with draft retained
and reopen. Narrow compilation/runtime and actual dialog captures are
still owed; prepared tests are not counted as passed.


Pre-capture source comparison found an inherited button-state difference:
Paseo's AdaptiveRenameModal disables submit for a blank draft, while the
existing native condition enabled it whenever it differed from the name.
Editing::rename_enabled now owns the nonblank/change/pending rule and the
screen delegates to it. Enter still runs validation and shows the required
name error. A pure transition fixture covers idle blank/unchanged, pending
and failed-write retry. No input engine, grammar, store or control paint
changes. This correction is prepared before the first normal capture
compilation; its new fixture still needs execution with the package gate.


### Queued-only correction before the final native run

The first final request settings2-20261115T130924Z-0610958f was formally
superseded while still queued. The supported queued-only cancellation
verified its process group and reached cancelled143 before replacement;
no compiler or test step had started. Its original records remain intact.
This is an authorized source amendment, not a compiler failure or a pass.

The search owner supplied00ebfdeafaac650462d73853b54050ac57ca2f84:
only the two cfg(test) counters inside ordinary cache methods change
from +=1 to saturating_add. Native arithmetic_side_effects is denied;
no actual lint failure on061 was observed. There is no production,
expectation, threshold or public API change. The coordinator authorized
this exact queued-only amendment to avoid spending a whole gate on the
known strict-lint issue. Freeze the corrected source and register the
same complete native gate with supported app export once, in the ordinary
lane and a fresh output directory. Captures still await that actual export.

### Executed selection package: 2026-11-15

The preceding queued/export debts are superseded by actual source
`0a0a5a87bc490620ccccebca104b8a2686319450`, run
`settings2-20261115T135403Z-0a0a5a87`. Native fonts, format, quality,
whole all-target Clippy, normal app build and tests each returned zero;
tests were **1420 passed, 0 failed, 2 ignored**. The logs show the changed
View, Kit, Link and App compiling/checking. Supported app export also
returned zero. Original logs, spec and six-step digests are retained in
`/tmp/work/m12-settings4/rg-selection-final2`; this is the final consuming
source, not a pass borrowed from the lead. No unchanged whole rerun is due.

The exported normal binary SHA256 is
`282f70d9ecd0662398fdc22ecebedb21e8d7170440e7dfd8d83a129f48c6300b`.
Eight real flows cover 1280x900 and 390x844, EN/KO and dark/light:
opening selects the existing name, first input replaces it without Ctrl+A,
blank input is refused, reopening selects again, pending edits/Backspace/
Escape are ignored, failed saving preserves the draft, and a subsequent
edit/retry commits exactly that draft. All eight completed. The three
actual selection-engine tests, including typing/paste/both IME replacement
paths under readonly, and the blank/unchanged/pending submit fixture also
passed in the combined gate. These are synthetic engine tests, not a
physical IME or operating-system claim.

There are 56 original PNGs in `/tmp/work/m12-settings4/shots/selection`.
`selection-capture-manifest.json` records every SHA/dimension and the
explicit subset personally opened; `selection-capture-results.json`
records each flow. Pending is forced only in the isolated legacy reader
using a controlled FIFO, then a real directory-write refusal. This does
not prove the future safe-file API (which must refuse FIFOs), latency or
new format/writer behavior. Capture-driver failures are preserved as
attempt-01 and manual unfocused artifacts; they are not product failures
or passing UI evidence. The final driver explicitly focuses the window.

The capture app used the approved shared protocol-23 root binary, with
its different source identity explicit (`same_root_commit=false`); the
actual app showed the fixture connected. Runs used UID/GID 1000 with
private directories 0700 and book/credentials 0600. Original book bytes
and modes were restored after every case. Only the verified owned node
and Xvfb were stopped, checking PID, executable, owner and start time;
the app was already stopped by the driver. No production permissions,
services, real credentials or customer data were changed.

Review requested is bounded to the pure render/style move, shared-engine
selection/frame/rename consumer and its submit rules. Inherited pending
label remains Rename rather than Paseo's Saving; error typography remains
Meta rather than its base size, and existing footer/frame adaptation was
not redrawn. The previously accepted parent-isolation change does not
resolve the separately deferred sibling-occlusion finding. No full E1/F5,
physical keyboard/IME, hardware GPU, accessibility or latest combined
release approval follows from this package. V1 review remains pending.

Public book values have a separate route: strict proto all-target Clippy
on `7becce7f` actually returned zero. Per the coordinator's corrected
decision, its five pure root/docs commits were cherry-picked by the node
integrator as a58e08bc/c1717a02/d9bbb8c0/390fc201/c75d6391. Their A12 and
nine root tests belong to that integration receipt, still awaited here;
the app receives them through an announced base. No app ancestry is
merged into the node line. Native appearance compatibility and E1's
private-files/routes implementation remain separate unfinished work.


### Same-package V4 return: previous failure belongs to the old draft

V1 forwarded the final packet to V4 under the review-balance rule. V4
row239 verified the exact six-step gate, exported binary and all56 images,
but returned one actual consumer defect: typing after a failed save cleared
the validation error only; the dialog still displayed Editing.refused.
The retry-editable originals truthfully show the old failed-save sentence
alongside the new draft. Those scenes are evidence of that defect, not a
passed error-clearing state. Existing selection/pending/retry checks still
have their recorded narrower outcomes; the package is not approved.

Paseo rename-modal.handleChange clears all displayed error on edit. The
correction keeps the existing TextField subscription and clears both
previous validation and book refusal only while that rename is editable.
Pending/mismatched-answer behavior and editing engine are unchanged. A real
consumer regression renders the actual dialog, dispatches through a detached
link, injects mismatched then matching failed answers, types through the
real TextField and checks absent prior error, retained new draft and exact
retry command. No model-only stand-in or network/file task is involved.
Its execution and changed-source checks/export/affected scenes are owed;
no unchanged whole gate or full matrix is requested by the return. The
earlier sibling policy and disclosed typography/label adaptations stay
separate.
