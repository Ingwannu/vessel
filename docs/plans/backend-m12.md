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

So: a desktop app that is native (no web view), on the framework Zed is
built on; the same UI on a phone and in a browser where that can be made
to work; an app that updates itself.

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
3. **Outside.** GPUI (Apache-2.0) and `gpui-component` (Apache-2.0) for
   the UI; a terminal emulator crate for terminals, as Zed uses one;
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
8. **Updates.** How Zed updates itself, read from its source; what
   Vessel needs for the same (artifacts per platform, a signature, where
   they are published), not built.

The spike ends with a report (`docs/plans/backend-m12-spike.md`): each
question's answer with its evidence, what it recommends for ADR 0035's
open points, and a proposal for the phases below with their order.

## After the spike (written out when it reports)

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
