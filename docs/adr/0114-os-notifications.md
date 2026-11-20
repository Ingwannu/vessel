# ADR 0114: OS notifications and attention rules, app and page

Status: accepted by the coordinator on 2026-11-19 as client-side only: no node or protocol change. impl U.
TRACKER J12-J35, J54-J55; related A17, A61, G48, H49, H113. Paseo reference 0.15.4. The claims follow-up
(ADR 0089, rebased onto the node line) is TRACKER 815.

## Context

Paseo tells the person when an agent needs them or finishes:
- An OS notification is shown on one chosen client (Electron `Notification`; web `Notification`).
- The sound is the OS's own, behind the "Play sound" switch.
- A click focuses the window and opens the agent.
- A desktop-only Settings › Notifications section has a permission row, Play sound and Test notification.

The rules are:
- Paseo never shows an OS notification for an error.
- It shows none when the app is actively visible and the person is on that agent.
- It dedupes each agent by attention timestamp.
- The daemon elects one recipient from the clients' 15 s heartbeats (180 s presence).

Vessel today:
- **Page:** a Notifications section already exists. It offers four modes (off, notifications, sound, both;
  default off) and a count in the tab title. It shows `Notification`s with a click to the session, plays
  two drawn Web Audio tones, and asks the browser for permission when a mode is chosen. Its source is a
  diff of the 2 s session poll, and it notifies only while the page is hidden or unfocused.
- **Native:**
  - It has the same preferences (`Notifications { alerts, title }`, shared with the page through the
    prefs contract).
  - It has the pure diff (`view/src/notices.rs`).
  - Its patched GPUI has a system notification API: `show_system_notification`,
    `dismiss_system_notification`, `on_system_notification_response` and `set_app_identity`. Linux goes
    through notify-rust over D-Bus and Windows through WinRT toasts. Both are silent: Windows sets
    `<audio silent>`, and Linux sends no sound hint. The patches have no macOS implementation.
  - Nothing calls that API. The Notifications section is declared but not drawn, and there is no audio
    backend.
- **Node:**
  - ADR 0089 (one connected recipient by `notification.claim`, and terminal attention) is only on the
    staging line `backend/m12-wp-inbox-node`, at protocol 25. There it has five methods, their handlers,
    a store and tests.
  - `backend/m12-node` (protocol 33) carries only its types. `ATTENTION_NOTIFICATIONS_PROTOCOL` is
    `None` and no method is registered.
  - `push.presence` and the recipient policy (protocol 22) exist, but only the Web Push worker reads
    them, and neither the page nor native sends presence.

## Decision

[Decision Log]

**Purpose and intent.** The app shows the system's notifications with sound and click-through, and the
page follows the same attention rules. Settings › Notifications is drawn on native. All 26 rows move from
none to done, or to a stated difference, without waiting on the node.

**Existing implementation and constraints.**
- Neither client may claim one recipient until ADR 0089's line is merged and its protocol assigned. That
  is not this row's to do.
- The GPUI notifications are silent on both platforms. The shared Alerts preference has a "Sound only"
  mode that shows no banner, so the sound cannot come from the OS.
- The native app also builds for wasm32, so anything platform-only is cfg-gated.
- Gates are Linux only; Windows is checked on the operator's PC.

**Alternatives considered.**
1. Wait for ADR 0089 (claims) and build everything on it. This is blocked on a merge and a protocol
   assignment that this row does not own, and every row stays none meanwhile.
2. Client-side attention now (the page's diff, already in `view/src/notices.rs`), with Paseo's rules
   applied in each client, and claims added later as a second row. **Chosen.**
3. The sound from the OS notification, by adding a sound field to the GPUI patch (a Linux `sound-name`
   hint, Windows toast audio). This cannot do "Sound only", it touches the release track's patch, and
   Linux daemons differ in what they play.
4. The sound by the app (chosen):
   - `cpal` (chosen, after a build probe on the gate VM): small; ALSA on Linux (which also reaches
     PipeWire and Pulse), WASAPI on Windows.
   - `rodio`: brings decoders the drawn tones do not need.
   - Running `paplay`/`pw-play`: depends on tools a desktop may lack, and is not available on Windows.

**Chosen approach.**
- **Source.** Every fleet update runs `notices(before, after)`, as the page does. A session that starts
  to need the person (a permission or a question) or that stops working is a notice. The first sighting
  says nothing.
- **Rules**, for both the app and the page, as Paseo's `notifyAgentAttention`:
  - A failure is never an OS notification. It stays in the inbox and the badges.
  - A notice is not shown when the app is actively visible (the window is active, or the page is visible
    and focused) and the target session is the one shown. Otherwise it is shown, including while the app
    is in front on another session. The page today shows notices only while it is away; it changes to
    this rule.
  - One notice per transition. The same tag (machine, session, kind) replaces an earlier notification
    (Windows, Linux, the page's browsers).
  - The window or tab title counts the notices since the app was last active, when the title switch is
    on.
- **Native delivery:**
  - `set_app_identity` at startup.
  - `show_system_notification` with an opaque tag, kept in a bounded in-process map from tag to
    (machine, session).
  - A click activates the window and opens the session (`open_session`). A stale or unknown tag only
    activates the window.
  - Text is the page's: "Session needs you" or "Session finished", with the session's name as the body.
- **Sound.** The page's two drawn tones, with the same frequencies, lengths and gain, synthesised in
  `view` (pure and tested) and played by `cpal` in `app` (not on wasm32). They play for the "Sound only"
  and "Notifications with sound" modes.
- **Settings › Notifications (native):**
  - "Session notifications", the four modes.
  - "Count in the window's title".
  - "System notifications", saying where they go: the desktop's notification service on Linux, Windows
    notifications on Windows, and "not available in this build" elsewhere. Paseo's permission row has
    nothing to ask on Linux or Windows.
  - "Test notification" with Send, showing the page's test text and playing the needs-you tone. The result
    is Paseo's inline Alert under the section:
    - success: "Test notification sent" / "Vessel handed the notification to the operating system.";
    - error: "Unable to send test notification" and why. Only what is known before sending can fail: a
      build without system notifications, or Linux without a session bus. GPUI's call returns nothing.
- **Claims later.** A follow-up row adopts ADR 0089 once its protocol is assigned on the node line:
  `push.presence` heartbeats, `notification.claim` before showing, and terminal attention (G48).

**Why this over the other alternatives.**
- Every client-side piece is needed in the claim design too: the rules, the delivery, the sound, the
  settings and the click routing. Claims only add a gate before `show`, so nothing built now is thrown
  away.
- The node and the protocol are not touched, so this row does not wait on the integrator.

**Pros, cons and impact.**
- Pros:
  - The rows are done on both clients.
  - One rule set across both clients, matching Paseo.
  - No protocol change.
- Cons, as stated differences until the claims row:
  - Two open clients may both notify, where Paseo elects one.
  - There are no terminal notifications.
  - The OS notification text is generic: the session's name, not Paseo's last-message preview. Native
    fleet cards do not carry the last message, and ADR 0089 keeps OS text free of content.
  - Mobile push (J30-J35) does not apply: Vessel has no mobile app, and the page's Web Push is ADR 0041's.
  - macOS shows no system notification until the GPUI patch has one, and plays no sound (link has no
    audio output there); its test is the error Alert with that reason.
- Impact:
  - New dependency `cpal`, native only. Linux builds need `libasound2-dev`, and Linux runs need
    `libasound.so.2`; the release packages must name it. No change on wasm32.
  - The page's rule changes: it now also notifies while visible on another session.

## As built (native)

| What | Where |
|---|---|
| The rules | `view/src/notices/announce.rs` (`Seen`, `announce`, `Unseen`) |
| The diff, and the failure rule | `view/src/notices.rs`: a session whose process failed is not announced |
| The tones | `view/src/notices/tone.rs` (`samples`) |
| The player | `link/src/sound.rs` (`sound::play`), with a stub on other systems in `link/src/lib.rs` |
| Delivery, click and title | `app/src/areas/notify.rs` |
| The settings page | `app/src/areas/settings/notifications.rs`; the select is `choices::List::Alerts` |
| The words | `view/locales/{en,ko}/notifications.ftl` |

- **Identity.** `set_app_identity` runs before the window opens. Labelled builds are `Vessel.App` and a dev
  build is `Vessel.App.Dev`, so a dev build's notifications never replace the installed app's. The
  Windows patch registers the id under `HKCU\Software\Classes\AppUserModelId`.
- **Tags.** Each tag is `vessel-<random run id>-<n>`. It carries no machine, session or path. The last 64
  tags are mapped in the process. The same (machine, session, kind) reuses its tag. Windows replaces
  the earlier toast by tag. On Linux the GPUI patch passes the tag's id as `replaces_id` and dismiss
  calls `CloseNotification` (patch 16 in spikes/release/patches, TRACKER 818, 2026-11-19). Before it,
  the D-Bus record of the frames showed a second notification for the same tag. A click on a known
  tag activates the window and opens the session;
  an unknown tag only activates the window.
- **Sound.** One sound per change of the fleet: needs-you if any, else finished. Ten sessions ending in
  one read sound once. The page plays one per notice, which is a stated difference.
- **Title.** Render writes the title from a value the fleet event leaves. The event has no window, and
  writing from there would borrow the window inside its own update.
- **Dependency.** `cpal =0.16.0`, the voice track's exact pin, with lock entries taken from its lock. It
  is owned by link on Linux and Windows.
- **Browser link.** `web.rs` `answer` is split by kind of request into `web/refuse.rs`. A source test
  (`link/src/web_dispatch_tests.rs`) keeps every `Command`, `Files` and mentions `Ask` named, with no
  catch-all.

## Presence (2026-11-20, the claims follow-up's first row)

[Decision Log]

**Purpose and intent.** The first of the claims rows (the coordinator's plan, 2026-11-20): both clients
report presence (`push.presence`, live since protocol 22). A machine then knows whether a person is here
before it sends a Web Push, and which window to tell.

**Existing implementation and constraints.**
- The node already decides from presence with Paseo's policy (`push/policy.rs`, Paseo's
  `agent-attention-policy.ts:3,42-76`):
  - present means real input within 180 s;
  - only a present, visible window focused on the target suppresses;
  - otherwise the most recently active present window is the one told;
  - otherwise the machine pushes.
- Neither client reported presence, so every Web Push went out even while the person used Vessel.
- The protocol has no client timestamps: `Active` is stamped by the node at receipt (ADR 0089).

**Alternatives considered.**
- Wait for the claims protocol and send presence with it. That leaves pushes going to people at their
  screen until then, and presence does not need it.
- Send presence only while notifications are wanted. Paseo always sends it, and the node uses it for Web
  Push too.

**Choice: Paseo's client tracker, copied** (`~/refs/paseo-latest/packages/app/src/hooks/client-activity-tracker.ts`
and `use-client-activity.ts`).
- Timing:
  - a heartbeat every 15 s while connected, and one at once on connecting (`:1`, `:125-148`);
  - an immediate heartbeat on input at most every 5 s (`:2`, `:63-73`).
- Input and visibility:
  - window focus, a pointer press, a key, a wheel and a touch are input; opening counts as input (`:43`);
  - a new shown session is input and sends at once (`:78-89`);
  - becoming visible is input; becoming hidden waits for the next heartbeat (`:90-105`).
- Focus:
  - `session` on the machine whose session the window shows, when visible;
  - `elsewhere` on every other machine;
  - `background` while hidden.
- Visible is the document's visibility on the page (Paseo's), and the window's visibility on native
  (`observe_window_visibility`). A visible window that is not focused still shows its session, as in
  Paseo.
- Where it lives:
  - page: `web/src/data/presence.ts` (the tracker), `features/notifications/use-presence.ts` (the wiring);
  - native: `view/src/presence.rs` (the tracker and `Told`), `link/src/presence.rs` (a report per machine
    connection), `app/src/areas/presence.rs` (input observed at the root in the capture phase, keystrokes,
    activation, visibility, the 15 s timer).
  - The app hands a snapshot to the link through `Link::presence`. That is a latest-value channel (tokio
    `watch`) beside the command queue, not a command:
    - only the latest state matters;
    - a heartbeat must never fill the queue that carries a person's actions;
    - screens' tests that read that queue see only actions.

    The first build sent it as a command, and three such tests read a presence report where they expected
    their own request.

**Why this over the alternatives.** It is the reference's behaviour, value for value, and the node's
policy is already Paseo's. Each difference is stated below.

**Pros, cons and impact.**
- **This changes Web Push today.** A machine sends no push while a window of the page or the app had real
  input in the last 180 s. It suppresses a session's push entirely while a visible window shows that
  session. A window that goes quiet stops counting 180 s after the report that carried its last input,
  and pushes resume.
- Difference, by the protocol: Paseo sends the input's own time. Vessel's report says `active` and the
  machine stamps it. Input that waits for a heartbeat keeps presence up to 15 s longer than Paseo's. A
  machine that connects hears `active` only for input in the last 15 s, the same bound.
- Difference: no OS idle time (`idle_age` is pending under ADR 0089's constant, and native has no idle
  API).
- A machine refusing presence (older than 22, or a device it takes none from) is not asked again by the
  page until it reconnects. The app's link tries each heartbeat; an older machine is refused by the client
  before the wire.
- Tests:
  - page: `data/presence.test.ts`;
  - native: `view/src/presence/tests.rs` and `link/src/presence/tests.rs`;
  - both run the node's 180 s rule over the reports: a window that goes stale lets push resume, a hidden
    tab is present but never suppresses, and the latest input wins.

