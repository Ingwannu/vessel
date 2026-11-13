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
| next | | the board and a session's lifecycle; machines, devices, sync, agents, models and the settings; what the page never drew (agent install, schedules, export); settings kept on a node (ADR 0036) |

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
