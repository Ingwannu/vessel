# Vessel's native app (milestone 12)

The native app ADR 0035 decided on and ADR 0036 makes the one experience
on every surface: the same UI crate on the desktop (Linux, Windows,
macOS) and on a phone (Android through the community mobile layer; the
host project is `android/`, track M's), with the page's look, words and
behaviour. It is a client of its nodes as the `vessel` CLI is: the same
node book, token files and device credentials, reached through
`vessel-client` (directly, or through a node's relay over the secure
channel; SSH routes come later).

This is a workspace of its own, outside Vessel's: Vessel's gate does not
build it, and its tree (GPUI's) stays out of the main lock file.

## Crates, and what may depend on what

| Crate | Owns | Must not | Depends on |
| --- | --- | --- | --- |
| `view` (`vessel-app-view`) | what a screen means: the board's lanes and phrases, a conversation folded from its records, the permissions waiting; **the words** (`view/src/text/`, the legacy area tables; `view/locales/`, the staged Fluent catalog of ADR 0039) | use any UI toolkit; do I/O | `vessel-proto` |
| `link` (`vessel-app-link`) | the nodes: the book, connecting, the session list, a session's stream, prompts, answers, pairing; the app's remembered choices (`prefs.rs`); a tokio runtime of its own, commands in and events out on bounded channels (a command the queue does not take comes back typed from `Link::submit`, and the app settles what waited on it in its own words: `app/src/unsent.rs`) | use any UI toolkit | `vessel-app-view`, `vessel-proto`, `vessel-client` |
| `kit` (`vessel-app-kit`) | Vessel's kit: the design system's tokens in both themes, the type scale with the faces embedded, the controls (button, choice, text field, Markdown), the window's chrome (title bar, window controls, resize edges), its glyphs | know about nodes or sessions | - |
| `app` (`vessel-app`) | the window, the routes, the areas (screens), the binary | name `gpui-platform` outside its entry points (`main.rs`; `web.rs` in a browser build); depend on `gpui-component` (the kit's); hold a sentence of its own (words come from `view::localization` or the remaining `view::Text` tables) | `vessel-app-kit`, `vessel-app-link`, `vessel-app-view`, `vessel-proto`, `vessel-app-log`, `vessel-app-updater` |

```
app ──> kit ──> GPUI (gpui-pre 0.3.10) + gpui-component 0.8.1 (text field, Markdown only)
 │
 ├──> link ──> view, vessel-client, vessel-proto
 ├──> view ──> vessel-proto
 └──> vessel-proto; on the desktop vessel-app-updater, vessel-app-log (spikes/release)
```

The table is checked: `node ../../scripts/check-boundaries.mjs --workspace .`
compares the last column with every crate's normal and build dependencies
on `vessel-*` crates (in both directions), and holds the two tables below;
it builds nothing (`cargo metadata --no-deps`). A new edge is a change of
this table, with its reason in the same commit.

### External crates with one owner

| Crate | Who may depend on it | Why |
| --- | --- | --- |
| `gpui-pre` | only `vessel-app-kit`, `vessel-app` | Nothing of GPUI's reaches `view` or `link` (ADRs 0035, 0036). |
| `gpui-pre-platform` | only `vessel-app` | The platform layer is chosen where the app starts. |
| `gpui-component` | only `vessel-app-kit` | The component layer (text field, Markdown) is reached through the kit. |
| `gpui-base` | only `vessel-app-kit` | The kit's field draws on its editing engine. |
| `fluent-bundle` | only `vessel-app-view` | The words live in `view` (ADR 0039); the app holds no sentence. |
| `vte` | only `vessel-app-view` | The terminal parser (ADR 0073). Only the grid decodes PTY bytes; the kit draws cells it is given. |
| `tokio` | never `vessel-app-view`, `vessel-app-kit` | `view` does no I/O and `kit` draws; the runtime is `link`'s (and the app's entry). |
| `cpal` | only `vessel-app-link` | One desktop device owner shares capture/playback between dictation and voice (ADR 0080). |
| `sonora` | only `vessel-app-link` | The audio owner supplies AEC, noise suppression and gain control through the same reverse stream (ADR 0080). |

### Names only some files may use

| Names | Crate | Where | Why |
| --- | --- | --- | --- |
| `gpui_platform` | `vessel-app` | only `src/main.rs`, `src/web.rs` | The desktop's and the browser's entry points pick the platform; screens draw through GPUI and the kit. |
| `std::fs`, `std::net`, `std::process`, `std::env`, `std::thread`, `tokio` | `vessel-app-view` | only `build.rs`, `build_support/` | What a screen means is computed, not fetched; the build script reads the catalogs at compile time. |

Nothing of GPUI's reaches `view` or `link` (ADR 0035, 0036): the browser
track compiles them to WebAssembly, and the layer under the kit can be
replaced without touching them.

Settings' agent discovery remains in `view::agent_status` (the page's
fixtures). `agent_status::readiness` separately describes the node's
installation, conversation support and authentication facts from
`agent.status` (ADR 0032). Unknown sign-in is never inferred from a
successful handshake; installer offers target only the missing part.
Its row explanation includes a typed short badge and keeps the machine's
diagnostic separate from translated instructions. Sign-in help is typed
as labelled account, command and method steps, so a screen can copy the
exact node-provided command without inventing a login flow.

Provider-removal preparation stays in `view::agent_status::removal` and
reuses the existing machine-page tickets. Agent snapshots retain the
reporting `NodeId`, so a reused book name cannot stand in for identity.
The prepared request is not yet connected to a destructive call or control.

The existing agent-option query/cache method lives in private native
`link/src/options.rs`; the root actor still calls it for `Command::Options`.
Its module split changes no request, cache order or event.

The link's crate root is kept under the 500-line budget so every package
can register without growing a listed file: `Command` lives in
`link/src/command.rs`, `Event` and `KeptLists` in `link/src/event.rs`
(both re-exported from the root), the actor's command dispatch
(`State::handle`) in `link/src/dispatch.rs`, and its refresh and
connection (`refresh`, `connect`, `hand_kept`, `keep_commands`) in
`link/src/refresh.rs`. A package adds its variants and its dispatch arm
there, its own module and hub with one line each in `lib.rs`, and its
logic in its own module.

Data flow: `link` lists every book node's sessions every two seconds (the
page's cadence) and sends a `Fleet`; opening a session follows its log
from the start and sends its records, which the shell folds in batches
into `view::Conversation`; the areas draw the result. Everything a person
does that reaches a node is a `link::Command`.

The command palette's matching lives in the shared pure `view::matcher`
module; its cursor and result bands stay in `view::palette`. Provider
catalogs reuse the same parsed `Query` and opaque ordered `Score` with
their visible name/id fields. Each query token must match a visible label/subtitle or hidden
keyword; subsequences stay within a word. Visible matches rank first,
command groups rank by relevance, and session navigation follows below
them by match tier and last activity. Before searching, only five recent
sessions appear; archived sessions are excluded. A query resets selection
to the best result; passive fleet updates retain identity and do not
scroll. The app only supplies entries, draws kit rows and reveals the
selection when the person types or presses an arrow.

Git action preparation lives beside the code panel's existing models in
`view::workspace::git`: commit scope and validated message, branch
create/rename/switch, per-file index actions, discard decisions and stash
identity. It reuses `workspace::git_quick` and the workspace's refusal
words, and produces the existing wire requests for `link::workspace`.
The node remains authoritative; preparation never runs a local git
command. The G1 delivery notes are `docs/plans/backend-m12-g1.md` in the
repository root.

## The app's layout

```
app/src/
  main.rs          the window (the kit's chrome), language, theme
  shell.rs         the window's state: route, fleet, prefs, areas; link events dispatched; the frame
  routes.rs        THE registry: `Route`, what draws each, what arriving does, the sidebar's list
  chrome.rs        what is in the title bar (the mark and label; the update's status at the right)
  update.rs        the updater (track R's crate) on the shell: its state followed, check now,
                   automatic or only when asked, restart into a new build, "Updated to …" once
  brand.rs         the brand's drawing behind the content panel (ADR 0059): the screens report
                   their boxes while laid out, the backdrop draws the same frame; the first
                   message's journey; brand/avatar.rs, the agent message's avatar
  areas/
    mod.rs         one `mod` line and one field of `Areas` per area
    common.rs      pieces several areas draw (a frame's title, its foot, a session's colour)
    draft/         the new chat: Paseo's composer card, its pickers (pickers.rs), the
                   step of Paseo's ladder the card's width allows (density.rs), the
                   features as controls or as Paseo's features sheet (features.rs)
    session.rs     a session: the conversation and the composer in Paseo's one chat
                   column (at most 820 wide, 16 from the edges), permissions, stop
    terminal/      terminal-session lifecycle, background replay, appearance and
                   cursor presentation, and the bounded scroll viewport; adapts
                   view's parser/grid to the kit's single terminal_grid renderer
    composer/      what the composers share: an agent's lists kept and refreshed
                   (offers.rs), a running session's controls (running.rs, modes.rs),
                   the `/` menu (slash.rs), drafts and recall (drafts.rs)
    about.rs       About as Paseo's settings section: the version, the app's updates
                   (Paseo's words), automatic updates (a switch), the log file; and the
                   note shown once after an update
    pair.rs        adding a machine: Paseo's paste-link modal, then its host
                   confirmation, over the new chat
    board.rs, archive.rs, sidebar.rs
```

### Workspace panes and session resources

`view::layout` owns typed targets and keys, validated split tree, placement,
compact sheets, close plans and durable projection. `areas/layout` is the sole
native frame/renderer registration point, reusing kit tabs, menus, buttons and
resize gestures. Feature areas own their content and backend lifecycle. No
feature creates a parallel pane manager. The native adapter currently retains
an explicit session-route bridge until authoritative workspace resolution is
available; equal paths never imply equal workspace identity.

The reviewed session resident contract is in
[`wp-layout-residents.md`](../../docs/plans/wp-layout-residents.md).
`view::residents` defines the pure per-window admission/focus/lifetime table and
per-attempt reply fences. It contains no GPUI state or connection identity cache.
The current SessionView and ConversationArea globals remain the only live product
owner while concrete owner adapters are prepared; the coordinated cutover must
remove those globals and the single follow together. Pure core tests do not prove
multiple agent panes work. Package details and evidence limits are in
[`wp-layout.md`](../../docs/plans/wp-layout.md).

### Adding a screen (an area)

1. Its meaning in `view` (a module of its own, ported from the page's
   `web/src/data/...`, with the page's test cases as its tests: same
   inputs, same outputs; fixture files used by path, not copied).
2. Its node calls in `link`: a `Command` variant, an `Event` variant
   when it answers, the call in `State::handle`.
3. Its words in `view/locales/{en,ko}/<area>.ftl`, both languages with
   the same variables and context comments. The English file generates
   the typed API; use `shell.words.<area>()` for the prepared catalog.
   Format messages with values in view when their data changes, retain
   typed data to retranslate an existing outcome, and borrow plain labels
   when rendering. Follow [the translation guide](view/locales/README.md).
   An active area's existing Rust table stays with its owner until its
   coordinated migration; `shell.t` adapts those remaining areas together.
4. Its screen in `app/src/areas/<area>.rs` or `app/src/areas/<area>/`:
   a state type (fields: `TextField`s, choices), an `impl Shell` block
   for what a person does there (methods on the shell, so a click reaches
   the link, the fleet and other areas), `pub(crate) fn render(shell,
   tokens, …) -> Div`, and `arrive` if arriving focuses or settles
   something.
5. Register it: one `mod` line and one field in `areas/mod.rs`, one
   variant and one arm in each match of `routes.rs`; `Route::NAV` if the
   sidebar lists it. An event from the link for it: one arm in
   `Shell::on_event` (`app/src/shell/events.rs`) calling the area's method.

Settings areas with nested pages can supply a scroll identity at the
`scroll_id` slot next to their content registration. Agents uses one for
its machine list and one per agent detail; a detail’s offset therefore
cannot hide a different page’s title. Other sections retain their existing
scroll identity. This is the frame’s one outer scroller, not an area-local
parallel page layout.

The Devices page keeps invitation text outside text fields: only an
explicit reveal projects it for up to thirty seconds or until expiry,
whichever comes first. Copy, focus loss and navigation mask it. Its pure
state and stale-response tests live in `view/src/devices/`. Device rows
have one access/last-use hint; exact pairing/last-use dates and all scopes
stay in their tooltip and in the read-only details opened by the row, so
touch does not depend on hover. Details and revoke use one view selection
rule and never open a revoked/missing row or replace a pending confirmation.
Ordinary paired rows have no repeated state badge.
Devices binds its public data and callbacks to the authenticated NodeId and
an actor-lifetime admission generation. The existing connection registry in
`link/src/connections.rs` owns parsed book bindings and retirement; it is
independent of whether reading sessions succeeds. A reload, observed binding
replacement, successful rename/forget or transport loss retires old work.
Confirmations additionally carry a distinct, non-reused intent. Requests and
replies keep the same context; neither an alias lookup nor a late reply can
retarget an operation. The contract and explicit observation limits are in
`docs/plans/backend-m12-device-targets.md`. Effective scopes remain
unknown until Devices queries the existing device.self call; a failed session
read is no substitute.
The area's `devices/row.rs` composes the shared row/button so a wrapped
badge/action group retains its trailing edge; its bounds cases live beside
it (verification status is in the G4 notes).

Settings lists Paseo's sections in Paseo's two groups and order
(`view::settings`: App, then the chosen machine's, Paseo's Host, headed by
the machine picker; Paseo's `desktopOnly`/`webOnly` per surface). A
section registers its renderer once, in `settings::sections::page`; a
section without one is not listed (no placeholder pages), so the list and
the pages cannot disagree. State and event slots are added only when needed.
Overview and Connections share one read of the machine book
(`settings/book.rs`, asked while either is shown); a route reaches the
screen as its endpoint only (`view::connections`).

Sync is read-only: `link::sync` projects manifests into names and metadata
before a UI event exists. Partial failures remain explicit; secret values
and tool transport settings have no field in its screen model. Local
commands use the kit's `settings::code_row` and trailing copy button; the
app never executes them. Member rows keep one compact version/report hint;
conflict paths and the received-copy suffix live only in the following
alert. The home row's actual composition has a GPUI bounds regression in
`app/src/areas/settings/sync/frame/tests.rs` (no app shell or node starts).

### Adding a kit control

One file per control in `kit/src/` (`widgets.rs` holds the smallest),
drawn from `Tokens` and `Type` only, exported from `kit/src/lib.rs`. A
control another track could need (a menu, a dialog, a sheet, tabs, a
tooltip, a virtual list) is the lead's to write: ask through the
coordinator first, so two tracks do not write two.

A settings screen (About now, the settings track's sections later) is
built from `kit::settings`: Paseo's section with its small title, the
card, a row with its title and hint at the left and its control at the
right, and Paseo's switch; buttons are `kit::button` (accent) and
`kit::outline_button` (Paseo's outline).


The kit's `qr.rs` draws a supplied module matrix on a white tile in both
themes, with a four-module quiet zone. `view::devices::qr` encodes with
`qrcode` 0.14 (MIT OR Apache-2.0, default features off); neither layer
writes a matrix or a link. QR and text share the invitation reveal gate.

`kit::button_with_kind` is the shared entry: typed primary, secondary,
destructive and ghost kinds plus enabled/disabled state. Destructive fill
and border use `neutral.destructive`, text its separate foreground. Ghost
has transparent fill/border and muted text, turning foreground on hover;
filled kinds set no hover override. Pressed is 85% and disabled 50%.
Disabled callers attach no handler. The existing bool `button` delegates
and stays until the lead retires it after moving its callers.

`kit::breakpoint::TOUCH_TARGET` is the shared minimum touch target;
screens take gaps and insets from `kit::theme::spacing`.

`kit::assets::Glyph::CalendarClock` is the shared scheduling mark: an
original calendar and clock on the kit's 24-unit, 1.5-stroke line grid.
Navigation and empty states use this one registered vector.

`PanelLeft` is the desktop sidebar mark. `MenuCompact` is the compact
sidebar's original 16×12 vector: two full bars and one half bar, each 1.5
high. Its filled round rectangles keep the painted footprint exact; legacy
`Menu` and `PanelRight` are unchanged.
`Upload` and `Download` are the shared transfer arrows/trays for push/pull,
with the ordinary line grid at action 16/menu 14; legacy ArrowUp/Update stay.


Shared action glyphs also include `Trash2`, `ArchiveRestore`, `Minus`,
`GitBranchPlus`, `Commit` and `Undo2`; existing `Pencil`/`Archive` stay.
Each is registered once in `assets.rs` on the same 24-unit line grid.
`outline_button_with_glyph` puts one before an outline button's words
(Paseo's `leftIcon`: 14 square, the button's 8 gap, glyph and words in one
tone), as Settings › Overview's Remove draws its Trash2.

`kit/badge.rs` supplies Paseo's StatusBadge for every area: typed success,
warning, error and muted variants; small and extra-small sizes; optional
leading content; semantic status tokens separate from board lane colours.
Labels remain whole and explanations belong below their row.

`kit/split_button.rs` supplies Paseo's joined primary action and caret:
26-point pointer / 44-point touch geometry, shared busy spinner, disabled
primary, and `menu_right` for the menu aligned to the caret's right edge.
The caller owns its actions, labels and menu state.

Picker rows retain `Leading::Glyph` at 16 points; `Leading::SmallGlyph`
is the explicit 14-point slot used by Paseo’s branch switcher.

`kit/toolbar_label.rs` supplies Paseo’s compact current/base branch trigger:
12-point muted label and caret, highlighted fill, typed open/disabled state,
and explicit single-line or wrapping text. Composer pills keep their shape.

Provider error detail follows Paseo's fixed palette red[300] through
`Tokens::status().red_300`; its semantic status dot still uses the ordinary
status error signal. Areas use the shared token rather than a local colour.

`kit/alert.rs` supplies Paseo's five notice variants and four density sizes,
with optional title, wrapping description and an indented action. It uses
existing semantic/neutral colours and palette blue 300 / amber 500 in
`theme/status.rs`; a notice has no focus or click behaviour. Pure logic
returns `view::alert::Variant`, mapped once by `app::badge::alert_variant`.
Its glyphs are the kit's own embedded drawings, as in `assets.rs`.



`kit/dialog.rs` accepts a clonable close closure so Escape, the scrim and
the close glyph retain the caller's decision identity. Function-item callers
keep their existing behavior; stale-decision policy stays with the feature.

`kit/focus.rs` provides one weak, once-per-open return policy for controls.
`DialogState::dismissed` acknowledges an actual close after the feature's
pending/stale guards; it preserves a newer focus and clears the old cycle.
Unadopted unit-close callers keep their prior behavior until they call it.

`kit/dialog.rs` keeps a fitting dialog at its natural size. When its
content exceeds the card's existing height limit, only the body scrolls;
the heading and footer actions retain their size and remain reachable.

`kit/command.rs` supplies Paseo's Command Center pieces. The canonical
`command_row_parts` renderer accepts semantic prose and code-path captions
with separate UTF-8 marks; the existing plain-row APIs delegate to it.
The kit validates ranges and merges overlaps; matching/ranking stay in
`view`. Paths use the existing code face and code-size preference at 12/16,
while their prose prefixes use the interface caption metrics.

A host opts into persistent `CommandState` and `command_frame_animated`
for the desktop entering fade (the motion tokens' surface length and
curve) and the shared `sheet::snap`
60/90% stops, pan-down dismissal and content-scroll handoff. The static
entry point and existing edge-sheet API remain available. The desktop
panel has no exit animation: Paseo's Command Center unmounts its Modal
at close (`command-center.tsx`, `if (!state.open) return null`), so only
the entering curve runs. The compact sheet closes with its spring, and
its backdrop keeps input until it is gone, as `sheet-backdrop.web.tsx`
keeps its target until the portal dismisses. `CommandState::is_open` is
the logical state (input and focus); the host presents and samples
state, restores prior focus once at close, and stops drawing only when
`CommandMotion::hidden` becomes true. A zero-height window waits for a
resize instead of requesting frames. Direct dragging remains available
with the App's reduced-motion flag; springs and fades finish at once.
The App supplies this flag. On Linux/FreeBSD its portal follower reads and
subscribes to the system preference; `VESSEL_APP_REDUCE_MOTION=1` or `0`
forces the flag for one execution and skips that follower. Nothing is
saved. Actual OS startup/live-change behavior remains unverified here;
Windows/macOS system following is still absent.

GPUI provides pointer coordinates and native capture, but no recognizer
velocity: a bounded 100 ms/eight-sample history supplies release velocity.
The shared spring follows the installed Reanimated helper (including its
64 ms delayed-frame cap); its numeric fixtures were sampled from that
helper.

`kit::settings::row_titled` is `row_at` with the caller's title element
(a title with Paseo's `SettingsInfoTip` beside it); `row_at` builds its
text title and delegates to the same private body.

`kit::settings::section_with_trailing` is the shared section head with an
optional trailing action (Paseo’s `SettingsSection`); `section` delegates
with no action. Areas supply the action’s state and tooltip, rather than
copying the heading/card gap into another header. `row_with_content` is
its row frame for custom contents (such as a provider’s mark and inline
status); ordinary `row_at` delegates to the same padding, border and
compact wrapping frame.

`kit::skeleton::pulse` owns one mounted animation phase. Its draw closure
receives one `Opacity` to pass to every `shape`; only the neutral surface2
shapes fade, leaving cards and borders solid. Callers supply typed shape
sizes/corner steps and compose existing settings/card frames. The pulse
stops scheduling frames when unmounted; reduced motion follows GPUI.

`kit::popup::appear` is how a floating surface arrives (the owner's word
39): from clear to solid and four points below its place, over a length
from `kit::motion` on its curve; menus, a combobox's list and tooltips
use the quick length (Paseo's menu enters in 150 ms), dialogs, sheets and
toasts the surface length. Nothing plays on the way out: Paseo's desktop
popovers, menus and dialogs unmount at once. `popup::fill_in` fills the
row a list's keyboard position arrives on over the quick length. Held
motion (Animations off, reduced motion, a software renderer) draws every
end state at once (GPUI's `with_animation`).

`TextField::form_multiline` uses the same input state and form shell as
the kit's existing fields: Enter starts a line, Mod+Enter submits, and
after twelve lines the editor scrolls. Its marked-text regression also
exercises the composer; platform input-method support remains shared.

`kit::field::composer_hooks` gives a composer's screen what Paseo's
composer needs of its text: `caret`, `selection`, `replace_range` and
`replace_all` (byte offsets; one undoable edit, the caret after it),
`on_edit` (after every edit), and `hooked`, which offers Up, Down, Enter
(alone), Tab and Escape, and a paste of images or files, to the screen
before the text sees them. `KeyAnswer::Pass` leaves everything as before
(Enter sends, Tab indents); `Taken` stops the key. An input method
composing is never asked. Screens take a key only while their menu shows.

The permissions modes' marks are Paseo's shield family on the kit's own
`Shield` outline: `ShieldPlus`, `ShieldCheck`, `ShieldOff`, `ShieldEllipsis`
and `ShieldAlert` (`assets.rs`, the 24-unit 1.5-stroke grid). A list row
draws one in the foreground with `Leading::Mark` (Paseo's mode rows);
`Leading::Glyph` stays muted. `cargo run -p vessel-app-kit --example
shield_marks` with `GPUI_X11_SCALE_FACTOR` draws them beside `Shield` at
14, 16 and 20 points, off the pixel grid, for the scale check.
A list in which any row leads with a mark keeps the mark's column on every
row (an empty one where a row has none), so labels share one left edge;
agent lists, whose marks are the agents' own widths, are left as they are.
`ComboHeader::note` is a muted line above a list's rows: the composer's
lists say there that the list shown could not be refreshed.

What each agent on each machine offers (its models, efforts, modes, on/off
options) is one store, `view::offers`: shown at once when a control opens,
refreshed behind it (on open, at connect, when a machine's agents change;
one request per agent in flight), laid over an open list at the edge away
from its anchor so the rows shown stay put (the list's keyboard highlight
follows its row). The link keeps the last lists on the device between runs
(`link/src/offers.rs`: `agent-options.json` in the app's state folder,
owner-only, versioned, bounded, keyed by the node's id) and hands them over
when a machine connects. `docs/plans/backend-m12-composer.md` has the
rules and what was followed from Paseo.

A composer's text that was not sent is its draft (per session, and the
new chat's): put back when the composer comes back and kept on the device
(`link/src/drafts.rs`: `drafts.json` beside the agents' lists, owner-only,
versioned, bounded; a file that does not read is set aside as
`drafts.json.unread`). Up and Down in an empty session composer recall
the messages sent in it, newest first.

`kit::attachment` is Paseo's `AttachmentPill` and `AttachmentFrame`: a
file attached to a message as a 48-high pill (an image's picture, any other
file's name over its kind), a remove button at its top left in a composer,
none in a sent message, and Vessel's own progress bar and trouble edge for
a file on its way to the node.

`kit::autocomplete` is Paseo's `Autocomplete` (the composer's `/` and `@`
list): rows drawn best last (nearest the text), the chosen one on
`surface2`, a chosen command's card above the list, the loading, empty and
failed states as one muted line; at most 220 high, scrolling, the chosen
row kept in view. `cargo run -p vessel-app-kit --example autocomplete`
with `VESSEL_AUTOCOMPLETE_SCENE` (`commands`, `long`, `files`, `empty`,
`loading`, `failed`, `scroll`) and `VESSEL_APP_THEME` draws each state.
The list takes a `Motion` (an easing and a length) from the screen, which
reads the app's one set of motion tokens; the kit has none of its own.

`Tooltips::on_code` keeps the tooltip’s existing chrome and placement but
uses the code face for a whole path or hash. Existing `on` stays body text.

`kit::form_summary` supplies GOAT’s branch/files summary and fixed file-list
inset. Static outer edges keep the content geometry and never take focus.

`kit/toast.rs` retains its legacy toast entry and adds `toast_with` for typed
content/optional hover wiring. Info uses neutral chrome and foreground Info16,
with no close control. `view::toasts` owns its keyed monotonic3200ms lifetime,
pausing and resuming the remainder; old callbacks cannot clear a replacement.
Legacy Ok4s/Error-persistent cards keep their stack and controls. Only older
Info is replaced, an explicit compatibility difference from Paseo's single card.

`kit::screen_header::frame` is the shared screen/back-header renderer:
`Content { title, leading, actions }` and typed desktop/compact density.
The ordinary header and compact callback entry points delegate to it.
Navigation controls/actions are already wired by the caller; the kit owns
no route, sidebar state, titlebar or window drag. Desktop title is 14/300;
compact is one-line 16/500 with a 44-point menu button. Chrome integration
and avoiding duplicate sidebar toggles stay with the desktop lead.

The settings page accepts one optional `page::DetailHeader` (title and
caller-owned back callback). Ordinary page calls pass None; detail headers
use the same shared `kit::screen_header` frame as other areas. The existing
`compact_header` wrapper supplies its navigation callback to that renderer,
so ordinary and detail navigation keep one compact look.

Diagnostics uses `Glyph::Stethoscope`, one original24-grid/1.5-stroke vector,
consumed at16 for pointer and20 for compact settings navigation.
Settings' list also draws `Glyph::FolderGit` (Projects, Workspaces) and
`Glyph::Boxes` (Providers): original drawings on the same grid after
Paseo's `FolderGit2` and `Boxes`, at the same two sizes.

The source-specific `host_rename_button(id, ButtonState, tokens)` keeps
Paseo's pencil: `theme::icon::SM`, padding `spacing::S1`, `theme::radius::MD`,
static muted ink, a frame and a hit (slop `spacing::S2`) computed from those;
its caller owns the callback and pending state, with no extra paint. Unlike
Paseo's it has no accessible name (the kit has no accessibility tree yet).
Enabled actions consume the left-button release after the generated caller
click, without a capture handler. Later transparent sibling overlap remains
an open kit hit-test policy; this is parent isolation, not global occlusion.

### Theme cycling

Settings > Shortcuts > General lists **Cycle theme**. Its default is
Command+Option+T on Mac and Control+Alt+T elsewhere, through the same
registry used for rebinding and conflicts. It calls `Theme::next` and the
existing appearance preference setter. The command center suppresses this
action, including a rebound key; character-preferred input and focused
composition stand down before shortcut matching. Non-Mac terminal focus is
excluded by policy, but its actual focus query awaits the terminal surface;
this integration still has a placeholder there.

### Named colour palettes (ADR 0070, staged)

The kit can build `theme::Palette::{Light, Dark, Zinc, Midnight, Claude,
Ghostty, PureBlack}` through `Tokens::palette`. `Scheme` still means only
light or dark: status and syntax retain their existing scheme-specific
sets. `Tokens::of(Scheme)` and both shipped token constructor bodies are
unchanged. The new dark variants share one tint-to-token mapping; their
authored values and deliberate brand-accent substitution are in ADR 0070.

The existing stored `view::prefs::Theme` now includes all seven concrete
palettes and System. Appearance keeps its existing tiles, with System
first. One app adapter supplies both their previews and the applied tokens;
it compares actual tokens so two dark palettes do not collapse to Dark.
`set_palette` updates the component layer and refreshes the windows too.
The run override still pins light/dark as before. The plan and current
verification limits are in `docs/plans/app-themes.md` at the repository root.

`theme::palette::GREEN_500` remains the fixed success colour used by the
typed menu. It has one definition, shared with the named palette module;
it is not replaced by the theme's accent or semantic status signal.

### Rules every screen follows

`kit::theme::spacing` contains Paseo's complete spacing scale, with its
keys in the constant names (`S0_5`, `S1`, …) and values in logical points.
Its exact-value test keeps every step comparable with `styles/theme.ts`.

- Words through `view/locales/` (or an active area's remaining `view/src/text/` table); English and Korean together.
- Colours, type, spacing and radii from the kit's tokens, never a literal
  in a screen. Pretendard is behind every face (the kit sets it); no
  generic family anywhere. `node scripts/check-tokens.mjs` holds the part a
  tool can see: a `px(<number>)` in `app/src` (one equal to a spacing step
  is written `px(spacing::S…)`), and a hex colour anywhere outside the
  kit's tokens (`kit/src/theme.rs`, `kit/src/theme/`). What was there on
  2026-11-14 is listed per file in `scripts/tokens-allow.txt` and may only
  fall (`px-step` 443, `px-other` 90, `hex` 44).
- An error belongs beside the control it is about; the frame's foot is
  for what belongs to no control.
- Every screen works from 360 points wide (the phone); targets are 44
  points on touch; nothing depends on hover (ADR 0036).
- Two widths, both named in the kit, each with its reason:
  - `kit::breakpoint::PHONE` (768): below it the sidebar is a sheet from
    the title bar's menu button, and the title bar's statuses are icons.
    ADR 0036's width, the page's too, so the sidebar folds at the same
    size on every surface.
  - `kit::picker::COMPACT` (600, Paseo's compact width,
    `constants/layout.ts`): below it controls are touch-sized (44), a
    picker's list and a dialog are sheets from the bottom, and the
    composer's card is tighter. Paseo's own width, so its controls change
    where Paseo's do.
  Between 600 and 768 the sidebar is a sheet and the controls are still
  pointer-sized: a narrow desktop window, where a mouse is still likely.
  `kit::breakpoint::INSPECTOR` (1280) folds the inspector, when there is
  one.
- No library's default look anywhere (the owner, on 0.0.0.0.3): fields
  are the kit's (`TextField`: a form field, or the composer's bare text
  in `composer_card`), pickers the kit's (`pill` and `combobox`),
  buttons the kit's; carets and marks are the kit's vector glyphs
  (`assets::Glyph`), never characters. Controls take their colours from
  `tokens.neutral` (Paseo's scale); the accent is the brand's.
- A struct pattern over a protocol type (`vessel_proto`) ends with `..`,
  so a field a newer node adds does not break the app (old and new peers
  are both accepted). The exception is a type that carries what a person
  approves or agrees to (`ProjectCommands::Listed` in `Listed::of` and
  the worktree's `NeedsApproval` that carries it; the permission card's
  `PermissionRequest` and each `PermissionOption`; an `InstallOffer`
  where it is shown and run): its pattern names every field on purpose,
  with a comment saying so, so that a new field breaks the build until
  the screen shows it or refuses it by type. Ignoring it would let a
  person approve something the screen never showed.
- The sidebar and the conversation's timeline are drawn as regions
  (`app/src/region.rs`): GPUI views of their own, cached between frames,
  drawn again whenever the shell changes. A field's caret blink (or any
  view's own notify) outside a region then draws the window without it.
  What a region shows must come from the shell or from views drawn
  inside it: a model entity read there and notified on its own would
  leave the region showing old content. Measured on 2026-11-14 (release,
  composer focused on an open session, the caret blinking): the app's
  own work per blink fell from 7.1 to 2.1 ms (the draw probe below).
  A region is drawn again when its size changes, but layout comes after
  drawing: a decision taken from a size measured at the last layout (a
  list's viewport width) is a frame late, and with nothing else marking
  the region it stays late. Such a decision checks itself with a canvas
  that measures this frame and marks the region dirty only when the
  answer changes (`conversation::outline::width_probe`, the chat outline
  rail that stayed at 800 x 600 in 0.0.0.0.19).

### The brand's drawing (ADR 0059)

`view::brand` decides the mark's large multiline drawing without a toolkit:
the measured ratios (`ratios`), offset lines with concentric turns, loops
and the star (`geometry`), where each screen's drawing goes and how it
steps down as the window narrows (`layout`), the first message's journey
(`motion`: 300 ms move, 150 ms star to circle, 150 ms the agent's mark, the
kit's motion tokens and curve), and the shapes the kit paints. Its
tests place the drawing over the real screens' measured boxes (1280, 1920
and 390 wide) and check it never comes within 12 points of text.

`kit::vector` paints shapes it is given (points with a stroke or a fill,
a share of one theme ink) as the agent marks are painted: rasterised at
exactly the device pixels they cover and drawn 1:1, cached by content
(twelve rasters, at most two of them panel-sized; one pushed out is
dropped from GPUI's atlas). `vector::drawing` makes its shapes at paint
time, so a backdrop can use boxes the content measured in the same frame,
and rasterises only the area it fills (the open session's frame runs far
below the screen); `vector::inline` draws in a box of its own (the
avatar).

`app::brand` is the shell's `brand`: the new session's column gives
`on_children_prepainted(shell.brand.new_session(skip))` (the heading, the
composer's card and the pickers after `skip` children; any later child,
such as the "no machine yet" note, counts with the pickers), the session's
screen gives `shell.brand.session_body(viewport)` and its composer's column
`shell.brand.session_column()`, the row waiting for the reply gives
`shell.brand.reply_head()` (its first child is the avatar's slot, where the
star lands), `shell.brand.sent()` starts the journey, and `brand::avatar(…)`
draws an agent message's avatar. The
journey steps once per presented frame while it runs; held motion (reduced,
off, a software renderer) shows its end at once.

`kit::window_glyph` draws the title bar's minimise, maximise, restore and
close glyphs in the mark's line language (one line, round ends, corners on
half a pitch, the back stroke broken one gap short of the front one) at 9
points, in whole device pixels at every scale: the line `⌊scale + ¼⌋`
pixels (1, 1, 1, 2), the gap at least two, straight strokes on whole rows
through `vector::snapped`. The cells, their fills and their hit areas stay
the chrome's. `kit::frame::panel_edges` draws the content panel's edge as
two lines one pitch apart (one at a phone's width) with GPUI quads whose
boxes and widths are whole device pixels; it sits in a parent that does not
clip and replaces the panel's own border. `kit/examples/window_glyphs.rs`
is the audit sheet: every glyph in the bar's four states and the two-line
edge, captured at 1, 1.25, 1.5 and 2 (`GPUI_X11_SCALE_FACTOR`).

## Why GPUI, and which

ADR 0035: the app is native, drawn as Zed draws its own; Zed's framework
is a reference and is not taken as it is. The kit is ours so the layer
under it can change. Underneath for now: the `gpui-pre` snapshot 0.3.10
(Zed's tree, republished weekly), with `gpui-component` 0.8.1, the newest
pair the community mobile layer also pins. Every problem met with it is
in `docs/plans/backend-m12-spike.md` (question 9). The toolchain is Rust
1.98.1 (`rust-toolchain.toml`), not Vessel's 1.93: the snapshot uses
`std::hint::cold_path`, stable since 1.95.

## Build, run, gate

```bash
cd spikes/native
(cd ../../web && npm ci)          # Pretendard comes from the page's packages
scripts/fonts.sh                  # the faces into assets/fonts (not committed)
cargo run -p vessel-app
```

When the kit's list of faces grows, run `scripts/fonts.sh` again in every
worktree: the build embeds each listed file and fails on a missing one
(2026-11-15: Geist Mono's bold, italic and bold italic, for the
terminal's SGR 1 and 3; CI runs the script on every gate).

Terminal history reads carry a process-local opening identity through the
link, including refused commands. Reopening the same machine/session cannot
take an earlier read or replay completion. Live bytes also replay on the
background executor, so a burst of small records never resets a UI parsing
budget. The retained frame includes appearance, measured metrics and viewport;
a quiet terminal follows theme/code-size changes even with motion held.
Wheel and recognized touch pans expose view's bounded history. Cursor work
stops on blur, navigation, hidden presentation and held motion.

Linux's ignored `terminal_states_capture` test draws the real shell against
a disposable fleet with `VESSEL_CONFIG_DIR`, `VESSEL_TEST_TERMINAL_SESSION`
and `VESSEL_TEST_TERMINAL_STATE=ready|loading|unread`. Ready reads normally;
Loading holds a receipt and Unread passes a typed failure while the real
follow continues. These two fixtures prove display states, not transport
failure. They cannot be enabled in an installed build. Use a private display
and the fake-fleet procedure below, and inspect each resulting image.

A `.cargo/config.toml` beside this file (untracked, one per worktree)
sends build output to a directory of your own on the data disk with six
jobs; one build at a time.

Linux needs the X11 and Wayland development libraries GPUI links
(`libxkbcommon-dev`, `libwayland-dev`, `libxcb*-dev`, `libfontconfig-dev`)
and, to run, a Vulkan driver (Mesa's is enough).

The gate of this workspace (the main workspace's lints are copied into
its manifest), run from `spikes/native` with its pinned toolchain
(`rust-toolchain.toml`), the whole workspace, never a `-p` subset; every
track runs exactly this before it reports a slice:

```bash
cd spikes/native
cargo fmt --all --check
node ../../scripts/check-rust-size.mjs --workspace .     # file budget
node ../../scripts/check-boundaries.mjs --workspace .    # this README's crate tables
node scripts/check-tokens.mjs                            # tokens, not literals
cargo clippy --all-targets -- -D warnings
cargo test
```

The three `node` checks build nothing and take about a second, so a
violation fails before the compile. The file budget is the main
workspace's (`docs/QUALITY.md`, M3): a file of product code over 500 lines
fails unless `scripts/rust-size-allow.txt` lists it, and a listed file may
not grow past its recorded size (22 files on 2026-11-14). The release
workspace (`spikes/release`, whose crates the app links) has the same two
checks in its own gate (`spikes/release/README.md`).

Environment:

- `VESSEL_CONFIG_DIR`: another configuration directory (its
  `vessel/nodes.toml` is the book; `vessel/app-prefs.json` the app's
  remembered choices), as the CLI reads it.
- `VESSEL_APP_LANG=ko|en`: the language (default: the system's).
- `VESSEL_APP_THEME=light|dark`: the scheme (default: the system's).
- `VESSEL_APP_SIZE=1280x800`: the window's size.
- `VESSEL_APP_TITLEBAR=system`: the system's title bar and frame instead
  of the app's own (the way out if the app's bar misbehaves on a
  platform).
- `VESSEL_APP_NO_UPDATE` (any value): no updates for this run.
- `VESSEL_APP_CONVERSATION=full`: conversations as the "Full" display
  setting shows them (every tool call a row of its own; an agent's log
  as one folded row) instead of collapsing tool calls, until the app
  reads that setting.
- `VESSEL_APP_SLOW_MOTION=<n>`: the brand's first-message journey runs
  n times slower, so a capture on a virtual display can take its frames
  (with `VESSEL_APP_REDUCE_MOTION=0`). Nothing is stored.
- `VESSEL_APP_SCROLL_BENCH=<entries>`: once the session shown holds that
  many entries, it scrolls itself for 600 frames and logs the median,
  p95 and longest frame.
- `VESSEL_APP_FRAME_LOG=<file>`: a line per frame a moving control draws
  and, for every window frame, `draw <time> <took>`: the microseconds
  from the root's render to the end of its paint, the app's own CPU per
  frame before the renderer rasterises (on a software renderer the
  process's CPU is mostly rasterising and grows with the pixels).
- `VESSEL_BUILD_LABEL` (at build time): the label in the title bar and
  the sidebar's foot, `dev` without it. With `VESSEL_UPDATE_FEED` and
  `VESSEL_UPDATE_KEY` too (track R's `build.sh` sets them), the build
  updates itself; without them it never does.

## Composed sections in one bounded viewport

`kit::ScrollViewport` is the shared scrolling body for already rendered
sections. Keep its `ScrollState` across renders and add sections in order;
they retain natural heights while the viewport fills only available flex
space with a zero minimum height. Keep fixed header/footer siblings outside
it. `SectionIndex` queries measured section bounds before the body's own
scroll offset; `bounds` and `offset` report the viewport and current position.
The state also supplies a minimal focus/reveal wrapper for an existing focus
target. It provides no button activation or complete row accessibility.
Wheel handling and clamping remain GPUI's, with no new scrolling physics.
`VirtualList` remains for indexed, virtualized row delegates. Existing dialog
and autocomplete internals are not migrated by this control-only change.

## Trying it against fake nodes, and capturing a screen

```bash
# in the repository root: the node, the CLI and the fakes
cargo build -p vessel-cli -p vessel-node -p vessel-testkit --bins
# here: two nodes with fake agents and a book naming them
scripts/fleet.sh start /tmp/fleet <the main workspace's target>/debug
(umask 077; VESSEL_CONFIG_DIR=/tmp/fleet/config cargo run -p vessel-app)
/tmp/fleet/pair.sh                # a pairing link for `laptop`
scripts/fleet.sh stop /tmp/fleet
```

The fake agent answers `plain` with an echo, `entries <n>` with `n`
entries, `wait` by waiting, `shapes` with a real conversation's shapes;
any other prompt runs a tool that asks for permission. The fleet's
Claude Code adapter offers what the real one sends
(`FAKE_ACP_OPTIONS=real`, `crates/vessel-testkit/src/bin/fake_acp/real.rs`):
sixteen models, subagents (`ocx-*`) with descriptions of several lines,
names longer than a list and Korean ones, an on/off option with no name.
Look at every picker with it before a build; `kit::picker`'s test lays
out rows of those shapes and fails if one grows past a line or runs into
the next.

Settings also has checked shapes (`scripts/settings-fixtures.py`): many
paired devices, long and Korean names, a never-seen and a revoked device;
a home bundle with instructions, skills and MCP names; a synthetic member
behind version zero with kept edits. These fixture reports do not run a
real member sync loop. The second machine has no paired devices. Sync
shapes include a current long-name member, a Korean unreported member, an executable skill and
present/missing/local-only secret names. `settings-fixtures.py empty ROOT`
makes the disposable second node an instructions-only home (no servers);
`settings-fixtures.py files-empty ROOT` makes it a server-only home (no files).
Both provide empty member and secret lists. The server-only shape names an
empty skills directory; a home naming neither an instructions file nor a
skills directory is refused. That shape exercises an unavailable publication;
`standalone ROOT` returns it to the default no-home shape.

Normal-build captures always mask the invitation. Revealed-state model
checks use inert, test-only invitations in `view/src/tests/devices.rs`;
no app feature can substitute a fake invitation in an installable build.

Screens are checked against the two `shapes` sessions the fleet makes on
`laptop`: a long thought streamed a few characters at a time (Korean
first, a syllable alone in a chunk), Markdown with a table, a 240-character code
line and Korean, a long path, a wide diff, a command on one long line
asking for permission with four answers, and 2,400 lines of output.
`shapes` has its permission answered (the output in the timeline),
`shapes-asking` is left asking. Every Claude Code session also opens
with the start-up log the real adapter writes before the first prompt
(ten `[session/create] …` lines, `FAKE_ACP_STARTUP_LOG`). `fleet.sh start`
fails if either `shapes` session lacks any of it (it reads them back
with `vessel log`), so a fake-acp built before these existed is caught:
rebuild it. A third session on `laptop`, `turns`, holds five turns (one
long prompt, one Korean; each tool's permission allowed): the outline of
prompts and read marks are checked against it. A fourth, `queued`,
runs a turn that waits until stopped (`wait`, which takes a steer) with
three prompts queued behind it (one long, one Korean, one short),
checked through `vessel session queue`: the queue above the composer is
checked against it. Two more, `ask questions` and `ask one`, are left
waiting on what Claude's adapter sends for its `AskUserQuestion` (three
questions, one choosing several and one in Korean, and one question):
the question card is checked against them. Three more, `permit plan`,
`permit codex` and `permit two`, are left waiting on permission requests
in the real adapters' shapes (Claude Code leaving plan mode with five
"always" answers, Codex's five answers, two at once): the permission card
is checked against them and the `shapes-asking` session's long command.

`scripts/shell-fixtures.py` adds eight long English/Korean session titles
and long working paths through the real CLI, pins two and archives two.
It reads the disposable node's durable projection to fail startup when
these shapes are missing. It refuses a book outside disposable loopback
endpoints or an adapter that is not the real-shape fake. This tests shell
layout against node data rather than hand-built screen rows.

The fleet also checks Agents and Models shapes (`scripts/agent-fixtures.sh`):
an added agent with a long name and argument, a definition whose program
is missing, Codex signed out, a failed status check, agent-supplied sign-in
steps, a missing Codex adapter on `build-server`, and a long model route.
Each node has its own fake program directory. Its `npm`, `npx` and `uvx`
never download or execute packages. To hold an app-started install for a
capture, create a FIFO at `<fleet>/<machine>-home/install-gate` before
pressing Install. Write `fail` to it to release exit 7, or `success` for
success. The fake installer prints its output but changes no installation.
The fleet uses private scratch directories (`umask 077`); run it as the
same development account that runs the app. Opening the registry allows
the node’s ordinary public catalogue fetch; all other fixture traffic is
local. The current SDK cannot exercise older/newer-agent readiness over
the wire; see `docs/plans/backend-m12-agents.md` for the source boundary.

On a machine without a display, use an Xvfb of your own on a free display
number (`ls /tmp/.X11-unix`), a fresh private authority file, Mesa's software
Vulkan, and ImageMagick's `import`. Run the app and fixture CLI as the same
development account, with the **same HOME and config root**: stored device
credential paths are relative to that home. A different app HOME can make a
correctly paired fixture appear unreachable.

```bash
umask 077
fleet_root=/tmp/work/vessel-fleet
capture_root=$(mktemp -d /tmp/work/vessel-capture.XXXXXX)
export DISPLAY=:140 XAUTHORITY="$capture_root/authority" # choose a free display
python3 - "$XAUTHORITY" "$DISPLAY" <<'PYAUTH'
import pathlib, secrets, subprocess, sys
path = pathlib.Path(sys.argv[1])
path.touch(mode=0o600)
subprocess.run(["xauth", "-f", str(path)],
               input=f"add {sys.argv[2]} . {secrets.token_hex(16)}\n",
               text=True, check=True, stdout=subprocess.DEVNULL)
PYAUTH
Xvfb "$DISPLAY" -screen 0 2880x1800x24 -auth "$XAUTHORITY" -nolisten tcp &
xvfb_pid=$!
HOME="$fleet_root" VESSEL_CONFIG_DIR="$fleet_root/config" \
  VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
  VESSEL_APP_THEME=light VESSEL_APP_SIZE=390x844 <target>/debug/vessel-app &
app_pid=$!
# Wait for the intended screen to appear before taking its capture.
# Drive the normal app with xdotool; use --window coordinates after mapping.
import -window <app-window-id> "$capture_root/shot.png"
```

Record the app source/binary hashes, PIDs, viewport, theme, language and UI
scale beside captures. Inspect every image: a mapped window or a successful
capture command does not prove navigation reached the intended screen.
For extra glyph scales set `GPUI_X11_SCALE_FACTOR` to 1.25, 1.5 or 2 at app
startup; the physical window grows with that scale. Before stopping an owned
process by PID, verify its executable under `/proc/<pid>/exe`. Stop your app,
fake fleet and display when finished; keep the authority cookie private.

## Translation files (ADR 0039)

`view::localization` sits beside the legacy tables. Its host build script
reads `view/locales/<tag>/<area>.ftl`, validates the Fluent AST, and generates
a typed API from English. The target library embeds the resources and does
no I/O. Prepare `Translator::get(locale)` before drawing: plain accessors
borrow retained fields, while messages with values are formatted when those
values change. The allocation-counting test covers warmed plain access.

`LANGUAGES` exposes each embedded tag, own name and completeness; the
checking locale `en-XA` is excluded. `resolve_languages` applies the checking
override, a typed preference, or the host provider's preferred system tags in
order. Each tag tries exact then base; English is the final fallback after the
list is exhausted. `resolve_language` remains the single-tag adapter. The app
uses sys-locale's existing native/browser list at startup or preference events,
with no locale I/O during drawing. English/Korean are the shipped languages;
a partial candidate stays outside the shipped directory. The source API is
English and Korean must be complete with the same variables. Run the shared
validator/report:

```bash
scripts/locale-coverage.sh
```

`view/locales/<tag>/formats.ftl` owns number separators, primary/secondary
sizes, ten digit characters, month/weekday names, whole date/time patterns
and the h12/h24 choice. Missing contributed settings/patterns use English.
The build validates settings and emits static numeric rules automatically;
adding a folder needs no separate generator command or Rust edit. Messages
call standard `NUMBER`/`DATETIME` names. NUMBER remains numeric for plural
selectors; DATETIME accepts typed Gregorian dates/local clock readings and
a caller-resolved zone label. This small implementation is not complete
Intl: no other calendars, time-zone conversion, currency/percent formatting,
significant-digit rules or automatic numbering-system discovery. Typed
decimals are finite, within the u32 count magnitude, and at most nine decimal
places. Patterns currently receive minute precision. Dynamic formatting is
performed when data changes, before painting. ICU4X is not a dependency of
this formatting layer; ADR 0039 retains its rejected measured cost.

Pairing is the first complete migrated area: its two dialogs, name/link
errors and successful status use `view/locales/{en,ko}/pairing.ftl`. Other
owners keep their area tables until their own migration. The shared machine
file-fault sentence still belongs to its table, including when pairing shows it.
About's licence and bundled-software labels use `about.ftl`; inventory roles
map exhaustively to cached plain getters in `view/src/about/words.rs`.
The Fluent libraries have the Translations role; UUID has the Identifiers role.
About's inventory test covers normal external declarations in both the shared
manifest and each native member, including target-only and inline declarations.
Member-local names not in the shared manifest follow its entries alphabetically;
fonts follow crates. Build/development-only dependencies are not inventory rows.
The connected-machine section and trademark section still belong
to their existing areas and have not moved with the licence vocabulary.
Title-bar/update-card labels use `update.ftl`. `view::updates::PreparedUpdate`
formats their value messages once per updater state or language event; the
host retains the runtime and typed state together in `app/src/update_words.rs`.
Renderers share the prepared text without formatting a version or progress
sentence. Developer build names remain data; numbered release prefixes and
whole status sentences live in the files. Typed updater errors select complete
recovery messages; their diagnostics stay in the state/logs. A failed restart's
owned frame notice also rewords on a language change without replacing a later
notice from another area. No update error's Display becomes UI text.
Rewind's menu and copy vocabulary use `rewind.ftl`; typed node error codes
choose safe whole recovery sentences. Its retained notice is prepared at
answer/language events. Conversation paints receive the shell's already-prepared
catalog beside their remaining legacy words; rewind diagnostics remain in logs.
Fork menus, empty-fork labels and source captions use `fork.ftl`. Source titles
are prepared at fleet/act or language events and evicted when no longer listed;
rendering borrows these cached captions. Failure/origin notices are retained at
the answer boundary for live retranslation. Typed fork/origin-read failures and
unknown empty-fork reasons choose whole recovery sentences; original diagnostics
remain in logs/session state. An uncertain fork asks to check the session list
before retrying.
Settings Chat uses `chat.ftl` for its section title, display row and choices;
the typed adapter borrows prepared labels. A live language change rewords the
existing picker while preserving its selected preference. Content-width labels
remain in their Appearance area until that area moves.

Settings Diagnostics controls use `diagnostics.ftl`. Its copied notice rewords
on a language change while the collected report and clipboard data stay fixed.
The technical report keeps its stable English schema and existing redaction,
as in the reference; collecting/refreshing remains an explicit action.
General's Language row uses System first, then each embedded language's own
name. Its typed preference goes through `Prefs::prefer`; labels, retained
field placeholders and pairing outcomes update at once, without restarting
or clearing typed input. `VESSEL_APP_LANG` wins over that preference.

For a normal app run against your private fake fleet, use
`VESSEL_APP_LANG=en-XA` to check expanded pseudo-language strings; the
pseudo-language is never offered as a saved choice. Unset that override
when checking the General row's live language change.

Add a string to `en/<area>.ftl` and `ko/<area>.ftl`, with context comments
and the same variables, then use the generated accessor in its caller.
A whole sentence is one message; use selectors instead of joining pieces,
and mark short buttons/badges in the comments. Counts are numbers, never
formatted strings. Prepare plain strings before painting and format dynamic
messages only when their data changes. See [the translator guide](view/locales/README.md)
for exact steps, supported grammar/formatting, and the area migration recipe.


### Add a language

A folder under `view/locales/<tag>/` is embedded in the released app and
listed in the chooser by its own `meta-name`. It is not a draft area.
Before adding it, the coverage report must show `0 missing` and
`complete=true` for every language being shipped, with no syntax, variable
or formatting-rule errors; the complete native gate must pass. The staged
app currently ships only English and Korean while legacy tables remain.
A report over migrated messages does not prove those legacy screens are
translated.

Prepare a candidate outside `view/locales/`, alongside copies of the
current English/Korean source folders, and inspect it without shipping it:

```bash
scripts/locale-coverage.sh --root CANDIDATE_ROOT
```

The two-message Japanese folder is solely a test fixture under
`view/tests/fixtures/locales/`. Registry, coverage and fallback tests read
that root; its partial translations never enter the released registry.
To inspect that proof, from this workspace:

```bash
scripts/locale-coverage.sh --root view/tests/fixtures/locales
```

Without `--root`, the report also reads the compiled shipped-language list.
Core validation supports partial candidates and English fallback; the
released-catalog test requires every shipped catalog to be complete.

### Browser-only wording capture

The browser-unsupported pairing sentence has a Linux display-test harness:
it feeds that typed result to the actual pairing renderer after the normal
offer reader succeeds. It proves the sentence and layout, not that a browser
transport runs. It is compiled only into tests; the installed app has no
preview option or event override. The ordinary gate reports this explicit
display test as ignored.

On your private authenticated Xvfb, create a FIFO in a private capture
directory (`mkfifo -m 600 "$capture_root/offer.pipe"`). Pipe your private
fleet's `pair.sh` stdout directly into that FIFO from a background writer;
never print the link or keep it in a regular file. Set
`VESSEL_TEST_PAIR_LINK_FILE` to the FIFO and `VESSEL_CONFIG_DIR` to an
isolated empty capture config, then run from this workspace:

```bash
VESSEL_APP_NO_UPDATE=1 VESSEL_APP_LANG=ko VESSEL_APP_THEME=dark \
  VESSEL_APP_SIZE=390x844 cargo test -- --ignored --exact \
  pairing_browser_words_capture --test-threads=1
```

Dismiss the pairing dialog with Escape, then close the window to finish the
test. Remove the FIFO and stop your verified owned writer/display/fleet
processes by their PIDs. Connection,
storage and name-error captures use the normal app and real isolated failure
paths, without this harness.

## A release

`spikes/release/build.sh` (the release track's) builds every artifact of
a label from this workspace, signs the set and replaces it in the
distribution directory at once. Nothing here writes there.

Git's screen policy models live beside its request preparation in
`view::workspace::git`: the compact Changes head, checked branch choices,
commit scope and submit gates, keyed pending intentions and file actions bound
to the code panel's shown machine/root and comparison. Retained draft facts
preserve the form after a failed refresh; only a ready read authorizes a write.
Drawing belongs to `app` through the kit, and prepared node calls belong in
`link`; Git output is never interpreted in a drawing callback.

Git's commit scope count, path preview and submit share one eligibility rule;
a partial zero remains unknown. Failed and unread facts give retry/read guidance,
and only a pending read says it is refreshing. Keyed success receipts retain the
original stash intention and protect the one-second success reset from late timers.

`Type::MonoBody` is the form path's 14/20 normal code step (GOAT's commit
summary). Existing `Mono` remains 12/20. Both use the independent code scale and
same embedded face/fallbacks; ordinary type continues to use the UI scale.

### Official agent artwork (ADR 0048)

`kit::agent_mark` is the one embedded-image control. Its opaque `Artwork`
constants carry reviewed vendor vectors, `Size` the requested logical
size, and `Space` the surface's existing room. Unlike `glyph`, it preserves
colours. A brand minimum/clear space that does not fit produces the bot
fallback. No URL or SVG from the wire reaches this control. Source hashes,
terms and permission requirements are in `docs/design/agent-marks/README.md`.

`agent_mark::GITHUB` is shared artwork for Copilot identification and
GitHub forge pages. `forge_mark(Forge::GitHub, size, tokens)` supplies the
second typed use without a second copy. An agent mark's identity still
belongs to `view`; the kit knows no node or session. The Invertocat's agent
use requires explicit GitHub Copilot wording in the same row or tooltip
(ADR 0048). Every mark, Gemini's gradient spark included, is a vector
rasterised at the device pixels it lands on (`agent_mark/raster.rs`);
none goes through GPUI's `img()`, which stretched a once-rasterised
texture and broke the marks at 1.25 and 1.5. The sharpness sheet
`kit/examples/mark_sharpness.rs` is the check.

The artwork picker header requests 16-point ink and reserves the official
image viewport width, rather than its rows' common 32-point column. A long header title follows the reference's
single-line ellipsis. Disabled artwork retains its approved colour/opacity;
the words and controls are muted separately. Enter validates the current
row's availability and bounds before dispatch. The 44-point touch model
pill holds OpenCode's 32 × 36 clear-space box; the short pointer pill and
inline spots use Generic. A Settings ProviderRow needs a fitted adapter,
not an unconstrained 36-high child inside its padded content band.

`view::AgentMark::of` maps only typed node identities; `app::agent_mark`
adapts them to kit artwork once. The draft, sidebar and board call that
adapter. Added agents retain Generic even if their labels mention a brand.
Settings › About lists vendor sources, trademark terms and the Gemini
spark's separate software licence. For long-name capture fixtures,
after `scripts/fleet.sh start ROOT TARGET`, run
`bash scripts/mark-fixtures.sh ROOT TARGET`; it checks the node's saved
bilingual titles and full custom-agent labels before captures.

Machine management uses `view::machine_book` for validated names, edit outcomes
and safe localized failures, and `link::machines` for the CLI's existing node
book. `BookFile` is reopened immediately before each atomic edit; the app does
not maintain a second machine list on disk. A forget ends the app's watched
stream only, leaving the remote session running. Original file errors remain
at the file boundary because a malformed book can contain credentials.

The actor resolves the book path once and reads it on the blocking pool for
startup, tick, reload, retry and edits. Syntax failures share `FileFault` with
pairing; no transient raw parser error reaches an event. Failed edits leave
clients and the session follow untouched. Browser refusals retain the original
Read/Edited response shape. Actor regressions use real framed RPC clients on
bounded in-process streams, including client-preserving rename and watched
forget; no real agent or network endpoint is contacted. Other areas' aliases
and project follows must be integrated before the Machines screen is delivered.

Branch choices distinguish local identity from a remote-tracking ref even when
their printable names match. Only the current local branch is a no-op; remote
choices always pass the existing remote-prefix and local-name validation.

The git toolbar (`app/src/areas/git/`) sits in track F's Changes-tab slot
and follows the repository F shows (`Shell::code_git_context`, read only).
It uses the workspace channel the code panel already has: a read is taken
when its echoed root is the one followed and a read of it is out; a write
when it equals the one write in flight. What choosing an action does is
`view::workspace::git::step` (explain in an Info toast and send nothing,
open a form, or send one prepared write). Its shapes on a running fleet:
`bash scripts/git-fixtures.sh ROOT TARGET` adds one project and session
per state (commit on a feature branch and on the default branch, push,
pull, no upstream, in step, push refused by the remote, no commits,
detached), bare remotes under `ROOT/remotes` with `main` as `origin/HEAD`, and a
device paired as a person pairs one in `ROOT/person-config`; the fleet's
`ROOT/readonly-config` shows what a read-only device sees. Run the app
with `HOME=ROOT` for either book: they name their credentials from it.
A branch name is drawn on one line cut in its middle, never wrapped
(`app/src/areas/git/name.rs`).

A session's pull request (`app/src/areas/pulls/`, rules in
`view::workspace::pull`) is read from its card (`CardDetail.pull_request`,
the node's watcher keeps it fresh) and asked on the same workspace channel
(`Ask::Pull`: `pr.get`, `pr.open`). The git menu offers Create PR or View
PR after the remote actions (Paseo's `git/policy.ts`); Create PR opens a
form with `gh pr create --fill`'s defaults, as the node takes no title
from a model. The link at the head of the Changes tab opens its facts; a
card shows its number, state and checks under the branch. The fleet's
nodes reach no forge (the remotes are bare repositories), so the screens
there show why a pull request cannot be opened.

The fake fleet also checks N3's node timezone and schedule preview through
`wire-gaps-fixtures.py`: ten runs, an omitted zone using the CLI's zone (different from the node's),
and an impossible-expression refusal. No schedule is saved by this check.

The reviewed settings controls from G3's 263d4a5/70bcfd5 supply the section
trailing slot and shared custom-content frame. Integration keeps the
lead's code_row and hint-face delegation; its private row_faced delegates
to that same frame. G3's 64167097 supplies red_300 unchanged in both themes
for ProviderRow error detail; semantic danger/status signals stay put.

`agent_mark::settings_mark(art, Size::Sixteen, SettingsRowEdge, tokens)`
fits reviewed artwork inside the unchanged shared settings row frame.
Its 24/23-high first/following layout slot uses the existing padding for
OpenCode's 32 × 36 quiet box; the image's integer edges and name baselines
are checked against the previous Chevron 14/Bot 16 composition. Generic
keeps that setting row's foreground colour. The kit still imports no view.
The standalone mark audit accepts `VESSEL_MARK_SCENE=pills`, `settings`
or `settings-errors` for the real pill_parts and shared-frame fit fixtures.
The measured reference ink and per-brand boxes are in the artwork source
record. No restricted vendor original is bundled by those fixtures.

For an initial Agents-loading capture, a FIFO named `discovery-gate` in a
fake node's scratch home holds fake Codex's version probe until the node's
own five-second bound ends it. Remove the FIFO before ordinary probes.
The fleet checks this fixture in an isolated scratch home using `timeout`.

The disposable settings fixture also checks read-only and session-access
device rows. For an already-seeded fake fleet, `settings-fixtures.py
row-access <fixture-root>` updates only its named inert records; fresh
`fleet.sh` runs seed those scopes directly.

It also joins the build server to the Settings fixture's laptop home,
checks leave and standalone refusal, revokes that temporary identity, then
joins with a long member label and waits for the real apply/report. The
existing behind/conflict report remains a synthetic Settings fixture.
Only disposable harness homes receive files. A real Read-only pairing is
stored as `laptop` in the separate `ROOT/readonly-config` book (select it
with `VESSEL_CONFIG_DIR`); it can read local history, cannot see member
identities and is refused sync/leave. `node pair --read-only` creates that
credential through the same public invitation RPC; no credential or link is
written to the fixture report.

## Spoken-conversation skeleton (ADR 0080)

`view::voice` models the seven conversation phases, exact node/session
identity, mute and generation-fenced starts/stops without I/O. Shared
`view::audio` keeps a single capture slot reserved through actual device
release confirmation, including Stop-during-start and stale callbacks. It cannot
mark capture ready before node enable, retarget a late completion, or
upload while muted or stopping. Its current public state is not a capability
claim: no voice entry point is displayed.

`link::voice` is staged under tests until the actual node adapter and duplex
`link::audio` device owner exist. Its bounded uploader and cancellation fence
separate immediate local Stop from uncertain remote cleanup; queued PCM is
discarded on Stop and device disposal precedes the remote wait. Fakes hold
remote requests and fill data queues to check privacy and deadlines. These
tests do not open a microphone or establish hardware/AEC readiness. The
shared physical capture lease remains owned until streams actually stop/drop,
including failure paths; dictation consumes that same engine.

`link::audio::Capture::acquire(Dictation)` exposes that single hardware owner.
It emits typed10ms mono16k PCM16 frames; control() gives a separate cloneable
Stop/status handle while a task awaits next(). Stream opening stays on the
device worker; Stop wakes a silent callback reader without polling. Other
platforms return typed Unsupported. Conversation acquisition returns
ConversationUnavailable until duplex output wiring exists. No product entry
point invokes acquisition yet; fake tests prove ownership, not physical device
or permission behavior. Its acquisition-wide64000-byte permit follows each
frame through raw capture, normalization, queuing and any borrowing upload.
A frame's uploadable predicate reflects its original capture interval; an old
or muted interval cannot reopen after unmute. The conversation mute mutator and
cancellable uploader are still test-only. The speaker callback/reference/drain
code remains staged.
Voice glyph source/notices live in kit/src/assets/voice/; their existing outline
Stop counterpart is unchanged.

Test-only `areas::voice` composes the shared meter and reviewed Circle button,
with typed Fluent EN/KO accessible names and state-derived switching/mute.
It has no Shell entry point or node command. `view::voice::Cue` supplies only
one-shot timing actions for the existing phase/generation; generated cue PCM
and real duplex rendering are still owed. No live preview or product voice
capability is enabled by these fake-driven surfaces.

Settings Overview keeps its page and book-edit commands in
`app/src/areas/settings/overview.rs`; `overview/dialogs.rs` only renders
rename/removal dialogs and delegates actions to that parent. The browser
demo's fixed agent and ACP-option catalog is `link/src/demo/catalog.rs`;
`demo.rs` retains session state and event generation. These boundaries do
not add another edit rule or another node implementation.

`cargo run -p vessel-app-kit --example host_rename_audit` opens an isolated
input audit of the shared rename action: ordinary, disabled, adjacent
button and clipped-parent cases. Counters distinguish the action, its
parent's mouse-down/release and the neighbour. It starts no node and adds no app
route. The fixed grid is an audit fixture; the action itself is unchanged.

Shared typed button content/loading/sizing is documented in
`docs/plans/backend-m12-g2-button-content.md`. Label/Icon/IconAndLabel share one
renderer; ButtonIcon::muted keeps the explicit source icon tone. button_control
adds guarded disabled/loading callbacks and Small28/px12 in both layouts with
Small20 ActivityIndicator. Medium is Pointer32/px16 or Touch44/px24; legacy
no-Window signatures remain Pointer. The published outline glyph API retains
its explicit tone and immediate-child geometry in the same shared renderer.

Action menus retain one published renderer (menu_right/TopRight, QUICK appear,
fill_in) and add typed leading14/16, Pending/Success, selectable Muted, KeepOpen,
measured open_end and caller width. See docs/plans/backend-m12-g2-menu-base4.md
for exact source, focus limits and consumer integration. The canonical palette
GREEN_500 and existing CircleCheck own success ink/art; no second palette.

The shared trigger adds ProviderRow (More14, visual32/r8/hit48, open/hover
surface2+foreground, pressed surface3 background only) through one frame.
trigger_presented takes typed Closed/Open with ButtonState. Existing entries
keep Schedule/Git defaults. trigger_visual_size/trigger_space derive the same
visual footprint; passive size has no identity, input, glyph or anchor.
See docs/plans/backend-m12-g2-trigger-profiles.md for source and pending proof.

Menu Content's typed Layer keeps Screen/priority1 as default and offers
AboveDialog/priority3 through the same renderer. Its transparent viewport
input boundary consumes only the nested menu's outside press, preserving
its host dialog; no second scrim or area popup is drawn. See
`docs/plans/backend-m12-g2-modal-menu.md` for source and pending proof.

### Allocation block approval boundary (base 5 adapter)

`command_changes::Listed` projects every `ServicePortsView` field explicitly.
Until the approval card displays and compares the full allocation block, it
refuses any current or last-approved block, even with zero named services or
when the current set removes the old block. The approved-folder comparison
fallback also retains that refusal. A typed refusal has separate EN/KO
Fluent copy; it does not enable allocation, helpers or `PROJECT_FILE_KEYS`.
This keeps an unseen setting out of an approval digest while the native display
is implemented. Existing terminal refusal and head test cases remain intact.

button_with_design accepts ButtonDesign::Regular(ButtonConfig) or the typed
CircleConfig family through the same button body/input guard. CircleConfig
has no density/regular-size/kind fields: it is44 square, round, icon20, with
Surface (surface0/neutral border/foreground) or Danger (fixed palette red600,
red800,white), no hover or pressed override. Loading uses shared Small20 and
refuses clicks. Existing button_control and every legacy signature stay intact.
Button::aria_label forwards an explicit role/name without creating a keymap;
consumer accessibility/OS/first-use evidence remains unverified.

The shared audio input constructor is `link::audio::device::input`; its raw
stream/callback queue is separate from capture-only DSP so a duplex bundle can
reuse that constructor with one capture/render processor. Stream disposal still
precedes retirement of the process-wide capture guard. Conversation acquisition
remains unavailable until that bundle/controller is connected.

The staged `link::audio::duplex` bundle constructs both streams on that device
thread and shares one processor with the existing playback Pump. Its input
wait handles actual output references/completions and receipt deadlines before
capture; partial startup failure/Drop closes the shared local fence. This is
test-only source pending controller wiring and checks. It does not enable the
public Conversation acquisition path or attest any physical device/AEC quality.

Waiting-cue PCM is generated independently in `link::audio::playback::cue`,
with the shared view duration and ordinary speech Packet/Pump path. It has no
copied recording, decoder or separate audio engine. The new quiet three-note
waveform differs from the reference asset; actual perceptual/device validation
and controller/timer activation are still owed.
