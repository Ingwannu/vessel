# Milestone 12, track M: the phone

Notes and evidence of the phone track (`backend/m12-phone`): what was
run, what it printed, what was seen, and what is not verified. The plan
is `docs/plans/backend-m12.md`; this answers its questions 5 (Android),
10 (Tauri 2), 7 (iOS) and 6 (browser), and records the phone app built
meanwhile (`spikes/phone/shell`).

Captures and logs named here are kept outside the repository by the
coordinator (`evidence/` beside the build outputs); they are named so a
reader can ask for them.

## Toolchain

What was there: an Android SDK with platform 34, build-tools 34, the
emulator, a `google_apis` x86_64 system image for Android 14 (API 34),
an AVD (`nh34`, a Pixel 5 profile, 1080x2340 at 440 dpi, software
keyboard) and a Temurin JDK 17. KVM is available.

What was added, outside the home directory:

- the NDK r27d (27.3.13750724), unpacked from Google's zip;
- `cargo-ndk` 4.1.2 (`cargo install --root`);
- Gradle's own downloads (`GRADLE_USER_HOME`): Gradle 9.4.1 for the
  mobile layer's example, the Android Gradle plugin, and build-tools 36,
  which that plugin installed into the SDK by itself;
- the Rust targets `aarch64-linux-android` and `x86_64-linux-android`
  (rustup, for the repository's 1.93 and the mobile layer's 1.97).

The emulator is started as its own instance (`-port 5600 -read-only
-no-window -gpu swiftshader_indirect -memory 4096`), so it changes
nothing in the AVD and stops by its own pid. There is no GPU on this
machine: every frame below is drawn by SwiftShader on the CPU, so what is
seen says whether something draws, not how fast.

## Question 5: native GPUI on Android

The community mobile layer, `longbridge/gpui-mobile` (crate
`gpui-pre-mobile`, licence GPL-3.0-or-later OR AGPL-3.0-or-later OR
Apache-2.0, so usable under Apache-2.0), at commit `7548ebc` ("Upgrade
gpui-pre to 0.3.10"), on the `gpui-pre` 0.3.10 snapshot, the same the
desktop track pins. Its own example app, built for the emulator's ABI
(x86_64, which its README calls untested) with `cargo ndk -t x86_64
--platform 31 build --release` and packaged by its Gradle project
(`assembleDebug`). Two lines changed in a scratch copy only: `x86_64` in
the example's `abiFilters`, and (below) a way to open its AI chat screen
on Android.

| Asked | Seen | Evidence |
|---|---|---|
| Does a window draw | Yes. wgpu found two adapters, tested SwiftShader's Vulkan, chose it ("GPU outcome: Vulkan"); the home screen drew in about 2 s from launch, system fonts and the bundled colour emoji loaded. | `gpui/01-launch.png`, logcat |
| Touch scrolling | Yes, with momentum: a 250 ms swipe moved the home list by about two screens' worth. | `gpui/02-home.png`, `gpui/03-after-swipe.png` |
| Soft keyboard | Shows on a tap in a text field, with a caret; Gboard got `inputType=0x90001` (text, multi-line, no suggestions). | `gpui/04-keyboard.png` |
| Hangul, committed | Typed with Gboard's Korean 2-Bulsik layout: the committed word `한글` is drawn correctly (Noto Sans CJK from the system). | `gpui/08-after-space.png` |
| Hangul, composition | Works through GPUI's input handler (GPUI Kit's `Input`); not in the example's own form fields, which take committed text only. See "Composition". | `gpui/16-chat-composing-han.png`, `gpui/17-chat-composing-hangeul.png`, `gpui/18-chat-committed.png`, `gpui/19-chat-backspace-in-composition.png`; the form: `gpui/06-…`, `gpui/07-…` |
| Rotate | The window resized in the same process (`AndroidWindow resize: 1080×2340 → 2340×1080` and back), the layout followed, state kept. | `gpui/09-landscape.png`, `gpui/10-portrait-again.png` |
| Background, foreground | Home and back: Pause, LostFocus, TerminateWindow ("surface unconfigured, renderer kept alive"), Stop; then Start, Resume, InitWindow ("replaced surface"). Same process, screen and scroll position as left. | `gpui/11-foreground-again.png`, logcat |

### Composition

The layer has a real composition path. Its Java host
(`GpuiInputActivity`, which the example's activity extends) keeps an
invisible `EditText` as the IME's target, and after each batch of edits
sends the editor's text to Rust with a kind: 0 composing, 1 committed
(`nativeIme`). On the Rust side (`src/android/text_input.rs`), kind 0
becomes GPUI's `replace_and_mark_text_in_range` on the focused input's
`PlatformInputHandler`, kind 1 `replace_text_in_range` and
`unmark_text`; Back finishes a composition without sending Escape.

What was seen:

1. **The example's form fields show no composition.** Typing ㅎ ㅏ ㄴ ㄱ
   ㅡ ㄹ, Gboard composed `한글` (its suggestion strip says so) and the
   form re-rendered after each key, but the field showed nothing until
   Space committed the word. Cause, in the example, not the layer: the
   form's fields (`example/src/screens/form.rs`) take text through the
   layer's older callback (`set_text_input_callback`), which receives
   committed text only; `forward_legacy_text` drops kind 0 by design.
   Any GPUI input written against GPUI's own input handler
   (`EntityInputHandler`, as `gpui-component`'s `Input` and GPUI Kit's
   are) takes the kind-0 path instead.
2. **GPUI Kit's `Input` on Android composes Hangul.** The example's AI
   chat screen uses GPUI Kit's `Input` (an `EntityInputHandler`, as
   `gpui-component`'s is) but is wired only into the iOS entry; a patch
   in the scratch copy opens it on Android for `gpui://ai_chat` and puts
   its composer at the top (point 3). With the same keys: `한` drawn
   underlined while composing, then `한글` underlined, then, after Space,
   `한글 ` committed without the underline; the send button enabled
   itself as the value became non-empty. Backspace inside a composition
   removed one jamo (`못` → `모`, still marked); Backspace after it
   deleted committed text. So composition holds through GPUI's own input
   path on Android, with Gboard's Korean layout, in the emulator.
   One detail of the host: the hidden editor is cleared only when a
   composition ends, so Gboard's marked range spans the whole word being
   typed, not the syllable (both syllables of `한글` were underlined);
   that is what Gboard does in a native field too.
3. **The keyboard covers a field at the bottom.** The layer's
   `keyboard_height()` (`src/lib.rs`) is set only by the iOS window
   (`src/ios/window.rs`); nothing on Android reports the IME's inset, and
   a `NativeActivity`'s surface is not resized by `adjustResize`. The AI
   chat's composer, at the bottom, was hidden behind the keyboard while
   typing (`gpui/13-chat-keyboard.png`). The fix is small and on the
   layer's side: read the inset (`android-activity`'s
   `MainEvent::ContentRectChanged` / `content_rect()`, or a
   `WindowInsets` listener in `GpuiInputActivity` reading
   `WindowInsets.Type.ime()`), and feed `set_keyboard_height`, so a view
   can pad its bottom.

### Known not to work, from its code

- **An activity made again on the `NativeActivity` path** (Back, then
  the launcher): the layer's own notes (`src/android/host.rs`) say a
  recreated activity runs on a new thread while the dispatcher kept the
  old one's looper, so GPUI asserts "must construct App on main thread"
  and the screen stays black. Its fix is the "host-driven" entry (a
  plain Activity with a `SurfaceView`, GPUI on a process-lived render
  thread), which the example does not use. An app of Vessel's would use
  the host-driven entry.

## The Tauri 2 shell (question 10's fallback): `spikes/phone/shell`

ADR 0036 makes this the fallback, shipped only where the native phone app
cannot be. It is built and passes its emulator test; build `0.0.0.0.1`
of it is the first Android build given out.

### What it is

- `core/` (`vessel-phone-core`, no Tauri type): `vessel-web`'s server and
  bridge started in the app's process on a loopback port the system
  picks (`Bridge`), and the app's own nodes (`PhoneSource`, the bridge's
  `NodeSource`): a book in the CLI's format under the app's private
  files, device credentials beside it (0600 in a 0700 directory),
  pairing by link with `vessel-client`'s `pair_over`, direct and paired
  (relay) entries reached through `Pinned` connectors, SSH entries
  refused with a reason, no "this machine". The first node paired becomes
  the default. Root certificates for `wss://` relays are read from
  Android's own store (`Roots`; `rustls-native-certs` knows none there).
- `app/` (`vessel-phone`, Tauri 2.12): one window at the app's start page
  (`dist/index.html`, in the APK); each time that page loads, a new
  launch of the bridge is started and the web view sent there
  (`launch.rs`, which says why). The web view may navigate only to the
  start page and the current launch's origin; the page's own CSP keeps its
  requests there. The build's label is shown for four seconds over the
  page (an initialisation script; Pretendard first) and is Android's
  `versionName`; the version code is the label's last number.
- `app/gen/android/`: Tauri's generated Gradle host, changed: no backup and
  no device transfer of the app's files, cleartext allowed for loopback
  names only (`network_security_config.xml`), the content padded by the
  system bars, the cutout and the keyboard, a warning for an Android
  System WebView older than 113, the web inspector only in a build made
  with `VESSEL_WEB_INSPECTOR=1`, the label's version fields.
- `scripts/`: `icons.sh` (the launcher icon from the brand mark, at build
  time, adaptive with a monochrome layer), `release-apk.sh` (align, sign
  with the one key, verify, `SHA256SUMS`), `test-fleet.sh` (a relay and a
  testkit node on the host for the emulator test).

### What the web view is allowed

No Tauri command. `tauri.conf.json` lists no capability and there is no
`capabilities/` directory, so every `plugin:` command (Tauri's core ones
included) is refused, and from the bridge's origin (remote, to Tauri)
everything is; the app defines no command of its own. Tauri still injects
its IPC script into every page (read from its source); without a
capability it reaches nothing. The only Android permission is
`INTERNET` (sockets, the loopback bridge's included).

### The emulator test (`evidence/shell/`)

Against a testkit node registered at a relay of its own on the host, the
relay's port forwarded into the emulator (`adb reverse`), the link naming
only the relay (`test-fleet.sh`):

| Step | Seen |
|---|---|
| The page loads | Yes, after two fixes below. The label pill shows. |
| Pairing by link | The link pasted, the machine's key, id and "Through the relay" shown, named, connected: "desk is ready · Paired · Connected (test-node · Linux x86_64) · Agents found". |
| Remembered | The machine and its sessions are there after the app is stopped and started again. |
| A session | Opened from the board; its conversation grew: the prompt, the thought, `echo: 한글 안녕`, a tool call, the plan, a permission asked; Allow completed the call and ended the turn; a second prompt (`plain`) answered. |
| Hangul in the composer | Gboard's Korean layout composed `한글`, then `한글 안녕`, underlined while composing, into the composer's text area; the value sent was `한글 안녕` (`08-composing-hangeul.png`, `09-composed.png`). |
| Keyboard | After the insets fix the composer sits above the keyboard (`15-keyboard-clear-of-composer.png`). |
| Rotate | The page re-laid out in landscape (its desktop rail), same process, nothing reloaded (`17-landscape.png`). |
| Background, foreground | Home and back: the same page, no new launch (`18-foreground-again.png`). |
| Widths | 360x800 and 412x915 (`wm size` at 440 dpi): the page's narrow layout, the sidebar as a drawer (`w360x800-*.png`, `w412x915-*.png`). |

Not verified: a real phone; a `wss://` relay (the test relay is `ws://`
on loopback; the roots code is unit-tested with a public root); QR codes
(none: a link is pasted); an activity made again by Android in a living
process (the start-page design is for it; not provoked in the emulator).

### What went wrong on the way, and the fixes

1. **A white page**: the release build refused the bridge's address,
   `net::ERR_CLEARTEXT_NOT_PERMITTED` (read over the web inspector).
   Tauri's template allows cleartext in debug builds only. Fixed with a
   network security config allowing cleartext for `localhost` and its
   subdomains and `127.0.0.1` only.
2. **The session screen failed** ("Something went wrong!"): `TypeError:
   Cannot assign to read only property 'toString'` in the session chunk.
   Cause: Tauri's `freezePrototype` (set in the first configuration)
   freezes `Object.prototype`, and a library of the page assigns
   `toString` on objects (the "override mistake"). Not the WebView's
   age. Removed; the page is not written for a frozen prototype.
3. **"The UI was restarted or its identity could not be verified"**: on
   one cold start the web view loaded the launch address more than once
   and the page was refused (the bootstrap is single-use). Fixed by the
   start page and a launch per load of it (`launch.rs`).
4. **The header under the status bar, the keyboard over the composer and
   the dialogs' buttons**: Tauri's activity draws edge to edge (Android
   15 imposes it on apps that target it) and the page knows no inset.
   Fixed in the activity: the content is padded by the system bars, the
   cutout and the keyboard.
5. `adb shell input text` dropped characters of a 328-character link and
   Gboard's Korean layout turned it into jamo: the test pastes the link
   through the inspector instead (as a person pastes it).

The emulator's Android System WebView is 113 (no Play updates on this
image). The page works on it once the above is fixed: nothing it needs
is newer. The shell warns below 113 (the oldest seen working; older is
not tried).

### What the page does badly at a phone's width, most harmful first

For the page's owners; not fixed here (the shell's insets fix is the only
change made for the phone).

1. **Touch targets**: the sidebar's rows, the composer's model and
   permission pickers and the timeline's disclosure rows are well under
   44 points (about 28 to 32).
2. **Landscape**: the composer takes most of the height; about two lines
   of the conversation stay visible above it.
3. **Keyboard hints on a touch screen**: "Ctrl+K", "N" beside Search and
   New session.
4. **The add-machine dialog** tells the person to run a command "on the
   other machine" and paste the link: fine on a phone, but there is no way
   to scan the QR code the CLI prints.
5. **The link field** has `autocomplete="off"` and `spellcheck="false"`
   but no `autocapitalize="off"`/`autocorrect="off"`; a typed link can be
   capitalised by the keyboard (a pasted one is not).
6. **Untitled sessions** both read "한글 안녕" in the sidebar: the first
   prompt is the title, so two sessions from one prompt cannot be told
   apart on a narrow sidebar without their machine and age.

## Problems met

- The first build failed: the mobile layer's directory has no
  `rust-toolchain.toml`, so rustup's default (1.97.1) built it, and the
  Android targets had been added to 1.93 only. Fixed by adding them to
  1.97.1 too.
- The emulator's System UI showed "isn't responding" once after boot
  (SwiftShader, cold boot); not the app's.
- Disk contention: Gradle took 5 to 12 minutes per APK and cargo waited
  on other workers' locks while seven workers built on the same disk.
