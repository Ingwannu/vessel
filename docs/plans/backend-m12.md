# Milestone 12: a native app

Status: started 2026-11-13 with a spike. Built on `backend/m12`, stacked
on `backend/m11` (not merged yet; milestone 11's second wave is still
being joined, and goes on beside this).

Read first: `AGENTS.md`, ADR 0035 (the decision and what it costs),
ADR 0017 (what the page is), `docs/design/web-ui/DESIGN.md` (the look
that is carried over), `docs/PERFORMANCE.md` (what the page measured).

## Goal

The owner's words, 2026-11-13: is a desktop app being made, and is it
light and fast as Zed is, and does it update well; then, choosing: Zed's
way, and the web and the phone too, by installing an app, as far as it
goes.

And, the same day: Zed's is not to be used as it is, because that
brings problems; it is a reference, and so is Tauri 2.

So: a desktop app that is native (no web view), drawn as Zed draws its
own; the same UI on a phone and in a browser where that can be made to
work; an app that updates itself. Zed is the model for the UI and Tauri
2 for what surrounds it (phones, installers, updates, extensions);
neither is taken whole (ADR 0035).

And then: build the app as soon as possible and hand over a link, a
first build labelled 0.0.0.0.1, to be tried and answered with feedback,
again and again.

So the order is turned round: **a build the owner can install comes
first**, and the questions below are answered on the way, by the tracks
that build it (see "Tracks"). A build is real: it talks to a real node,
shows its machines and sessions, opens a session and sends a prompt.

## Before writing code

1. **What exists.** Everything below the screen: the node, the wire
   (`vessel-proto`), the client with its routes, pairing and credentials
   (`vessel-client`), the node book and the preferences document's rules
   (`vessel-cli`, `vessel-web`). The design system and every screen's
   behaviour, as the page has them. The view logic of `web/src/data`,
   in TypeScript.
2. **Reuse or new.** The client, the wire and the book are used as they
   are: the app is a client, as the CLI is. The view logic is ported to
   Rust once, with its tests' cases, into a crate with no UI in it. The
   screens are new.
3. **Outside.** GPUI (Apache-2.0) and `gpui-component` (Apache-2.0) as
   the model for the UI and, as far as the spike finds they hold, its
   base; Tauri 2 (MIT or Apache-2.0) as the model for packaging, phones
   and updates; a terminal emulator crate for terminals, as Zed uses one;
   the community mobile layer and GPUI's web platform, if the spike
   finds they hold. Each with its licence checked against ADR 0014.
4. **Decision.** New development on an outside framework: ADR 0035.

## Phase 0: the spike (`backend/m12-spike`)

Nothing is promised about a phone or a browser, and little about the
desktop's numbers, until something of Vessel's has been seen running
there. The spike is thrown away or kept by what it finds; it lives
outside the main workspace (`spikes/native/`, a workspace of its own) so
the gate of `AGENTS.md` neither builds it nor is slowed by it.

One small real screen everywhere: the sidebar with machines and their
sessions, and a board's lane of session cards, read from a real node
through `vessel-client`, in the page's look (tokens, Geist, Pretendard),
with a text field one can type Hangul into.

Questions, each answered with what was run and what was seen:

1. **Which GPUI.** The released crate, the upstream snapshots the mobile
   and web layers build on, or Zed's tree: which of them builds the
   desktop, which the phone and the browser need, and whether one choice
   serves all. Every crate pulled in, with its licence: nothing that
   cannot be linked into Vessel (ADR 0014).
2. **Desktop, Linux.** The screen in a window, captured. Hangul drawn in
   Pretendard; what typing Hangul through an input method needs.
3. **Numbers**, beside the page's in `docs/PERFORMANCE.md`: the release
   binary's size, time from start to first frame, memory and CPU at
   rest, a frame's time while a list of 20,000 rows scrolls. Said plainly
   where a number is from a software renderer and so says little.
4. **Desktop, Windows and macOS.** What can be checked from Linux, and
   exactly what a CI job would run.
5. **Android.** The screen in the emulator, captured; touch scrolling;
   typing Hangul. What does not work, by name.
6. **Browser.** The screen from WebAssembly in a headless browser with
   WebGPU, captured; the size of what a browser downloads; what happens
   without WebGPU.
7. **iOS.** Cannot be built here. From the code: what a macOS job would
   need, and what is known not to work.
8. **Updates.** How Zed updates itself and how Tauri 2's updater does
   (its signature, its endpoints), read from their source; what Vessel
   needs (artifacts per platform, a signature, where they are
   published), not built.
9. **Not as it is.** What goes wrong if GPUI is taken as it is, each
   problem shown and not supposed (the released crate against the tree,
   the snapshots, what breaks between two of them, what Vessel's screens
   need that it lacks). Then the three ways of ADR 0035 with what each
   costs: a pinned GPUI; a pinned copy Vessel patches; the layers below
   (a GPU renderer, windowing, text shaping with input methods, layout)
   with GPUI as the model. What Vessel's screens need of a UI layer, by
   name (a list of 20,000 rows, a text field with input methods, rich
   text, a terminal's grid, menus, focus, accessibility), against each.
10. **Tauri 2 as the reference for the rest.** Read from its source and
    documentation: how one Rust core becomes an Android and an iOS app;
    its bundler's installers; its updater; its permissions and plugins.
    What a native (no web view) app takes from each, as code or as
    design. And the fallback: whether a Tauri 2 shell showing the page
    is a sound phone app where the native UI cannot reach, and what it
    would need of `vessel-client` (ADR 0020 described it).

The spike ends with reports (one per track, see "Tracks"): each
question's answer with its evidence, what it recommends for ADR 0035's
open points, and a proposal for the phases below with their order.

## Tracks (2026-11-13)

The spike's questions are split between three tracks, each in a
worktree and a cargo workspace of its own under `spikes/` (outside the
main workspace, which excludes `spikes` whole).

| Track | Branch | Where | Builds | Questions |
|---|---|---|---|---|
| Desktop | `backend/m12-spike` | `spikes/native/` | the app: connecting to a node, the sidebar and a board lane, a session with its composer; the Linux artifact | 1, 2, 3, 9 |
| R, release | `backend/m12-release` | `spikes/release/` | the Windows build from Linux, the installer, one script for every artifact, the signed update feed and the updater crate, the CI job for what cannot be built here | 4, 8 |
| M, phone | `backend/m12-phone` | `spikes/phone/` | Android: the native layer in the emulator (time-boxed), then the Tauri 2 shell around the page as an APK; what Tauri 2 teaches; iOS and the browser from the code | 5, 6, 7, 10 |

Rules the three share:

- Builds carry a label the coordinator gives out (`0.0.0.0.1`, then
  `0.0.0.0.2`, ...): a display label from a build-time variable, not the
  crate's version. Artifacts go to one directory per label with their
  checksums; the coordinator publishes from there.
- Screens are written against a kit of Vessel's own; nothing of GPUI's
  or Tauri's types reaches the crate that holds view logic.
- Signing keys (the update feed's, Android's) are made once outside the
  repository and never copied into it or into an artifact.
- Each track keeps notes of what it ran and what happened
  (`backend-m12-spike.md`, `backend-m12-release.md`,
  `backend-m12-phone.md`): the evidence for its questions.

## Build 0.0.0.0.1 and what the owner said (2026-11-13)

Windows (an installer, a zip) and Linux, from the desktop track's commit
that pairs, lists machines and sessions, and opens a session. On the
owner's Windows 11 the window opened, text drew, and pairing through the
relay worked. The owner's words, as given:

1. The app is good but awkward to use: **it should do everything the
   page does, the same way.**
2. Starting a first chat should be as in GOAT and agent-orchestrator:
   **type, send, and it runs.** As it is, something feels wrong.
3. The top of the window, with minimise, maximise and close, should be
   **drawn by the app itself, as Zed's is**, not Windows' own bar.

And from the first minutes of use: after pasting the pairing link the
owner typed the key into the name field (the screen's wording invited
it), and the machine the node ran on had no `vessel` command to compare
the key with.

And, a little later: nearly everything Paseo and GOAT support must be
supported.

So the milestone's scope is not a few screens. It is the page's whole
surface and, beyond the page, the feature tables of milestone 11
(`backend-m11.md`: what GOAT, Paseo and agent-orchestrator do, and
whether Vessel does): what those tables mark as built on the node but
not yet drawn is drawn in the app first, and what they mark as not
built is built on the node and drawn in the app.

In order:

- **0.0.0.0.2**: a new session by typing and sending (the references'
  flow, reproduced; `new-session-flow-compare` is the side-by-side the
  workers check against); the pairing screen in two plain steps with a
  prefilled name; the app's own title bar; a string table with Korean;
  the app's README and a layout in which several workers add screens
  without sharing files.
- **Then parity, in parallel tracks**, cut from an inventory of the
  page: the conversation in full; terminals; settings, machines,
  devices and the fleet; projects, files, git and pull requests;
  schedules. Each track ports its view logic into the crate without
  GPUI first, with the page's test cases, then draws it.
- The page's own new-session flow is brought to the references' in the
  same pass, so the two clients do not differ.

## One experience everywhere, and the second set of tracks (2026-11-13)

The owner's main requirement, said after the above: the desktop app,
the web and the phone app feel the same everywhere. ADR 0036 decides
how: one UI crate draws the desktop and the phone (Android was seen
working in the emulator, Hangul composition included:
`backend-m12-phone.md`); the browser gets the same UI if a track shows
it holds there, else the page, bound to the app by shared tokens, words,
keys and behaviour fixtures; a person's settings follow the person.

The page's surface was inventoried (cut into six groups) and the tracks
grew accordingly. Each app track ports its view logic first, in the
crate without GPUI, against the page's test cases, then draws, from 360
points wide.

| Track | Branch | Takes |
|---|---|---|
| Lead (desktop) | `backend/m12-spike` | the shell, the kit, the title bar, the new chat and the composer, pairing, the string table, the layout of `app`; merges the app tracks |
| C, conversation | `backend/m12-app-conversation` | the timeline in full, the session's bar and actions, find, outline, the inspector's summary, the terminal view |
| F, workspace | `backend/m12-app-workspace` | projects, worktrees, the approval, the code panel (changes, files), where a new session runs; then pull requests, shells, scripts, ports |
| Browser | `backend/m12-browser` | the same UI crate in a browser: does it draw, does Hangul compose, what a phone browser and a browser without WebGPU get (question 6; ADR 0036's open point) |
| M, phone | `backend/m12-phone` | the native Android app from the UI crate; the shell as the fallback |
| R, release | `backend/m12-release` | builds for every system, the installer, the updater |
| S, shell furniture (branch made, not started: waits for a free worker) | `backend/m12-app-shell` | the board and the archive in full, the sidebar in full, the command palette, the shortcut registry with its listener and settings page, notices and toasts, the system's notifications |
| E, settings (branch made, not started: waits for a free worker) | `backend/m12-app-settings` | the settings frame and its sections, the preference model behind one interface, machine management, devices and pairing links, sync, agents, models; then installing agents |
| next | | schedules and saved prompts, export and import; settings kept on a node (ADR 0036); the inbox and push |

Who owns which area and which kit control is kept in one place for the
workers (the tracks' brief, outside the repository) and repeated in
`spikes/native/README.md` by the lead. In short: the lead owns the
window, the routes, the title bar, the kit's base, the new chat and the
composer, pairing and the update notice, and merges every track; a
control another track owns is never written twice.

Builds so far: 0.0.0.0.1 (pairing, machines, sessions, a session);
0.0.0.0.2 (the app opens on a composer and a send opens the chat at
once; pairing in two steps; the app's own title bar; the owner's mark;
Korean by the system's language); 0.0.0.0.3 brings automatic updates,
as the owner asked ("as Zed has them"): the updater track R built, a
signed feed, the notice in the title bar.

## Build 0.0.0.0.3 and the look (2026-11-13)

The owner ran 0.0.0.0.3 on Windows 11 and said, in short:

1. Minimise, maximise and close need a double click (a defect of the
   app-drawn title bar on Windows).
2. The composer's text field is "the default one": a boxed input with a
   bright focus ring. It must be as Paseo's: one types straight into the
   card. The same holds for dropdowns from now on.
3. Only Claude Code is offered; and the model selector and the rest of
   the tray were made as the builder pleased. They must be as Paseo's,
   with GOAT.
4. Are settings and the rest being laid out as in Paseo, "with a Paseo +
   GOAT UI"?

So the look has a named measure, for the app and, after it, for the
page (ADR 0036 keeps them one):

- **The app's layout and controls follow Paseo's app and GOAT, cloned
  closely**: Paseo first (it is one UI on desktop, web and phone), GOAT
  where Paseo has nothing or for the conversation's details. Values,
  structure and behaviour are reproduced; each deliberate difference is
  written down with its reason.
- **Nothing looks like a component library's default.** Every control
  has one look, defined in the kit after the references.
- **Every agent a machine has is offered**, the built-in ones and the
  added ones, those that cannot start shown with the reason.
- The page's `DESIGN.md` stays the specification for behaviour the
  references lack (the board's lanes) and is brought to the same look
  afterwards.

A value-level specification drawn from both references' source is kept
for the workers beside their briefs.

0.0.0.0.4 carries: the title bar's buttons answering one click, the
composer and the pickers after Paseo, every agent listed, the update
notice in the title bar, the corrected mark. A build's number, once
published, is never built again.

**Why the buttons needed a double click** (track R, read from the
code): Windows asks the app what is under the pointer, and GPUI answers
with the first window control area, in paint order, that the pointer is
inside. The bar (the area that drags the window) is the buttons' parent
and is painted first, and the buttons did not occlude it, so over a
button Windows was told "caption". One click was therefore a press on
the caption (nothing happens); two were a caption double click, which
maximises or restores whichever button is under the pointer. The
buttons' hover colour is drawn by the app, so they looked right. The
fix is for each caption button to occlude what is under it, as Zed's
do, and a test in the kit that fails when the bar is hit over a button.
The rule for the title bar from now on: anything clickable drawn inside
the drag area occludes it.

## Builds 0.0.0.0.4 and 0.0.0.0.5, and what the owner saw (2026-11-13)

0.0.0.0.4: the window's buttons answer one click, the composer is the
card itself, the pickers and the combined model selector after Paseo,
every agent of the machine listed. 0.0.0.0.5: a session that ended or
was interrupted is continued by sending in it; agents that are not
installed are listed with how to install them.

**Updates work on real Windows**: the owner's installed 0.0.0.0.4 took
0.0.0.0.5 from the feed in two seconds and reopened as 0.0.0.0.5. The
0.0.0.0.3 before it read the feed twice and never fetched the artifact;
it shows nothing on failure and writes no log, so the cause is not
known. 0.0.0.0.6 adds a log file, the updater's steps in it, and
retries after a transient failure.

**The owner on 0.0.0.0.5** (Windows 11, dark): the model selector and
the other pickers look bad, the settings look bad, the version and
update screen looks very bad. Their captures show a list whose long
descriptions are drawn over the rows below (the real agent's options
have several-line descriptions; the fake fleet's are short), a bright
focus ring around a whole popup and around a whole panel, and an About
page that is loose text with a chip where a switch belongs.

What changes in how builds are checked:

- Every picker, every level of it, and every settings screen is opened
  against the real node's agents before a build is named, in the
  owner's theme; the coordinator looks at every capture, not a sample.
- The fake fleet carries the real shapes (long descriptions, many
  values, long names, Korean), with a test.
- No container, popup, panel or page can show a focus ring: removed at
  the root, not screen by screen.
- Settings pages, About included, are Paseo's settings layout.

## Build 0.0.0.0.7, and the composer's controls redrawn (2026-11-13)

0.0.0.0.7: feature pills with names and tooltips, the Settings frame
(General, Appearance with the three themes, About), pairing as a
two-step dialog, lists opening on their trigger's side, the
conversation against real shapes (code blocks, tables, the output box,
the person's message as a bubble), a session renamed where its title
stands, the adapter's start-up log out of the conversation, a plain log
file and readable panic frames.

The owner on its row of pills: not that way. What they asked for, with
captures of the control they mean:

- **One pill for the model and the effort.** It opens a small card: the
  level's word, the model's name under it (which opens the model list as
  before), a lightning icon that turns fast mode on and off, a reset to
  the agent's default, and a **slider** whose steps are the agent's
  levels.
- **The top of the slider holds what was separate**: an agent's "ultra"
  feature (Claude Code's Ultracode) is the last step, not a toggle of
  its own; an agent's own top levels are simply the last steps.
- **Fast mode shows in the slider**: with it on, the track moves and
  feels quicker.
- **No separate pill for an agent's other options.** The owner asked
  what "Agent" was; a control the owner cannot name does not belong in
  the row. What is neither effort, the top step nor fast moves to the
  top of the model list, each with a line saying what it is.

So the row is: the model-and-effort pill, the mode pill, send. This is
the first item of 0.0.0.0.8, and the page gets the same control in its
next pass (ADR 0036). The rule drawn from it: an agent's raw options are
not laid out as they arrive; each is placed where a person would look
for it, or kept out of the row.

## Everything the references do: who builds what (2026-11-13)

The owner, after build 0.0.0.0.6: build every Paseo and GOAT feature,
settings and the rest, quickly. The measure is the feature tables of
milestone 11 (`backend-m11.md`, "What the references do, and what
Vessel does"). This table says, per area, who draws it in the app and
whether the node has it. A row with no owner is a gap; there is none
left unassigned.

| Area (the tables' sections) | In the app | The node |
|---|---|---|
| A session: queue, steer, interrupt-and-send, questions, context meter and compact, plans, usage line, pin, archive, delete, unread, find, outline, read marks, started sessions under their starter | track C | has it (A, W) |
| A session as the agent's terminal; shell terminals | track C (the terminal's surface), track F (where shells sit) | has it (C) |
| The composer: attachments, the agent's `/` commands, `@` a file, drafts per session, recall of sent messages, a selection sent as a comment | the lead (the card), track F (`@` and the selection) | has it (A, B) |
| Agents: every agent of a machine, any ACP agent, installed and signed in, installing from the app, defaults per agent | the lead (the selector), track G3 (the Agents and Models pages) | has it (G) |
| Approvals: in the session; everything waiting, from every machine, in one place | track C; track S (the board's lane, then the inbox) | the inbox is not built |
| Projects, worktrees, the code panel, git, pull requests and their page | track F; track G1 (the git menu, the pull request pages) | has it (B, B2) |
| Scripts, services, ports and what opens | track F | has it (C); a browser panel is not built |
| Machines, devices, pairing links, sync | the lead (pairing), track E (machines), track G4 (devices, links, sync) | has it |
| Schedules, the recurring prompt, saved prompts, webhooks | track G2 | has it (D), but not the webhook calls |
| The board, the archive, the sidebar (group, sort, filter, pin), the command palette, shortcuts, notifications and sound | track S | has it; the inbox is not built |
| Settings: themes, sizes, widths, shortcuts, languages, updates, logs, diagnostics | track E (the frame and the pages), track S (shortcuts), track R (updates, logs) | more themes are not built |
| Rewind, fork, import from an agent's history, search across sessions, parking idle agents, a handoff to another agent | after the node has them: track C | track A2, started 2026-11-13 |
| Usage and limits (tokens, cost, each account's windows; what to do at a limit) | after the node: tracks S and E | not built (F) |
| A phone app; push | track M (paused for budget) | push is not built |
| The browser as a surface | the browser track (paused for budget) | |

Node tracks still to run, in this order: A2 (started; protocol 12,
schema 13, ADRs 0037 and 0038), the inbox and push, usage and limits,
the rest of the tables (interface switch, a side question, one prompt
to several models, delivered files, a reviewing agent, spreading by
load, diagnostics), the browser panel, themes, dictation, plugins. The
page follows the app in look (ADR 0036) and gets each new feature in
the same pass as the app.

What limits the pace is the accounts, not the plan: three accounts give
about an hour of four workers per five-hour window each, two of them
with weekly room. Two app tracks (S and E) run on the stand-in capacity
in the gaps.

### Four added tracks, held strictly (2026-11-13)

The owner asked for four more workers on a second model, and said how
they are to work: code quality and everything said so far, held very
tightly, and reviewed well. They take rows out of the table above so
that nobody builds the same thing twice:

| Track | Branch | Takes | From |
|---|---|---|---|
| G1 | `backend/m12-app-git` | the git menu of a session and the pull request pages | track F (who keeps projects, the code panel and the place pills) |
| G2 | `backend/m12-app-schedules` | schedules, their runs, saved prompts | track E |
| G3 | `backend/m12-app-agents` | Settings › Agents and Models | track E |
| G4 | `backend/m12-app-devices` | Settings › Devices and Sync | track E (who keeps the frame, General, Appearance, Shortcuts, Machines) |

[Decision Log]
- Purpose: more of the table built per hour without the look or the
  code slipping, on capacity that does not come out of the three
  accounts' windows.
- Constraints: a second model has not seen the owner's corrections of
  this milestone; the owner's two complaints so far were both of work
  that passed because nobody looked closely (short fake shapes, one
  level of a picker checked).
- Alternatives: give them node work (rejected: the wire and the store
  are where a mistake costs most and A2 is in them); let them share
  areas with the running tracks (rejected: two hands in one file);
  merge on their own report (rejected: see the constraints).
- Choice: four areas that touch no running track's files beyond a
  registration line or a slot; one brief of strict rules (no escape
  hatches in the code, types that carry meaning, logic in `view` and
  tested there against the page's cases, kit controls only, real
  shapes in both themes, sizes and languages); the coordinator reads
  each slice line by line and looks at each capture before the lead
  merges it, and a slice that breaks a rule goes back.
- Costs: the review is the coordinator's time, so slices arrive one at
  a time, not batched; two kit controls (a split button, a QR drawing)
  are written by these tracks, not by the lead, and the lead may ask
  for them reshaped.

What the node does not send yet, found by these tracks in their first
hour. The app draws none of it until the node has it (nothing shown may
do nothing); each is for a node track:

| Missing | Where it shows | Until then |
|---|---|---|
| Open pull requests of a project with no live session (`pr.list` walks watched sessions only) | the pull request page | the page is "pull requests of this machine's sessions" and says so |
| The webhook calls (`hook.*`): ADR 0034 has the types and the store, not the calls, the listener or the CLI | schedules | no webhook screen |
| A machine's time zone | schedules | a rule's own zone and the viewer's zone, each named |
| The next runs of an expression before it is saved | the schedule editor | a preview computed in the app with the node's library and the node's test cases; the lasting fix is one implementation both call |
| Fleet join, leave and "sync now" over the wire (the CLI does them in the data directory), and a member's applied state | Settings › Sync | read-only, with the command to run |
| The registry's address from the command line or the environment | the fake fleet (a fixture registry cannot be served) | the rules are tested against a fixture; screens show the live public list |

## The browser, first report (2026-11-13)

The browser track built the same `app` crate for `wasm32` (GPUI's web
platform, single thread, the pinned stable toolchain) and showed, in
headless Chromium with WebGPU:

- **It draws**: the new chat with its pickers and the mark, the sidebar,
  a session's conversation.
- **Hangul composes** in the new chat's composer and in a session's:
  marked text underlined at the caret, the final consonant carried to
  the next syllable, a cancelled composition not inserted, Enter sends.

Not shown yet, and stated so: the data was a built-in demo (no
connection to a node from a browser); the renderer was software WebGPU;
the composition was driven through the browser's debugging interface,
not typed with a real input method. ADR 0036 lists what must still be
seen before the browser gets the native UI; until then the page stays
the web client.

What it taught the desktop: a character the app's typefaces lack (the
pickers' small triangle) shows only by the system's fallback on the
desktop and as a missing-glyph box in a browser, so carets and marks are
drawn as vector glyphs from the kit, never as characters; and the title
bar's window buttons belong to the desktop only.

## After the first builds (written out as the tracks report)

- **View models** in Rust (`vessel-view`): the conversation folded from
  records, the board, an agent's controls, the sidebar, drafts and
  preferences, find; ported from `web/src/data` with its tests' cases.
- **The shell and the kit**: the window, the theme from the design
  system's tokens (light, dark, system), the type, the controls the
  screens need on `gpui-component`, a gallery of them.
- **Screens**, in the order the page got them: machines and pairing;
  the board and the sidebar; a session (the conversation, the composer,
  permissions; a terminal); a new session; settings. Then what the page
  does not have yet: pull requests, shells and scripts, schedules,
  adding agents.
- **Releases and updates**: the pipeline, signing, the updater.
- **Android**, **iOS**, **the browser**: each if the spike says it holds.

## What happens to the page meanwhile

It stays, and stays the browser's client (ADR 0035). Milestone 11's
joins in flight are finished and deployed, so what wave 2 built can be
used. New screens are drawn in the native app first.

## Left for the owner

- Where releases are published, the signing key, Windows code signing,
  macOS notarisation, an Apple developer account for iOS.
- Whether CI may build what this machine cannot (Windows, macOS, iOS):
  it needs branches pushed, which `AGENTS.md` forbids today.
