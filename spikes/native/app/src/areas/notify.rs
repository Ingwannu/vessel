//! The system's notifications, the notice sound and the window title's
//! count (ADR 0114; TRACKER J12-J29, J54-J55), as Paseo's desktop
//! notifications, on GPUI's system notification API.
//!
//! [Decision Log]
//! - Purpose: a session that starts to need the person, or finishes,
//!   reaches them while they are elsewhere, and a click takes them to it.
//! - Existing assets: the diff and the rules are view's
//!   (`notices::notices`, `notices::announce`). The patched GPUI shows
//!   notifications on Linux (D-Bus) and Windows (toasts, its app id
//!   registered by the patch), and calls back on a click. The sound is
//!   link's (`sound::play`).
//! - Choice:
//!   - Each change of the fleet is diffed against the one before. What
//!     the rules keep is shown, sounded and counted.
//!   - A notification carries an opaque tag: a random run id and a
//!     counter, never a name or a path. The tag is mapped to its session
//!     in this process, for the last 64. A click on a known tag brings the
//!     window forward and opens the session. An unknown or older tag (from
//!     an earlier run, or past 64) only brings the window forward.
//!   - The same session and kind reuses its tag. Windows then replaces
//!     the earlier toast, and Linux the earlier notification (the GPUI
//!     Linux patch maps the tag to the server's `replaces_id`, patch 16,
//!     TRACKER 818).
//!   - One sound per change of the fleet: needs-you if any, else finished.
//!     Ten sessions finishing in one read sound once, not ten times over
//!     each other. (The page plays one for each notice.)
//!   - The window's title is written by render, which has the window. A
//!     fleet event does not, and writing it there would borrow the window
//!     from inside its own update.
//!   - Settings' test keeps its result for Paseo's inline Alert. GPUI's
//!     call returns nothing, so only what is known before sending can
//!     fail: a build without system notifications, or Linux without a
//!     session bus.
//! - Differences until ADR 0089's claims (TRACKER 815): every open app
//!   decides for itself; no terminal notices (behind the constant in
//!   `notify/terminals.rs`); the body is the session's name, not Paseo's
//!   last-message preview; no macOS banner (the GPUI patches have none).

use std::collections::VecDeque;

use gpui::{
    AnyWindowHandle, App, SharedString, Subscription, SystemNotification,
    SystemNotificationResponse, WeakEntity,
};
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::board::Fleet;
use vessel_app_view::notices::{Announce, NoticeKind, Seen, Unseen, announce, notices, title_with};
use vessel_proto::notifications::TerminalAttentionRevision;
use vessel_proto::terminals::TerminalPlace;
use vessel_proto::{SessionId, TerminalId};

use crate::routes::Route;
use crate::shell::Shell;

/// How many notifications a click still opens the session of.
const KEPT: usize = 64;

/// The notifier's state, the shell's.
pub(crate) struct NotifyArea {
    /// The window is the active one.
    active: bool,
    /// The notices the title counts.
    unseen: Unseen,
    /// The window's title as opened; the count goes before it.
    base: SharedString,
    /// The title render writes next.
    title: Option<SharedString>,
    /// This run's part of every tag.
    run: SharedString,
    /// The tags shown, oldest first.
    shown: VecDeque<Shown>,
    next: u64,
    /// The title switch's turns, for its motion.
    pub(crate) turns: u64,
    /// The last test's result, shown under Settings' section.
    pub(crate) tested: Option<Tested>,
    /// The last terminal attention a view cleared, so a render does not
    /// clear it again (`notify/terminals.rs`).
    cleared: Option<(TerminalId, TerminalAttentionRevision)>,
    _activation: Subscription,
}

impl std::fmt::Debug for NotifyArea {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NotifyArea")
            .field("active", &self.active)
            .field("unseen", &self.unseen)
            .field("shown", &self.shown.len())
            .finish_non_exhaustive()
    }
}

/// A notification shown, and what it was about.
#[derive(Debug)]
struct Shown {
    tag: SharedString,
    machine: String,
    about: About,
    kind: NoticeKind,
}

/// What a notification was shown for: a session, or a terminal and where
/// it lives (claims row 3, `notify/terminals.rs`).
#[derive(Debug, Clone, PartialEq, Eq)]
enum About {
    Session(SessionId),
    Terminal {
        terminal: TerminalId,
        place: TerminalPlace,
    },
}

impl NotifyArea {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        let activation = cx.observe_window_activation(window, Shell::notify_activation);
        let shell = cx.weak_entity();
        let handle = window.window_handle();
        cx.on_system_notification_response(move |response, cx| {
            clicked(&shell, handle, &response, cx);
        });
        Self {
            active: window.is_window_active(),
            unseen: Unseen::default(),
            base: crate::window_title().into(),
            title: None,
            run: SessionId::random().to_string().into(),
            shown: VecDeque::new(),
            next: 0,
            turns: 0,
            tested: None,
            cleared: None,
            _activation: activation,
        }
    }

    /// The tag for a notice about `about` on `machine`: its earlier one, or
    /// a new one.
    fn tag(&mut self, machine: &str, about: About, kind: NoticeKind) -> SharedString {
        let earlier = self.shown.iter().position(|shown| {
            shown.machine == machine && shown.about == about && shown.kind == kind
        });
        if let Some(kept) = earlier.and_then(|at| self.shown.remove(at)) {
            let tag = kept.tag.clone();
            self.shown.push_back(kept);
            return tag;
        }
        self.next = self.next.wrapping_add(1);
        let tag: SharedString = format!("vessel-{}-{}", self.run, self.next).into();
        if self.shown.len() >= KEPT {
            let _oldest = self.shown.pop_front();
        }
        self.shown.push_back(Shown {
            tag: tag.clone(),
            machine: machine.to_owned(),
            about,
            kind,
        });
        tag
    }

    /// What a tag was shown for, and on which machine.
    fn target(&self, tag: &str) -> Option<(String, About)> {
        self.shown
            .iter()
            .find(|shown| shown.tag == tag)
            .map(|shown| (shown.machine.clone(), shown.about.clone()))
    }
}

impl Shell {
    /// The window became active or not: coming back clears the count.
    fn notify_activation(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.notify;
        area.active = window.is_window_active();
        if area.active && area.unseen.back() {
            area.title = Some(area.base.clone());
            cx.notify();
        }
    }

    /// Between two reads of the machines: show, sound and count what
    /// started to need the person or finished (ADR 0114).
    pub(crate) fn notify_fleet(&mut self, earlier: &Fleet, cx: &mut Context<'_, Self>) {
        let found = notices(earlier, &self.fleet);
        if found.is_empty() {
            return;
        }
        let settings = self.prefs.settings.notifications;
        let shown = self
            .areas
            .session
            .open
            .as_ref()
            .filter(|_| self.route == Route::Session)
            .map(|open| (open.machine.as_str(), open.session));
        let area = &mut self.areas.notify;
        if area.unseen.found(found.len(), area.active, settings.title) {
            area.title = Some(title_with(&area.base, area.unseen.count()).into());
            cx.notify();
        }
        let seen = Seen {
            active: area.active,
            shown,
        };
        let told = announce(found, seen, settings.alerts);
        let now = self.ask_before_telling(told);
        self.tell(&now, cx);
    }

    /// Show `told`'s system notifications and play one sound for them: the
    /// needs-you tone when any needs the person, else the finished one.
    pub(crate) fn tell(&mut self, told: &[Announce], cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.notify;
        let words = self.words.notifications();
        for one in told.iter().filter(|one| one.system) {
            let notice = &one.notice;
            let body = if notice.title.is_empty() {
                words.untitled().to_owned()
            } else {
                notice.title.clone()
            };
            cx.show_system_notification(SystemNotification {
                tag: area.tag(&notice.machine, About::Session(notice.session), notice.kind),
                title: self.t.lang.notices().title(notice.kind).into(),
                body: body.into(),
                actions: Vec::new(),
            });
        }
        let sound = told
            .iter()
            .filter(|one| one.sound)
            .map(|one| one.notice.kind)
            .min_by_key(|kind| match kind {
                NoticeKind::NeedsYou => 0,
                NoticeKind::Finished => 1,
            });
        if let Some(kind) = sound {
            vessel_app_link::sound::play(kind);
        }
    }

    /// Settings' test: a notification and the needs-you sound, as the
    /// alerts say. Its result stays under the section until the next
    /// test (Paseo's inline Alert).
    pub(crate) fn notify_test(&mut self, cx: &mut Context<'_, Self>) {
        let alerts = self.prefs.settings.notifications.alerts;
        let words = self.words.notifications();
        let system = Readiness::of_this_system();
        let tested = match (alerts.system(), reach(system)) {
            (true, Ok(())) => {
                cx.show_system_notification(SystemNotification {
                    tag: format!("vessel-{}-test", self.areas.notify.run).into(),
                    title: words.test_title().into(),
                    body: words.test_body().into(),
                    actions: Vec::new(),
                });
                Tested::Handed
            }
            (true, Err(unavailable)) => Tested::Failed(unavailable),
            // Sound only: link plays where the build has system
            // notifications too (Linux, Windows); elsewhere it is a stub,
            // so saying "sent" would be untrue (the app reviewer, row 338).
            (false, _) if system.built => Tested::Sounded,
            (false, _) => Tested::Failed(Unavailable::Build),
        };
        if alerts.sound() && !matches!(tested, Tested::Failed(_)) {
            vessel_app_link::sound::play(NoticeKind::NeedsYou);
        }
        if alerts.system() || alerts.sound() {
            self.areas.notify.tested = Some(tested);
            cx.notify();
        }
    }

    /// A click on one of this run's notifications: open its session, or its
    /// terminal.
    fn notice_clicked(&mut self, tag: &str, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some((machine, about)) = self.areas.notify.target(tag) else {
            return;
        };
        let session = match about {
            About::Session(session) => session,
            About::Terminal { terminal, place } => {
                self.terminal_notice_clicked(&machine, terminal, &place, window, cx);
                return;
            }
        };
        let open = self
            .fleet
            .machines
            .iter()
            .filter(|known| known.name == machine)
            .flat_map(|known| &known.sessions)
            .find(|card| card.session == session)
            .map(crate::areas::session::Open::of);
        if let Some(open) = open {
            self.open_session(open, window, cx);
        }
    }
}

/// What Settings' test did (Paseo's inline Alert under the section).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tested {
    /// The notification was handed to the system.
    Handed,
    /// Sound only: the sound was asked for; no notification was sent.
    Sounded,
    /// The system cannot show it, and why.
    Failed(Unavailable),
}

/// Why this system cannot show a notification, known before sending.
/// GPUI's call returns nothing, so a failure after it is not seen here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unavailable {
    /// This build has no system notifications and no sound (macOS, a
    /// browser).
    Build,
    /// Linux without a session bus: the notification service cannot be
    /// reached (an SSH login, a bare display).
    NoBus,
}

/// What decides whether this system can show a notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Readiness {
    /// The build shows system notifications here (Linux, Windows).
    built: bool,
    /// Linux only: a session bus is named or at its usual place.
    bus: bool,
}

impl Readiness {
    fn of_this_system() -> Self {
        // A browser build's `target_os` is `unknown`.
        let built = cfg!(any(target_os = "linux", target_os = "windows"));
        let bus = if cfg!(target_os = "linux") {
            std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some()
                || std::env::var_os("XDG_RUNTIME_DIR")
                    .is_some_and(|run| std::path::Path::new(&run).join("bus").exists())
        } else {
            true
        };
        Self { built, bus }
    }
}

/// Whether a notification can be handed to the system.
const fn reach(system: Readiness) -> Result<(), Unavailable> {
    if !system.built {
        Err(Unavailable::Build)
    } else if !system.bus {
        Err(Unavailable::NoBus)
    } else {
        Ok(())
    }
}

/// Render's part: the title the last change asked for.
pub(crate) fn retitle(shell: &mut Shell, window: &mut Window) {
    if let Some(title) = shell.areas.notify.title.take() {
        window.set_window_title(&title);
    }
}

/// A click on a notification, from the system: bring the window forward,
/// then open the session the tag was shown for.
fn clicked(
    shell: &WeakEntity<Shell>,
    handle: AnyWindowHandle,
    response: &SystemNotificationResponse,
    cx: &mut App,
) {
    cx.activate(true);
    let done = handle.update(cx, |_root, window, cx| {
        window.activate_window();
        shell.update(cx, |shell, cx| {
            shell.notice_clicked(&response.tag, window, cx);
        })
    });
    if !matches!(done, Ok(Ok(()))) {
        tracing::debug!("a notification's click found no window");
    }
}

mod claims;
mod terminals;

#[cfg(test)]
mod tests;
