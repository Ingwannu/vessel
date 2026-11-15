# Milestone 12, the spike: the desktop app (questions 1, 2, 3 and 9)

Status: in progress, 2026-11-13. The desktop track of the plan's Phase 0
(`docs/plans/backend-m12.md`): the app itself on Linux, and questions 1
(which GPUI, licences), 2 (the Linux desktop), 3 (numbers) and 9 (what
goes wrong when GPUI is taken as it is). Windows, packaging and updates
(questions 4 and 8) and the phone, the browser and Tauri 2 (5, 6, 7, 10)
are other tracks' notes.

The order of work changed on the day (the owner: a build to install and
try first, the report after), so this file grows as the builds do. Every
statement below was run and seen unless it says "read" or "not verified".

Captures are in a local folder on the machine that made them, not
committed (file names below).

## The app as built

`spikes/native/` (its README has the structure and how to run it): four
crates, `view` and `link` without anything of GPUI's, `kit` (Vessel's own
kit over GPUI) and `app`. Build labels: `0.0.0.0.1` is the first one
given to the owner.

What the first build does, each seen working against two real nodes
started with the testkit's fake agents (`scripts/fleet.sh`), on Linux
under Xvfb with Mesa's software Vulkan (lavapipe):

| What | Seen | Capture |
| --- | --- | --- |
| Machines in the book with their sessions; the board's four lanes; light theme | yes | `desktop-board-light.png` |
| A session opened from a card: its conversation streamed from the node, drawn as a list that lays out only what is in view | yes (40 entries) | `desktop-session-light.png` |
| Hangul typed into the composer, drawn in Pretendard, sent; the agent's reply with Hangul | yes, through `xdotool type` (no input method: see question 2) | `desktop-hangul-typed.png`, `desktop-permission-light.png` |
| A permission asked by the agent, answered with a button; the tool call then completes | yes | `desktop-permission-light.png`, `desktop-answered-light.png` |
| Pairing from a pasted `vessel node pair` link: the node's id, key fingerprint, routes and expiry shown; named; paired over the secure channel; the credential written (mode 600) and the node added to the book; its sessions on the board; dark theme | yes (a direct route on loopback; a relay route not verified here) | `desktop-pair-offer.png`, `desktop-paired.png` |

## Question 1: which GPUI, and the licences

**Read on 2026-11-13** (crates.io's API, the repositories):

- `gpui` on crates.io is 0.2.2, from 2025-10-22: a year behind Zed's
  tree. Nothing builds on it any more (`gpui-component` left it).
- `gpui-pre` is Zed's tree republished weekly (0.3.4 … 0.3.12, the last
  on 2026-11-09), with the platform crates split out: `gpui-pre-platform`
  (picks the platform per target), `-linux`, `-macos`, `-windows`,
  `-web`, `-ios`, `-wgpu`, `-apple`. There is no Android crate in it.
- Zed's own tree (main) has `gpui_ios` (Metal through the shared Apple
  renderer) and `gpui_web` (WebGPU, then WebGL2), Apache-2.0, and no
  Android platform.
- `gpui-component` (now in a repository renamed `longbridge/gpui-kit`)
  pins one snapshot exactly per release: 0.8.1 pins `=0.3.10`, 0.9.0
  `=0.3.11`, 0.9.1 `=0.3.12`.
- The community mobile layer `longbridge/gpui-mobile` (published as
  `gpui-pre-mobile`; its crates.io release 0.1.0 is from 2026-09 and
  targets 0.3.4, its main branch pins `=0.3.10`) is licensed
  `GPL-3.0-or-later OR AGPL-3.0-or-later OR Apache-2.0`: Vessel would
  take it under Apache-2.0. GitHub labels the repository AGPL-3.0 because
  of one of its three licence files; the choice is the crate's.

**Chosen for the app: `gpui-pre` 0.3.10 with `gpui-component` 0.8.1**,
because it is the newest snapshot that the component library and the
mobile layer's main branch both pin, so one GPUI can serve the desktop,
the phone (if that track finds it holds) and the browser. The spike's
one lock file resolves all of it; nothing of Zed's GPL crates is in it.

**Licences** (`cargo metadata --filter-platform`, the crates linked into
`vessel-app`, normal and build dependencies; 2026-11-13):

- Linux: 620 crates. All MIT, Apache-2.0, BSD-2/3-Clause, ISC, Zlib,
  Unicode-3.0, 0BSD, CC0-1.0, BSL-1.0 or Unlicense, alone or as a choice,
  except the ones below.
- Of concern, and why each is acceptable under ADR 0014 (Vessel links
  them into a binary it distributes under PolyForm Strict):
  - `option-ext` 0.2.0, MPL-2.0 (through `dirs`, already in Vessel):
    file-level copyleft; unmodified, its source is public; acceptable.
  - `self_cell` 1.3.0, `Apache-2.0 OR GPL-2.0-only`: taken as Apache-2.0.
  - `ring` 0.17, `Apache-2.0 AND ISC` (already in Vessel).
  - `notify`, `tiny-keccak`: CC0-1.0.
- Windows (x86_64-pc-windows-msvc): 530 crates; adds `dunce`
  (`CC0-1.0 OR MIT-0 OR Apache-2.0`). macOS (aarch64-apple-darwin): 528
  crates; adds `cbindgen` (MPL-2.0), a build tool, not linked.
- Fonts embedded in the binary: Geist and Geist Mono (Vercel) and
  Pretendard, SIL Open Font License 1.1, as the page bundles them; the
  tarball carries their licence texts.
- Nothing GPL-only, AGPL-only or of unknown licence is linked. Zed's own
  crates that are GPL (its `auto_update`, its editor) are not in the tree
  and no code was taken from them.

## Question 2: the Linux desktop

Run on this machine (no GPU, no display): Xvfb, Mesa 25.2's lavapipe
(Vulkan on the CPU), `import` to capture. GPUI's Linux platform drew
through wgpu on Vulkan, X11 (`x11: compositor present: false`).

- The screen in a window, light and dark: seen (captures above).
- Hangul drawn in Pretendard behind Geist (the kit gives every style
  `Pretendard` as its fallback; GPUI's Linux text system, cosmic-text,
  honours the list): seen in the sidebar, the composer and the
  conversation. The mono stack falls back to Pretendard too
  (`desktop-permission-light.png`, the prompt).
- Typing Hangul without an input method: characters sent as key events
  (`xdotool type`, one character at a time) arrive in the field, are
  drawn and are sent. A burst of `xdotool type` dropped characters; one
  at a time none were lost, so that is the tool's keymap race (it remaps
  one spare keycode per character), not the app's.
- **Typing Hangul through an input method (X11, XIM)**: fcitx5 5.1.7 with
  its Hangul engine (installed for this; see "Packages"), in a private
  D-Bus session and a scratch configuration, `XMODIFIERS=@im=fcitx`.
  GPUI connected to it (`zed_xim: XConnected`); Ctrl+Space turned the
  engine on (`fcitx5-remote`: 0 before, 2 after); the 2-set keys
  `g k s r m f` composed 한 and 글 and a space committed "한글" into the
  composer (`desktop-ime-committed.png`). **What does not work**: the
  syllable being composed is not drawn at the caret. fcitx drew it in
  its own popup at the screen's top-left corner
  (`desktop-ime-composing.png`: the field shows 한 while 글 sits at 0,0),
  so the caret's position does not reach the input method over XIM. A
  person typing Korean sees each syllable only when it is finished.
  Whether the cause is GPUI's X11 client or the component layer's input
  (which answers the caret's bounds) is not established yet; Wayland
  (text-input-v3) was not tried (no compositor here).

## Question 3: numbers

Not measured yet.

## Question 9: what goes wrong when GPUI is taken as it is

Each problem met, with what was run and what happened:

1. **The toolchain.** `gpui-pre` 0.3.10 does not build on Rust 1.93,
   Vessel's pin: `error[E0658]: use of unstable library feature
   cold_path` (`gpui-pre/src/profiler.rs:473`). It needs 1.95 or later;
   the app's workspace pins 1.98.1 (Zed's), so Vessel builds with two
   toolchains while the app is outside its workspace, and moving the app
   in means moving Vessel's toolchain with GPUI's.
2. **Headless without a word.** `gpui-pre-platform` has its `x11` and
   `wayland` features off by default. Built without them, the app ran,
   logged nothing wrong, opened no X window (`xwininfo -root -tree`: no
   children) and every capture was black: GPUI's Linux platform starts
   headless when no display server it was built for is named
   (`GraphicalEnvironment::guess_compositor`). Turning the features on
   fixed it. An error, or a default that draws, would have saved the
   hour.
3. **Snapshots pinned exactly, everywhere.** Each `gpui-component`
   release pins one weekly snapshot (`=0.3.10`, `=0.3.11`, `=0.3.12`),
   and so does the mobile layer, so a library can only be moved with all
   the others; the newest component release (0.9.1) cannot be used with
   the mobile layer's newest snapshot (0.3.10).
4. **The libraries' documentation is ahead of them.** `gpui-component`
   0.8.1's README documents `gpui_kit::*`, the umbrella crate of a later
   rename, not the crate it ships in; its text input lives in another
   crate (`gpui-base`, one 12,000-line file for the input's state).
   Working out the API took reading the sources.
5. **Windows shaders depend on the host** (read, not built; the Windows
   track has the details): `gpui-pre-windows`'s build script compiles its
   HLSL with `fxc.exe` only when the build script itself runs on Windows,
   and a debug build compiles them at run time from the crate's source
   path, so a debug `.exe` only runs where it was built.
6. **The window's title and activation.** `cx.activate(true)` logs
   "activate is not implemented on Linux"; no consequence seen.
7. **Composition is not drawn at the caret on X11** (question 2): with
   fcitx5 over XIM the syllable being composed appears in the input
   method's own popup at the screen's corner, not in the field.

What the screens needed of the UI layer, and where it came from:

| Need | From | State |
| --- | --- | --- |
| a list of thousands of rows of differing heights | GPUI's `list` (measured rows, only what is in view) | works; 20,000 not measured yet |
| a text field with input methods, selection, clipboard | `gpui-component`'s input, behind the kit's `TextField` | typed keys and XIM composition commit; the composing syllable is not shown at the caret (X11) |
| Markdown | `gpui-component`'s `TextView::markdown`, behind the kit | works |
| fonts and fallback | GPUI's text system, the faces embedded | works |
| a terminal's grid, menus, focus order, accessibility | — | not built yet |

## Build 0.0.0.0.2 (the owner's feedback on 0.0.0.0.1)

Commits on `backend/m12-spike`: `cc824a8` (layout, words, new chat,
pairing, title bar), `a8aac22` (the gate), `da7687a` (words per area, the
page's board tests), `cabf9d3` (arriving at a route the link changed),
`b1c9cbb` (the mark). Each seen against the fake fleet under Xvfb:

| What | Capture |
| --- | --- |
| The app opens on the composer, focused, under "What do you want to work on?"; machine, agent, folder in a tray under it; the app's own title bar and window controls | `desktop-draft-light.png`, `desktop-mark-light.png`, `desktop-mark-dark.png` |
| Hangul typed in the draft, Enter: within 0.4 s the conversation's screen with the prompt and "build-server에서 Claude Code 시작하는 중…"; then the session, titled from the prompt, its reply streaming, a permission, Stop and Queue (Korean, dark) | `desktop-draft-ko-typed.png`, `desktop-draft-ko-pending.png`, `desktop-draft-ko-open.png` |
| The same in English: the pending screen, then the opened session | `desktop-draft-sent-pending.png`, `desktop-draft-sent-open.png` |
| Pairing in two steps (Korean, light): the link's field focused with its placeholder; the key large with its one sentence and nothing to type; the name under a label that stays, filled with `machine`; the key typed as a name answered beside the field; paired as `home-pc`, the new chat next with that machine chosen | `desktop-pair-step1-ko.png`, `desktop-pair-step2-ko.png`, `desktop-pair-namekey-ko.png`, `desktop-pair-done-ko.png` |

Remembered between launches: `vessel/app-prefs.json` in the
configuration directory (machine, agent, the folder per machine), seen
written after a send.

### The new chat against the comparison's checklist

`refs/notes/new-session-flow-compare.md` section 4, item by item:

| Item | State |
| --- | --- |
| First screen | Changed as asked (the composer whenever a machine exists; pairing when none), not the page's welcome-then-board rule. The page must change the same way (ADR 0036). |
| Focus on open | Met. `Mod+L` not bound yet. |
| Machine | Partly: the last chosen (the app's own prefs), else the first connected. No address, caller or book default yet. |
| Agent | Partly: the pick, else the remembered one, else the machine's first agent that runs a conversation (the node's order, not the page's "first of the four"). |
| Profile and choices; the options probe | Partly (after 0.0.0.0.2): the options probe, the agent/model/effort control and the permissions control laid out by the page's rules (`view/src/options.rs`, the page's option tests ported), the last start's choices per machine and agent. No profile yet. |
| The spec | Partly: structured, `model: null`, `cwd` from the folder; no profile, no options; terminals not offered. |
| Place | Partly: a folder (home, typed, or one used before on that machine). Projects and worktrees are track F's, embedded where the folder chip is. |
| Send enabled | Partly: a machine, an agent and a prompt. |
| Keys | Enter sends. One line only: Shift+Enter, Mod+Enter and the send-key setting come with the multi-line composer. Whether Enter during a Hangul composition sends was not tested. |
| New-session shortcut | Not yet (Ctrl/Cmd+N and `N`). |
| Draft persistence | Not yet: the text is not kept across a restart. |
| Files on the first message | Not yet. |
| After a start | Partly: the choices remembered (in the app's own file, not the bridge's prefs: a second copy the checklist says to avoid; the app has no bridge, so where both clients keep them is open), the fleet read again, the session opened. |
| Failure wording | Partly: a refused start returns to the draft with the text restored; its words go to the frame's foot, not under the box yet. |
| Title | Met: the prompt's first line, 80 characters, until the node's own; a folder's name, else "Untitled session". |
| Follow-up delivery | Met for the default (queue); no modifier for the other way yet. |
| Shared logic | Ported, not shared: the board's logic with the page's test cases (`view/src/tests/`); the new chat's not yet. |

Noted, not changed: pairing over a relay sat on "Pairing…" for more than
eight seconds in the coordinator's run against the owner's node (direct
routes are tried first; a timeout per route is the likely cause, not
measured).

### The title bar (question 9 too)

The app hides the system's bar and draws its own (`kit/src/chrome.rs`):
on Windows its controls answer the system's hit test through GPUI's
`WindowControlArea` (drag, minimise, maximise with the snap layouts,
close), on Linux the app moves the window, opens the window menu and
resizes at the edges itself, on macOS the traffic lights sit in the bar.
Seen here: the bar and its three buttons drawn (Xvfb, no window manager,
so nothing to move, snap or resize). **Not verified**: anything on
Windows or macOS; the owner tries each build. Problems met: the
double-click handler GPUI's own elements lack comes from the component
layer (`gpui-base`'s `InteractiveElementExt`); a window icon can be set
only on X11 through GPUI (`WindowOptions::icon`).

The way back, until the bar is seen on each platform:
`VESSEL_APP_TITLEBAR=system` gives the window the system's bar, frame and
buttons, and the app draws none (`chrome::system_titlebar`, read once at
start; the release READMEs say how to set it). Seen here: with it, no
app-drawn bar (Xvfb has no window manager, so no system bar is drawn
either).

## After 0.0.0.0.2

The coordinator's three small changes for the next build, each seen
against the fake fleet:

| What | Capture |
| --- | --- |
| The prompt in the conversation in the interface face (Pretendard behind Geist), as the page draws it; monospace is for code and the terminal only | `b3-session-prompt.png` |
| `VESSEL_APP_TITLEBAR=system`: no app-drawn bar | `b3-system-titlebar.png` |
| Pairing a fresh app and keeping the filled-in `machine`: the book's entry is the node's own name, `laptop` | `b3-paired-own-name.png` |
| The new chat laid out as the page's: the agent, model and effort control ("Fake-small High") and the permissions control ("Default") in the box's foot, the machine and folder under it | `b3-draft-options.png` |
| The control opened: the agent, the model, the effort under its groups, the agent's other settings under "Claude Code 설정" | `b3-agent-panel.png` |
| Fake-large chosen and sent: the session starts with `model = fake-large` (the fake answers that model's way), and `app-prefs.json` keeps the choice for that machine and agent | `b3-sent-with-model.png` |

The node's name cannot fill the field: a pairing link carries the node's
id, not its name, and the node says its name only in the pairing answer.
So the app fills in `machine` as before, and when the person kept it, the
node's own name replaces it after pairing, made into a book name (a run of
other characters becomes one `-`, a letter or digit at each end, 64
characters at most) and only if the book does not have it; a name the
person typed is kept as typed.

## Updates, as Zed shows them (for 0.0.0.0.3)

The owner asked for automatic updates "as Zed has them". Track R's
updater does the work (`spikes/release/updater`, unchanged); the app shows
it (`app/src/update.rs`, `app/src/chrome.rs`, `app/src/areas/about.rs`):

- The title bar, at the right before the window's buttons: nothing while
  idle or up to date; "Checking for updates…" and "Downloading update…
  42%" in quiet type; "Restart to update" as a small button once the new
  build is in place, which starts it with the same arguments and quits; a
  failure as quiet words with "Try again", never a dialog. Below 768
  points wide each is an icon (the restart and the retry stay buttons).
  With `VESSEL_APP_TITLEBAR=system` the same status sits in the sidebar's
  foot. The bar's right slot occludes the bar behind it, so Windows' hit
  test should not read it as the drag area (not seen on Windows).
- About (the sidebar's foot, "Vessel <label>"): Paseo's settings layout
  and words since 0.0.0.0.6 (below): the version, what the updater last
  did, "확인", the update button, and "자동으로 업데이트" (on by default;
  kept in `app-prefs.json`; off, the updater's timer does not check and
  "확인" still does: track R's `Updater::set_automatic`).
- After an update, once: "Updated to <label>" with the feed's notes, at
  the top of the frame, dismissible. The notes are kept in the app's
  preferences when the build is ready (only the process that downloaded
  it knows them); the new build shows them when its label differs from
  the one that ran last, or when the updater restarted into it.

Seen on Linux against a local signed feed (a test key made with
`vessel-app-feed keygen` in a private scratch directory; two debug
builds labelled 0.0.0.0.91 and 0.0.0.0.92 with the feed's loopback
address and the test key's public half; 0.0.0.0.92 published with notes;
a loopback server that delays the channel's file 8 s and sends the build
at 10 MB/s, so each state lasts long enough to capture):

| What | Capture |
| --- | --- |
| Checking, five seconds after start | `u-checking.png` |
| Downloading, 15% | `u-downloading.png` |
| Ready: the button in the title bar; About with the same button | `u-ready.png`, `u-about-ready.png` |
| The button pressed: 0.0.0.0.92 opens with "0.0.0.0.92(으)로 업데이트했습니다" and the notes; the preferences then hold `last_label` 0.0.0.0.92 and no notes | `u-updated.png` |
| At 420 points wide: the icon while downloading, the filled icon button when ready | `u-phone-downloading-top.png`, `u-phone-ready-top.png` |
| The feed down: "업데이트하지 못했습니다 · 다시 시도"; About says why and checks again on the button | `u-failed.png`, `u-about-failed.png` |
| "Update automatically" off: no check in the first 12 s (the automatic one comes at 5 s); "Check for updates" checks once | `u-about-manual.png` |

Not seen: the preference switched while the app runs (it starts a new
updater with the other timing once the current one is not checking or
downloading); anything on Windows or macOS.

## Build 0.0.0.0.6 (the owner's feedback on 0.0.0.0.5)

The owner, on 0.0.0.0.5 on Windows with the real Claude Code: the model
selector and the other pickers looked wrong (Claude Code's `Agent` option,
values `ocx-*` with descriptions of several lines, ran into the rows under
them; three settings poured into one flat list), "the other settings" too,
and the version and update screen was "개판".

- A list row is Paseo's `ComboboxItem` (`components/ui/combobox.tsx`):
  the label, then the description on the same line, each one line
  (`numberOfLines={1}`); the description gives way first and the label
  never shrinks except to the row's width (a name longer than the row
  ends in an ellipsis); a row keeps its height and the list scrolls
  inside the panel's 400. GPUI starts a new line at every line break even
  where text does not wrap, so a row folds breaks into spaces first, as a
  browser does for `white-space: nowrap` (`kit::picker::one_line`).
  `kit::picker`'s test lays out rows with a subagent's description of
  several lines, a name longer than the list and Korean, and fails if a
  row grows past one line or runs into the next (it fails with the fold
  taken out).
- Features are Paseo's features sheet (`composer/agent-controls/index.tsx`
  `SheetFeatureItem`, shown when the toolbar aggregates features): a row
  per feature with its name and its value; a row opens its own list with
  the feature's name above it and a way back: the option names alone for
  a choice, "On" and "Off" for an on/off feature (Paseo's sheet has no
  switch there: its toggle row opens On and Off too); a choice goes back
  to the features. A feature with no name shows its id.
- About is Paseo's settings section (`screens/settings-screen.tsx`
  `AboutSection`, `DesktopAppUpdateRow`; `styles/settings.ts`) built from
  `kit::settings`: the page 16 around, 24 above, at most 720; the section
  title; one card with rows of title and hint at the left and the control
  at the right: "앱 버전 / 이 기기 / v0.0.0.0.6", "앱 업데이트" with
  Paseo's status words ("앱이 최신 상태입니다.", "업데이트 발견: v…. 다운로드
  중...", "업데이트 준비됨: v…", "앱을 업데이트하지 못했습니다." with the
  reason under it) and its two buttons ("확인" outline, "확인 중..." while
  checking; "업데이트", "v…(으)로 업데이트" once a build is in place),
  "자동으로 업데이트" with Paseo's switch, and "로그 파일" with its folder and
  "로그 열기" (track R's `vessel_app_log::dir()`).
  Differences, each with its reason: no "새로운 기능" row and no release
  channel (one feed, no changelog screen yet); the automatic-update row is
  Vessel's (Paseo has no such switch; Vessel's updater has the setting);
  the update button restarts without Paseo's confirm (the title bar's
  "Restart to update" never asked, and the button says what it does); the
  log row opens the folder and does not copy the path; the switch does not
  slide (Paseo's slides in 180 ms).
- A session's conversation and its composer share Paseo's chat column (at
  most 820, 16 from the edges), so the card and the messages start at the
  same left edge. Still different: the person's message is a full-width
  block (Paseo's is a bubble at the right), the conversation track's to
  take.
- In a provider's model list the check marks only the agent in use (the
  other agents' defaults are not a choice made here), as Paseo's.
- The blue ring in the owner's captures is not the app's: the captures
  are PNGs whose outer 10 pixels are fully transparent and whose ring is
  `#60A2E6`, a colour neither the app nor its libraries have; a window's
  own pixels are never transparent in a capture of it. It is the capture
  tool's or the chat's selection outline. No container in the app draws
  anything on focus.
- The fake fleet's Claude Code adapter sends those shapes
  (`FAKE_ACP_OPTIONS=real`), and `vessel-agent`'s
  `a_real_agents_shapes_arrive_whole` checks they reach the node whole.

Seen against the real preview node (paired as a test device from a
scratch config; no prompt sent; the device revoked after), dark, each
capture named for what it shows: the pairing
screen, every picker at every level for Claude Code, Codex and OpenCode
(the model selector with every agent, each agent's models scrolled, the
search across agents; 사고; 모드; 기능 and each feature's own list; the
machine; the folder and a typed path), About up to date, a session.

## Build 0.0.0.0.7 (the differences from Paseo left in 0.0.0.0.6)

In the order the owner meets them, each as Paseo has it:

- The composer's controls take the step of Paseo's ladder their width
  allows (`composer/agent-controls/layout.ts`, `app/src/areas/draft/density.rs`):
  `full` with carets, the thinking and mode values, and a control per
  feature carrying the feature's name and no glyph (an on/off feature its
  name, in Paseo's highlight colour when on and muted when off, one click
  switching it; a choice its name muted, then its value, opening its list
  with Paseo's search row); `condensed` without carets, thinking as its
  glyph, the features as one control (Paseo's default glyph, its only
  mark) opening the features sheet; `tight` the mode as its glyph too.
  Paseo's measures (a control 28, its glyph 16, a caret 14, 12 of
  hysteresis). Differences, each with its reason:
  - The floors are the words measured as the text system draws them
    (rounded up), where Paseo counts 7 a character: counted, the named
    features fitted on paper and were cut on screen.
  - No control's words are cut while a narrower step exists: the row
    drops a step the moment it is under the step's floor, and the
    hysteresis holds only on the way back up (a fuller step needs its floor
    and 12). Paseo keeps a step until 12 under its floor, cutting words
    meanwhile, which costs it little (its features are icons by then); here
    they carry words, and a row of ellipses is what the owner sees. Only
    `tight`, the last step, cuts the model's words.
  - A named feature control has no glyph: ACP sends no icon, and the one
    default glyph on every named control said nothing and took 20 a
    control (with it, Claude Code's row needed about 792 of the 764 it has
    in the widest card, so the owner never saw it full).
  Measured on the preview node: Claude Code (model, thinking, mode, three
  choices) is full from a window of 1118 (the widest card, from 1160, has
  42 to spare) and condensed under it; Codex (two choices) full from 1066.
  The features sheet is Paseo's `AdaptiveModalSheet` (a card on a wide
  window): a row per feature with its value, each opening its list below
  it with the name above. Not here: Paseo's two narrowest steps (thinking,
  mode and features folded into the model control's sheet; the model as
  its glyph alone).
- Every control in the composer's row has Paseo's tooltip
  (`components/ui/tooltip.tsx`, `kit/src/tooltip.rs`): shown at once, 8
  above the control and centred, Paseo's popover card, not on touch. Its
  words are Paseo's: "name: value" on a choice, the feature's description
  (else its name) on an on/off feature, "모델 변경", "사고 모드" and "모드
  변경" on the model, thinking and mode. Not here: Paseo's fade in.
- A list opens on the side its caller names, as Paseo's combobox (no flip
  on a wide window): the controls in the card above them, the machine and
  folder under the card below them, 5 from the trigger, at most 400 tall
  and never taller than the room on that side less 16.
- Adding a machine is Paseo's two modals over the new chat: "페어링 링크
  붙여넣기" (its helper, the field, Cancel and Pair) and "이 호스트에 연결할까요?"
  (what connecting allows; the host, its key fingerprint and its relay in a
  settings card; Cancel and Connect). Vessel's own in them: the warning to
  treat the link like a password (Paseo shows it where a link is made, not
  where it is pasted; the coordinator asked for it here, where the link is
  in the person's hands), and the name the book will use, in the second.
- About: Paseo's confirm ("데스크톱 업데이트 설치") before the update restarts
  the app; "새로운 소식" opening the running build's notes (the feed's,
  kept by the build that downloaded it; Paseo's lists every version from
  its website); "경로 복사" beside "로그 열기"; the switch's thumb slides in
  180 ms once switched (the track's colour changes at once).
- The sidebar's foot is Paseo's: a hairline over one line, Settings (a
  gear) at its right end, opening Settings (the settings track's frame,
  merged for this build). Its other icons (a project, usage, hosts, help)
  are not here.
- Settings shows only what this build draws and applies
  (`app/src/areas/settings/mod.rs`, `drawn` and `applied`): General,
  Appearance and About. A section nothing draws yet (Shortcuts,
  Notifications, Agents, Models, Machines, Projects, Devices, Sync) is not
  listed, rather than a page saying another track will fill it. A row whose
  value nothing reads yet is not shown: the composer's send shortcut and
  default send (so General has no select this build), the font sizes,
  content width, and the conversation's display (the conversation folds by
  its own setting, not yet wired to this row). Appearance is the theme;
  choosing one applies at once (unless `VESSEL_APP_THEME` pins it for a
  run).
- The agent's start-up log (an adapter's lines on its error stream before
  the session starts) is no longer lines in the conversation (track C's
  change, merged for this build): the app collapses tool calls until the
  conversation's display row is wired, and collapsed the log is not drawn;
  at "Full" (`VESSEL_APP_CONVERSATION=full`) it is one folded row, "Agent
  log · N lines".
- About writes a numbered build as Paseo writes a version ("v0.0.0.0.7"); a
  developer's own build, labelled "dev", says "dev" ("vdev" is no version).
- A button's label wider than the room the button has wraps inside it,
  centred, as Paseo's `Text` does; a button is never wider than its
  container (a permission's longest answer, "Never allow …", was cut by
  the panel on a phone). On the new chat at a phone's width the send
  button sits 6 inside the card's edge, Paseo's `buttonRow` bleed of 6
  past its 12 of padding.
- The native release profile has line tables (`debug = "line-tables-only"`);
  the Linux executable ships without them (split into `vessel-app.debug`,
  kept with the Windows .pdb): 73,265,496 bytes before, 71,425,872 after.

## Build 0.0.0.0.8 (settings made real; the effort control)

### Settings that were kept and not read

- The send shortcut is read by both composers (`kit::SendOn`): Enter (and
  Mod+Enter) sends and Shift+Enter starts a line, or Mod+Enter sends and
  Enter starts a line. The input engine keeps sending on Enter; in the
  second mode a plain Enter that reaches the field starts a line itself
  (the engine's own Enter-starts-a-line mode starts one on Mod+Enter too,
  before sending). The session's placeholder names the key in force.
- The interface and code sizes are one scale in the kit (`kit::fonts`):
  every sans step at the interface size over 14, code at the code size
  over 12, as the page's `appearance.css`; spacing and control heights
  stay. Every size not on a step goes through `fonts::ui` or
  `fonts::code`.
- Content width is the session's one chat column (the conversation and
  the composer under it): "Comfortable" is Paseo's 820 (the page takes
  agent-orchestrator's 768: the owner's rule is Paseo's app), "Wide" 1152,
  "Full" the window.
- Conversation display is wired to the conversation's folds: "Full" shows
  every tool call and an agent's start-up log as one folded row;
  "Collapse tool calls" folds runs and hides the log.
- The default send (a message sent during a turn) stays hidden until the
  conversation's queue is wired to it.

### The effort control (the owner, 2026-11-13)

The owner replaced 0.0.0.0.7's row of a control per option with another
product's composer control (their captures; a third party's screens,
described here and not kept in the repository):

- One control in the composer's row reads the model, the agent's name
  (quieter, when the machine has several agents), and the effort's word
  (violet at the top step), led by a lightning bolt when fast mode is on
  (Paseo's model glyph when it is off: the captures show it on only).
  Pressing it opens a card above it: at the left a bolt that switches
  fast mode (outline off, filled violet on); in the middle the step's
  word at 14/600 over the model's name at 12 with a chevron (a hover
  background; pressing it opens the model's list); at the right a reset
  (the agent's default); under them a slider of the agent's levels.
  (2026-11-14: the card and slider now follow the Codex app's values:
  254 wide, 4 in; see `backend-m12-effort-slider.md`.)
- The slider's steps are the agent's `thought_level` values in its
  order, without "default"; the last step is the agent's own "ultra"
  switch when it has one (Claude Code's `ultracode`: choosing it sends
  the highest level and the switch on, a lower step sends the switch
  off) or its own highest level (Codex's Ultra). The switches folded in
  are named by id in a typed table (`view::effort::ROLES`), read off the
  preview node through the app's log of each probe: Claude Code sends
  `effort`, `fast` (on|off) and `ultracode` (on|off); Codex sends
  `reasoning_effort` (low to ultra) and `fast-mode` (off|on). An agent
  the table does not know gets its levels only; an agent with no levels
  (OpenCode) has no card: its control opens the model's list.
- Colours and motion (superseded 2026-11-14 by the Codex app's model
  card, `backend-m12-effort-slider.md`): the fill is the accent at every
  level; the top stop (`Effort::maximum`) turns the word purple, reveals
  a moving purple field from the thumb with a burst, and fades the
  stops; Fast streams white dots back along the fill and slides the
  filled stops away. Logic and motion in `view::slider`, drawing in
  `kit::slider`; frames only while something moves.
- Every option the control does not drive (Claude Code's agent, Codex's
  collaboration mode, anything an agent sends) is a row at the top of
  the model's list, where the reference has its one switch: an on/off
  option a switch row, a choice its value opening its own list with the
  option's name above. Claude Code's "Agent" says what it is under its
  name.
- The row's ladder: full (carets, the agent's name, the effort's word, the
  mode's word), condensed (no carets nor agent's name), tight (the model
  cut at Paseo's 56, the mode as its glyph); no word is cut while a
  narrower step exists.

What the page needs to follow (ADR 0036, not built here): the same
control in `web/src/features/composer`: `view::effort`'s table and rules
ported to the page's `data/options.ts` (the same ids, the top step, the
reset, `drives`), a slider control in the page's kit with the same
values and keyboard (arrows, Home, End; one choice per drag), the card
anchored above the control, the remaining options at the top of the
model combobox, and the ladder's three steps with the same floors.

## Motion: one set of tokens, one held flag (0.0.0.0.15)

The owner's word 39 (2026-11-14): quick, fine transitions on every change
a person causes; Settings › Appearance › Animations on or off; off, the
system's reduced motion and a software renderer give the end state at
once; nothing animates continuously.

- Tokens: `vessel_app_kit::motion` (`QUICK` 150 ms for a control's own
  change, `STANDARD` 300 ms for a surface, `CURVE`
  `cubic-bezier(.23, 1, .32, 1)`, `ease`, `transition(length)`, `held`).
  Every track reads its lengths and curve there; the effort slider's
  model in `view` restates them and an app test pins the two together.
- One flag: `app::motion` keeps each reason motion is held (the system's
  preference, a software renderer, the Animations setting through
  `motion::animations`) and sets GPUI's `App::reduce_motion` to their
  "or". GPUI's `with_animation` draws a one-shot transition's end at
  once under it; hand-driven motion asks `motion::held`.
- The caret: gpui-base blinked it every 500 ms whatever the flag, and
  each blink redrew the window (31 % of a core at 1280 × 800, 62 % at
  1920 × 1200 on a software renderer, 0.0.0.0.14). gpui-base is now a
  patched copy (`spikes/release/patches/README.md`, change 4) whose
  caret is steady while the flag is set. Measures in
  `backend-m12-effort-slider.md`, "Known costs".

## Resizable side panels (the owner's word 29, 0.0.0.0.16)

Both side panels, the sidebar at the left and the code panel at the
right, are resized by dragging their inner edge, after Paseo's
`sidebar-resize-handle.tsx` and its layout files.

- One kit control (`vessel_app_kit::resize`): a 10-point strip
  straddling the edge with the column-resize pointer; after 100 ms of
  hovering a 1-point line in the ink at 25 %; a press starts the drag and
  a window-wide shield (a `popup::layer`) holds the resize pointer, takes
  every move and the release, and lets nothing under it be hovered or its
  text selected; a double-click returns to the default (an addition:
  Paseo has none).
- Widths (`vessel_app_view::panels`, under test): the asked width kept
  between the panel's least and most and never leaving what is beside it
  less than its own least; the least wins when the window is too narrow.
  Sidebar: 200 to 600, the centre keeps 400 (Paseo's), default 264
  (Paseo's 320; the shell was laid out at 264). Code panel: 320 to 1600,
  the conversation keeps 480, default 440 (Paseo's Explorer 240 / 360 /
  320; the panel shows a diff inside it).
- Kept in the preferences' layout (`sidebar_width`, `panel_width`, whole
  points, `None` for the default), written once when the drag ends, not
  per move (`app::areas::edge`). Settings' own list stands in the
  sidebar's place at the same width.
- Below the phone's width the sidebar is a sheet and the code panel the
  whole screen, as before: no handle.

### A narrow window with both panels (0.0.0.0.17)

The 0.0.0.0.16 smoke test: at 800 × 600 with the code panel open, the
sidebar at its default 264 left the conversation about 190 points, and
dragged to 360 about 110 (one character a line, the composer unusable).
Each panel kept its own least (sidebar 200, code panel 320) and was sized
alone: the sidebar against the window less 400, the code panel against
its row less 480, so together they took more than the window held.

What Paseo does (`packages/app/src`):

- `components/desktop-sidebar-layout.ts`: the sidebar is drawn beside
  the content only when `viewportWidth - sidebarWidth >=
  contentMinimumWidth` (`canDesktopAppSidebarShare`), and the content's
  minimum is `EXPLORER_WORKSPACE_MIN_WIDTH` while the explorer is open
  (`resolveDesktopAppContentMinimum`); a panel is never wider than
  `viewportWidth - MIN_DESKTOP_CENTER_WIDTH` (400).
- `components/explorer-sidebar-layout.ts`: the explorer is kept between
  240 and `containerWidth - MIN_WORKSPACE_BODY_WIDTH` (360), and
  `EXPLORER_WORKSPACE_MIN_WIDTH = MIN_EXPLORER_SIDEBAR_WIDTH +
  MIN_WORKSPACE_BODY_WIDTH`.
- `screens/workspace/workspace-screen.tsx:3635`: `presentation={isMobile
  ? "overlay" : "dock"}`: on the compact layout the explorer is drawn
  over the workspace (`MobilePanelOverlay`), not beside it.

The change:

1. The panels give way before the conversation does: the sidebar's room
   counts the code panel's least when the panel is beside the
   conversation (what lies beside the sidebar is then the panel's least
   and the conversation's), so the sidebar shrinks to its least before
   the conversation goes under 480; the code panel is then held to its
   row less 480, as today.
2. Under the width where the panel's least and the conversation's least
   no longer fit in the row beside the sidebar at its least, the code
   panel is drawn over the conversation at the right (its stored width,
   held to the row), not beside it, as Paseo's explorer on its compact
   layout; the conversation keeps the whole row under it.
3. Stored widths are not rewritten: they apply again when the window
   grows.

Done in `e60165f3` (`view::panels::code_placement`, `sidebar_leaves`,
`over_width`, `Limits::width_leaving`; the sidebar's frame and the code
panel's frame in the app). Evidence: the rule's unit test
(`a_narrow_window_keeps_the_conversation_s_least`, with the smoke test's
numbers) and captures with both panels stored at their widest (sidebar
600, code panel 1600), both themes:

- 800 × 600: the panel over the conversation, the sidebar down to 308
  (before: the conversation about 70 points wide between them).
- 1024 × 700: both beside the conversation, the sidebar at 212, the
  conversation at its 480, the panel at its least 320.

Over the conversation (0.0.0.0.18, the coordinator's note on the 800 ×
600 capture: nothing said a conversation was underneath): Paseo's compact
panel has no title or back button of its own; it dims what it covers
(`mobile-panels/presentation.tsx`: a backdrop at black 50 %) and a press
on that backdrop returns to the agent (`onPress={showMobileAgent}`). The
same here: the conversation under the panel is dimmed and a press on it
closes the panel. A panel stored as wide as the row covers it all, as
Paseo's full-width panel does; its close button is then the way back.

## The node line in the app line, and seams for the packages (0.0.0.0.19, 0.0.0.0.20)

The app line's crates were 246 commits behind the node line (protocol 15
against 23), so the app could not call inbox, push, usage, side questions,
agent history or `device.self`. B0 merged the node line into the app line
(`07feb4ee`: node `d954fa3a`, the workspace stack `c408b653`, the app's
`1d1f11df`; on conflict the node's crates, web and root rules, the app's
`spikes/native`). The lead then built the package base every package
starts from, gated alone:

- `8d5513e4`: moves only. `LinkError` to `link/src/error.rs` and
  `Shell::on_event` to `app/src/shell/events.rs` (G2), `State::options`
  to `link/src/options.rs` (G3), and the link's crate root split so it
  leaves the size list: `Command` (`command.rs`), `Event` and `KeptLists`
  (`event.rs`), the actor's dispatch (`dispatch.rs`) and its refresh and
  connection (`refresh.rs`). Every package registers a command, an event
  and a dispatch arm, and `lib.rs` was a listed file held at its measured
  size, so each package's first registration failed the size check.
- `6036eeb8`: `view::Machine::node`, the node that answered under a book
  name (a name can be renamed or reused, so a queued request checks the
  node, not the name), and the kit field's events and `Editability` (W4),
  one shared editor policy instead of three copies.
- `6c782cb6` (B0, package base 3): `6036eeb8` with the node line's
  Quality B and a receipt fix. The native sources are unchanged; it was
  merged into the app line only after the release worker had taken the
  0.0.0.0.20 commit, so the release does not carry crates it was not
  gated with.
- `3cd0d3fc` (package base 4): from here the package base and the app
  line are one line. The .19 and .20 merges had gone only into the app
  line, so workers on base 3 lacked them (one was blocked on the Settings
  Chat words). Every package merged for a build now lands on this line,
  and the package base branch follows it by fast-forward.
- `074ade16`: the dialog's shared return focus, taken early as a seam so
  the packages that open dialogs, menus and selects reuse one policy: G4's
  captured dismissal (`a80e7fd3`, the close is a clonable closure) and its
  weak, once-per-open return to the opener (`fe53a206`, `42bde0bf`,
  `11fba709`), picked alone rather than with G4's moving branch. The focus
  commits need the closure commit to compile, and the reviewed tests had
  one doubled blank line that the format step refuses.

Root binaries for capture fleets are built once from that root
(`fleet.sh` end to end), so the packages do not each build them.

Two findings of the coordinator's smoke tests on 0.0.0.0.18 and .19:

- The grey ring beside Send in an ended session is the context meter with
  no data: Paseo draws its empty track as a closed circle. Ours was an
  open 48-piece polyline with a notch at twelve, then a closed one that
  stroked the joint twice on a translucent track; the whole ring now
  drops its repeated point and closes.
- The `=` in the timeline's gutter at 800 x 600 is the chat outline rail,
  which Paseo shows only in a panel at least 918 wide. Ours decided from
  the list's width at its last layout inside the cached timeline region,
  so after the window narrowed it was not drawn again (the rule added to
  the README's regions paragraph).

## Packages installed on the machine

- `fcitx5` and `fcitx5-hangul` (with their 12 dependencies: `fcitx5-data`,
  `fcitx5-modules`, `libfcitx5utils2`, `libfcitx5config6`,
  `libfcitx5core7`, `libhangul1`, `libhangul-data`, `libxcb-ewmh2`,
  `libxcb-icccm4`, `libxcb-imdkit1`, `libxcb-keysyms1`,
  `libxcb-xinerama0`): an input method with a Hangul engine, to test
  composition (question 2). Run only by the test, under its own Xvfb and
  D-Bus session; no service enabled. `fonttools` (for nothing in the end) went into a scratch
virtual environment in a local folder, not committed; `geist` was fetched with
`npm pack` into a scratch directory by `scripts/fonts.sh`.
