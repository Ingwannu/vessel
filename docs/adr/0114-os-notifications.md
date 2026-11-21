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
  - a heartbeat every 15 s while connected, and one at once on connecting (`:1`, `:125-148`). The app
    hands the link its first snapshot as soon as the shell exists, before the first machine connects, so
    opening reaches every machine as input (app review R1);
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

### Notifies (2026-11-21, ADR 0089 "Turned on", item 5, app part)

Paseo elects a claim's recipient only among clients subscribed with `notifications: true`
(`websocket-server.ts:2884-2894,2987-2989`; `session.ts:2336-2345`; the app's `session-context.tsx:492`). A window
that only sounds could otherwise be elected and take the notice from one that shows it.

- A window notifies when Settings' alerts include the system's notifications and the system can show one (the
  build shows them here, and on Linux a session bus is reachable): `Shell::window_notifies`
  (`app/src/areas/notify.rs`), the page's `windowNotifies`. It is read at each render, and a change sends a
  report at once (`Presence::notify`, `app/src/areas/presence.rs`).
- The report carries `notifies: false` only to a machine that admits claims (`Told::report`). Any other machine
  hears the field's absent value, which reads true, as every window did before.
- A window that does not notify still counts as present, so it suppresses and holds push back. It never claims:
  `ask_before_telling` tells every session notice at once, and `terminal_notices_found` tells terminal notices
  itself (the title's count and the sound) without asking.
- Tests:
  - `view/src/presence/tests.rs`, `a_window_that_does_not_notify_says_so_only_to_a_machine_that_admits_claims`;
  - `app/src/areas/notify/claims/tests.rs`, `a_window_that_does_not_notify_tells_at_once_without_asking`;
  - `app/src/areas/notify/terminals/tests.rs`, `a_window_that_does_not_notify_never_claims`.

## Claims (2026-11-21, the claims follow-up's second row)

Both clients ask a machine before they show a session's notice: `notification.claim`, ADR 0089 "The
clients' session claims". This replaces "two open clients may both notify" once ADR 0089's constant is
assigned.

- Until then, no machine admits claims, and every notice takes this ADR's path unchanged.
- Where:
  - the app: `view/src/notices/claims.rs` holds the rules; `link/src/claims.rs` asks
    (`Command::Claims`/`Event::Claims`); `app/src/areas/notify/claims.rs` (`ask_before_telling`,
    `notices_claimed`) tells only what passes;
  - the page: `web/src/data/claims.ts`, wired in `use-notices.ts`.
- What shows: `Admitted`, and a notice whose entry is not on the machine's first inbox page. That fallback
  is the coordinator's rule, and ADR 0089 states the limit. A first page that cannot be read falls back the
  same way for every notice, and no claim is made (ADR 0089, "Turned on", item 4; the page's
  `askBeforeTelling`). Before, a failed page silenced them. Nothing else shows, and a claim is never
  retried.
- Sound: an asked notice's sound plays with its notification when the answer admits it. On the app, the
  answer's notices share one sound, as a fleet change's do.
- Tests:
  - rules, both clients: `view/src/notices/claims/tests.rs` and `web/src/data/claims.test.ts`;
  - the link through the real actor: one first page, one claim per notice found, which the client refuses
    before the wire while unassigned; a failed page answers every notice `NotOnFirstPage`
    (`a_failed_first_page_tells_every_notice_by_the_old_path_and_claims_none`);
  - the app: only admitted and past-the-page notices are told, and every notice is told at once while
    unassigned.
- Owed at assignment: an end-to-end claim against a node, with two windows of which one is admitted.

## Terminal notices (2026-11-21, the claims follow-up's third row, app part)

An agent in a terminal reports through Vessel's hooks (ADR 0089) that it finished or needs input. Paseo tells
one client:
- the server picks the client and titles it "Terminal finished" or "Terminal needs input", with the
  terminal's name as the body (`websocket-server.ts:213-215,2955-3044`);
- the app shows it, and a tap opens the terminal tab (`session-context.tsx:658-677`).

The app now does the same behind ADR 0089's constant. Until the constant is assigned, the node fills no
terminal's attention, and no machine admits claims, so nothing here runs.

- **Source.** For each connected machine that admits claims, the link follows the whole terminal list
  (`terminal.list` with no place), waiting on its revision (ADR 0116). Once the feature is assigned, the
  node moves that revision when a terminal's attention changes (`terminals/registry/listed.rs`).
  - One follower per connection (`link/src/terminal_notices.rs`).
  - A newer connection replaces it, and a closed connection ends it. A failed call on a connection that stays
    open rests 15 s, then asks for the whole list again, keeping what was told (app review note 3; the page's
    `RETRY_MS`). Before, a failed call ended the follower until the next connection.
  - A list whose revision did not move is asked again only after a quiet second. A node that answers without
    waiting (its wait slots full, a delegated caller) would otherwise be asked at round-trip speed (app review
    note).
  - Following each terminal instead would cost a long poll per terminal, up to 96 a machine.
- **What is a notice** (`view/src/notices/terminals.rs`, with the Decision Log):
  - a terminal at Finished or Needs input, at an attention revision the connection has not told of;
  - never the connection's first list, as for sessions;
  - never a failed terminal, as the node's push worker.
- **Claim and show.**
  - Settings' alerts decide first: a window that tells nothing never claims.
  - The claim names the exact revision (`NotificationSource::Terminal`).
  - Only `Admitted` shows. A terminal has no inbox, so there is no fallback.
  - The title is Paseo's words; the body is the terminal's title, or "Terminal", never its command.
  - Admitted notices share one sound and count in the window's title, as session notices do.
- **Click** (`app/src/areas/notify/terminals.rs`): opens the session whose workspace holds the terminal's
  place, then the terminal's tab there. Nothing opens when the fleet lists no such session.
  - A session holds a place as the page's `holds` reads it (`view::terminal_tabs::holds`): its own shell, its
    worktree's, any folder of its project, or its folder as a path. Before, the click compared the whole
    place with `place_for(card)`, so a shell in a project's subfolder, or beside a session of a workspace,
    opened nothing.
  - A folder that is no session's as a path is asked of the node (app review note 2): the node keeps a folder
    place canonical, and a session keeps its folder as given, so through a link or a `..` the two differ. The
    link lists each folder session's folder (`Command::LocateTerminal`, `link/src/terminal_holder.rs`, with the
    Decision Log), and the session the node names opens on the next frame (`Event::TerminalLocated`,
    `settle_terminal_notice`). A find for an earlier click is dropped. Paseo's daemon names the workspace in
    the notification itself (`session-context.tsx:658-677`, `workspaceId`).
- **Presence.** A terminal tab with the focused pane in the shown workspace is reported as
  `Terminal{terminal}` focus (`view/src/presence.rs`, `Shown::Terminal`), as Paseo's focus target is an agent
  or a terminal (`agent-attention-policy.ts:12`). The machine then suppresses that terminal's notice and
  keeps the session's.
  - A report carrying `Terminal` focus is versioned to ADR 0089's constant (`rpc/since.rs`). So only a
    machine that admits claims hears it.
  - Every other machine hears `Elsewhere`. **This changes push today.** Before this row, a terminal tab
    reported its route's session, so that session's push was held back while the person looked at the
    terminal. Now it goes, as Paseo's does: its focus target is then the terminal, not the agent.
- **Viewing clears.** While the window is visible and a terminal's tab is shown, the app sends
  `terminal.attention_clear` with the exact attention revision its list shows (`notices::terminals::to_clear`,
  `notify/terminals.rs`), once per revision.
  - Paseo's node clears a focused terminal's attention on a visible heartbeat (`session.ts:5064-5066`).
    Vessel's clear is the client's, by revision, so a stale view never clears newer attention (ADR 0089).
  - A lost clear is not retried, and the next view sends it again.
- **Not in this part:** the page. It follows on the node line.
- **Tests:**
  - `view/src/notices/terminals/tests.rs`: the first list, a new revision told once, the kinds, a failed
    terminal, alerts off;
  - `link/src/terminal_notices/tests.rs`, through the real framed actor: the whole list, then from its
    revision, and only the new attention told; a failed call rests, asks for the whole list again and tells
    only what is new, and a closed connection ends the follower; a fresh connection follows only while the
    constant is assigned;
  - `link/src/terminal_holder/tests.rs`: the folders are listed in turn, a failed or empty one is passed
    over, and the first that lists the terminal names its session;
  - `view/src/terminal_tabs/places_tests.rs`, `a_session_holds_its_own_shells_its_workspaces_and_its_folder_as_a_path`;
  - `app/src/areas/notify/terminals/tests.rs`: only admitted notices are shown, with their titles; alerts
    off never claims; a viewed terminal's attention is cleared once, only on a machine that admits claims; a
    folder terminal no session names as a path is asked of the node, and the session it names opens;
  - `view/src/presence/tests.rs` and `link/src/presence/tests.rs`: a shown terminal is `Terminal` focus only
    to a machine that admits claims.


## Inbox read (2026-11-21, the claims follow-up's fourth row, app part)

On a machine that admits ADR 0089's constant, the app marks the session shown read with `inbox.read`
(`read_inbox_through(session, through)`) instead of `session.mark read`. The rule for when a read is due is
unchanged (ADR 0026, `view::session::read_through`).

- Why: `inbox.read` takes the `Read` scope, so a device given only `read` can mark this way, where
  `session.mark` asks for more. The node's inbox and its attention read the same mark.
- Where:
  - `view/src/notices/claims.rs` holds `read_call(protocol)`: `Inbox` when the machine admits claims, `Mark`
    otherwise;
  - `app/src/areas/conversation/read.rs` sends `Command::ReadInbox` for the session `act` would mark;
  - `link/src/claims.rs` holds `ReadInbox` and calls it, fire and forget.
- Nothing answers it but the session's list, so the read area does not wait on it in flight. A lost one leaves
  the session unread, and the next view sends it again.
- Not changed: the person's own "Mark as read" in a row's menu stays `session.mark`. Below the constant every
  read stays as it is.
- The inbox path never sets the read-only stop on `Forbidden`: `inbox.read` takes the `Read` scope, which
  every paired device has, so a refusal there is not the scope's (app review note 2 of row 4). Revisit it if
  `inbox.read` ever asks for more.
- Tests:
  - `view/src/notices/claims/tests.rs`, `a_read_is_marked_by_inbox_read_only_on_a_machine_that_admits_the_protocol`;
  - `app/src/areas/conversation/read/tests.rs`, `a_read_on_a_claiming_machine_is_one_inbox_read_and_no_mark`:
    one `ReadInbox` and no `Act::Mark` once the constant is assigned, the read mark before it.

## Badges (2026-11-21, the claims follow-up's fifth row)

Paseo shows how many workspaces want the person on the desktop's dock, and swaps its favicon among none, running
and attention (`desktop-badge-state.ts:5-14`, `use-favicon-status.ts`).

- **Page:** it does both, with Paseo's buckets over Vessel's sessions. The app badge uses `navigator.setAppBadge`
  where the browser offers it. The favicon has three states, drawn by the build. ADR 0089, "The page's badge and
  favicon", has the rule, the files and the tests.
- **App: a stated difference (TRACKER 831).** The app shows no count on the OS dock or taskbar:
  - Paseo's desktop shows its count on the macOS dock only: the app calls `setBadgeCount` only in Electron on a
    Mac (`use-favicon-status.ts:80-95`, `updateMacDockBadge`), which sets the `NSDockTile` badge through
    Electron's `app.setBadgeCount` (`desktop/src/window/window-manager.ts:181-194`);
  - GPUI has no badge call;
  - Windows' `ITaskbarList3` overlay and Linux's `com.canonical.Unity.LauncherEntry` would each need a patch;
  - the coordinator ruled on 2026-11-20 not to patch GPUI for it.
  A window has no favicon to swap. What stays:
  - the window title's count of notices since the app was last in front (`notices::title_with`, this ADR);
  - the sidebar's and board's own marks for each session that needs the person or failed.
- **Owed if the difference is lifted:** the count would come from the page's rule (`web/src/data/badge.ts`),
  ported to `view` as one function, and be set through a GPUI patch that goes through the lead.
