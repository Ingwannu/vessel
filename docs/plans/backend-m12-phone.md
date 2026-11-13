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

## Problems met

- The first build failed: the mobile layer's directory has no
  `rust-toolchain.toml`, so rustup's default (1.97.1) built it, and the
  Android targets had been added to 1.93 only. Fixed by adding them to
  1.97.1 too.
- The emulator's System UI showed "isn't responding" once after boot
  (SwiftShader, cold boot); not the app's.
- Disk contention: Gradle took 5 to 12 minutes per APK and cargo waited
  on other workers' locks while seven workers built on the same disk.
