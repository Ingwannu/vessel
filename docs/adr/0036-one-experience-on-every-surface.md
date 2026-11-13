# ADR 0036: One experience on every surface

Status: accepted as the aim, 2026-11-13 (the owner's words: the main
thing is that the desktop app, the web and the phone app feel the same
everywhere). How the browser is served is open until the browser track
reports; the rest is decided here.

## Context

Vessel now has two clients drawn by different code: the page (React,
ADR 0017) and the native app (Rust on a pinned GPUI snapshot, ADR 0035).
The owner used both on the same day and found the app awkward beside the
page, asked for everything the page does and for nearly everything GOAT
and Paseo do, and then named what matters most: one experience on the
desktop, on the web and on the phone.

Two clients written twice drift. Every screen of milestone 10 and 11
was judged by looking; nothing checks that a second client still looks
and behaves like the first. And a person's settings (theme, shortcuts,
how the sidebar is grouped, what Enter does) are kept beside each
client today (ADR 0028: by the bridge, for the page), so a phone and a
desktop of the same person would differ on the first day.

What the tracks of milestone 12 have shown so far:

- The app's UI runs on Windows 11 (the owner's machine) and on Linux.
- GPUI's community mobile layer runs on Android in the emulator: a
  window draws, touch scrolls with momentum, the soft keyboard opens,
  and Hangul composes in an input written against GPUI's input handler
  (the kind the app's kit uses). Two gaps, both small and on the
  layer's side: the keyboard's inset is not reported on Android, and
  the activity must be hosted the layer's second way to survive being
  made again. Seen in an emulator with a software renderer, not on a
  phone.
- The browser (GPUI's WebAssembly platform) has not been tried.

## Options

1. **Two clients kept alike by care.** What exists. It does not hold:
   the first day of use showed it.
2. **One web UI everywhere** (the page in Tauri shells; ADR 0020). One
   experience by construction, but the desktop is a web view, which the
   owner declined in ADR 0035.
3. **One native UI everywhere**: the app's UI crate on the desktop, on
   the phone, and in the browser through WebAssembly. One experience by
   construction, if the browser holds: a canvas has no text fields of
   its own, so input methods, selection, accessibility and phone
   browsers are where it may fail, and it needs WebGPU.
4. **One native UI on the desktop and the phone; the browser by
   whichever of the two holds**: the same UI through WebAssembly where
   it is good enough to be the web client, else the page, bound to the
   app by shared sources and checks so that it cannot drift unseen.

## Decision

Option 4.

- **One UI crate draws the desktop and the phone.** The phone is not a
  second design: every screen is written to work from 360 points wide,
  with the page's own rules for a narrow window (the sidebar becomes a
  sheet, the inspector a sheet, 44-point touch targets). The Tauri 2
  shell around the page stays as the fallback ADR 0035 names, shipped
  only where the native phone app cannot be.
- **The browser is decided by evidence.** A track builds the same UI
  crate for the browser and reports: does it draw, does Hangul compose,
  clipboard, links, what a browser downloads, what happens without
  WebGPU. If it holds, the browser gets the same UI and the page
  remains for browsers that cannot run it. If it does not, the page
  stays the web client and is bound to the app as below.
- **While two renderers exist, four things have one source**, read by
  both and checked by tests on both sides:
  1. *Tokens*: colours, type, spacing, radii, breakpoints. The design
     system's token file is the source; the kit's constants are
     generated from it or tested against it.
  2. *Words*: every string, in English and Korean, in one table.
  3. *Keys and commands*: the shortcut registry and the command list.
  4. *Behaviour*: the view logic (how records fold into a conversation,
     which lane a card is in, what a send does during a turn, what is
     remembered) is checked by fixtures both implementations run: the
     page's test cases are the fixtures, and the Rust port must pass
     the same inputs with the same outputs.
- **A person's settings follow the person.** Preferences are kept where
  every client of that person can read them (a node the person chooses
  as home, cached by each client), with only what belongs to a device
  (text size, the window's layout) kept on the device. This changes
  ADR 0028 and needs a wire addition; its own ADR and track decide the
  shape. Drafts follow the same rule if the track finds it sound.
- **A flow is designed once.** A change to a flow (the new chat first)
  is made in the app and in the page in the same pass, from the same
  written comparison with the references.
- **The references stay the measure**: the feature tables of milestone
  11 name what GOAT, Paseo and agent-orchestrator do; "the same
  experience" means the same on every surface *and* close to them.

## Why not one web UI, again

The owner chose the native desktop knowing the shells were on offer
(ADR 0035), and the first build confirmed the choice: it started and
drew on Windows at the first attempt. What the owner asks for now is
that the choice not cost sameness. Option 4 pays for that with checks,
and option 3 inside it removes the cost entirely where it holds.

## Costs

- While the page and the app both exist, every screen is drawn twice.
  The shared sources and fixtures bound the drift; they do not remove
  the work.
- The phone app depends on a community layer for GPUI that is young.
  Vessel pins it, patches what it must (the keyboard's inset), and
  keeps the shell as the way out.
- Settings on a node are a wire change and a migration from the
  bridge's file.
- If the browser gets the native UI, people on browsers without WebGPU
  see the page: two experiences again, for them. That is accepted and
  said.

## Open

- The browser (the browser track's report).
- iOS: cannot be built here; the mobile layer supports it and the CI
  job is written, not run. An Apple developer account is the owner's.
- Where preferences live and how they move (its own ADR).
- Whether the page's view logic is replaced by the Rust port compiled
  to WebAssembly, which would make behaviour one code and not two
  checked against each other. Not before the port is complete.

## References

- ADR 0017 (the page), ADR 0020 (the shells), ADR 0028 (preferences
  kept by the bridge), ADR 0035 (the native app).
- `docs/plans/backend-m12.md`, `docs/plans/backend-m12-phone.md`
  (what was seen on Android), `docs/plans/backend-m11.md` (the feature
  tables).
