// MODIFIED by Vessel (Team-WICKED), 2026-11-19: a tag's next notification
// replaces the one shown (`replaces_id`) and `dismiss` closes it
// (`CloseNotification`) (spikes/release/patches/README.md,
// "gpui-pre-linux", change 16); see the lines marked "Vessel patch". The
// rest is gpui-pre-linux 0.3.10 as published.
//! System notifications over the XDG notifications D-Bus interface, via
//! `notify-rust`.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use futures::StreamExt as _;
use futures::channel::mpsc;
use gpui::{
    ForegroundExecutor, SharedString, SystemNotification, SystemNotificationResponse, Task,
};
use notify_rust::{NotificationHandle, NotificationResponse};

/// The XDG action key invoked when the user activates the notification body
/// rather than a specific action button.
const DEFAULT_ACTION: &str = "default";

type ResponseCallback = Rc<RefCell<Option<Box<dyn FnMut(SystemNotificationResponse)>>>>;

/// Vessel patch: the notification each tag shows now, written by the thread
/// that showed it and read by the next `show` (its id becomes
/// `replaces_id`) and by `dismiss` (its handle closes it). Shared with those
/// threads, because the server's id is only known once `Notify` returns.
type Shown = Arc<Mutex<HashMap<SharedString, Showing>>>;

/// Vessel patch: one shown notification.
struct Showing {
    /// The server's id for it.
    id: u32,
    /// Which `show` made it: the threads of two `show`s of a tag can finish
    /// `Notify` in either order, and the later `show` is the one kept.
    generation: u64,
    /// Set when a later `show` of the tag replaced it in place (the server
    /// kept the id): its waiter and the new one's both hear the id's
    /// signals, and only the new one answers.
    superseded: Arc<AtomicBool>,
    /// Kept for `CloseNotification`; the waiter borrows it too
    /// (`wait_for_action_async` takes `&self`, where `wait_for_action`
    /// consumed the handle and so left nothing to close it with).
    handle: Arc<NotificationHandle>,
}

pub(crate) struct SystemNotificationState {
    response_sender: mpsc::UnboundedSender<SystemNotificationResponse>,
    response_receiver: Option<mpsc::UnboundedReceiver<SystemNotificationResponse>>,
    callback: ResponseCallback,
    _response_task: Option<Task<()>>,
    /// Vessel patch: see [`Shown`].
    shown: Shown,
    /// Vessel patch: the last `show`'s generation.
    generation: Cell<u64>,
}

impl SystemNotificationState {
    pub(crate) fn new() -> Self {
        let (response_sender, response_receiver) = mpsc::unbounded();
        Self {
            response_sender,
            response_receiver: Some(response_receiver),
            callback: Rc::new(RefCell::new(None)),
            _response_task: None,
            shown: Shown::default(),
            generation: Cell::new(0),
        }
    }

    pub(crate) fn show(&self, app_name: Option<&str>, notification: SystemNotification) {
        let mut builder = notify_rust::Notification::new();
        if let Some(app_name) = app_name {
            builder.appname(app_name);
        }
        builder
            .summary(&notification.title)
            .body(&notification.body)
            .action(DEFAULT_ACTION, DEFAULT_ACTION);
        // Vessel patch: the tag's notification on screen is replaced, as on
        // Windows, rather than joined by a second one. A `show` that comes
        // before the previous one's `Notify` has returned does not know its
        // id yet and shows another; the app shows one per transition.
        if let Some(id) = lock(&self.shown).get(&notification.tag).map(|shown| shown.id) {
            builder.id(id);
        }
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        let mut action_ids = HashMap::new();
        for (index, action) in notification.actions.iter().enumerate() {
            let transport_id = format!("gpui-action-{index}");
            builder.action(&transport_id, &action.label);
            action_ids.insert(transport_id, action.id.clone());
        }
        let built = builder.finalize();

        let sender = self.response_sender.clone();
        let shown = self.shown.clone();
        let tag = notification.tag.clone();
        // `show` connects to the session bus and `wait_for_action` blocks
        // until the notification closes, so both run off the UI thread.
        let spawn_result = std::thread::Builder::new()
            .name("system-notification".to_string())
            .spawn(move || match built.show() {
                Ok(handle) => {
                    // Vessel patch: remembered, then waited on through the
                    // shared handle; forgotten when it closes.
                    let handle = Arc::new(handle);
                    let superseded = Arc::new(AtomicBool::new(false));
                    remember(
                        &shown,
                        &tag,
                        Showing {
                            id: handle.id(),
                            generation,
                            handle: handle.clone(),
                            superseded: superseded.clone(),
                        },
                    );
                    futures::executor::block_on(handle.wait_for_action_async(|response| {
                        let action_id = match response {
                            NotificationResponse::Default => None,
                            NotificationResponse::Action(transport_id) => {
                                let Some(action_id) =
                                    response_action_id(transport_id, &action_ids)
                                else {
                                    log::warn!(
                                        "system notification returned unknown action {transport_id:?}"
                                    );
                                    return;
                                };
                                action_id
                            }
                            // Closed without being activated; replies are
                            // macOS's only.
                            NotificationResponse::Closed(_) | NotificationResponse::Reply(_) => {
                                return;
                            }
                        };
                        if !superseded.load(Ordering::Acquire) {
                            sender
                                .unbounded_send(SystemNotificationResponse {
                                    tag: tag.clone(),
                                    action_id,
                                })
                                .ok();
                        }
                    }));
                    forget(&shown, &tag, generation);
                }
                Err(error) => log::warn!("failed to show system notification: {error}"),
            });
        if let Err(error) = spawn_result {
            log::warn!("failed to spawn system notification thread: {error}");
        }
    }

    /// Vessel patch: closes the tag's notification (`CloseNotification`)
    /// through the handle its `show` kept; the closing signal then ends its
    /// waiter. Upstream did nothing here, so a notice the app had read stayed
    /// in the shade. A tag whose `Notify` has not returned yet is not closed.
    pub(crate) fn dismiss(&self, tag: &str) {
        let Some(showing) = lock(&self.shown).remove(tag) else {
            return;
        };
        // The call waits on the bus, so it too runs off the UI thread.
        let spawn_result = std::thread::Builder::new()
            .name("system-notification-close".to_string())
            .spawn(move || futures::executor::block_on(showing.handle.close_async()));
        if let Err(error) = spawn_result {
            log::warn!("failed to spawn system notification close thread: {error}");
        }
    }

    pub(crate) fn on_response(
        &mut self,
        executor: &ForegroundExecutor,
        callback: Box<dyn FnMut(SystemNotificationResponse)>,
    ) {
        *self.callback.borrow_mut() = Some(callback);

        // Responses arrive from per-notification threads; this task hands
        // them to the registered callback on the main thread.
        if let Some(mut receiver) = self.response_receiver.take() {
            let callback = self.callback.clone();
            self._response_task = Some(executor.spawn(async move {
                while let Some(response) = receiver.next().await {
                    // Take the callback out for the call: it may re-enter the
                    // platform or replace itself.
                    let taken = callback.borrow_mut().take();
                    if let Some(mut taken) = taken {
                        taken(response);
                        callback.borrow_mut().get_or_insert(taken);
                    }
                }
            }));
        }
    }
}

/// Vessel patch: the map stays usable if a notification thread panicked
/// while holding it.
fn lock(shown: &Shown) -> std::sync::MutexGuard<'_, HashMap<SharedString, Showing>> {
    shown.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Vessel patch: record a shown notification. A later `show` of the tag
/// that got there first is kept (their threads can finish `Notify` in
/// either order); whichever is older is marked superseded when the server
/// replaced it in place (the same id). One with another id is a separate
/// notification still on screen (a server that did not replace), so it
/// still answers, as does one whose entry `dismiss` already took.
fn remember(shown: &Shown, tag: &SharedString, showing: Showing) {
    let mut shown = lock(shown);
    if let Some(current) = shown.get(tag) {
        if current.generation > showing.generation {
            if current.id == showing.id {
                showing.superseded.store(true, Ordering::Release);
            }
            return;
        }
        if current.id == showing.id {
            current.superseded.store(true, Ordering::Release);
        }
    }
    shown.insert(tag.clone(), showing);
}

/// Vessel patch: drop the tag's entry when its own notification closed.
fn forget(shown: &Shown, tag: &SharedString, generation: u64) {
    let mut shown = lock(shown);
    if shown
        .get(tag)
        .is_some_and(|showing| showing.generation == generation)
    {
        shown.remove(tag);
    }
}

fn response_action_id(
    transport_id: &str,
    action_ids: &HashMap<String, SharedString>,
) -> Option<Option<SharedString>> {
    if transport_id == DEFAULT_ACTION {
        Some(None)
    } else {
        action_ids.get(transport_id).cloned().map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caller_action_ids_do_not_collide_with_transport_action_ids() {
        let action_ids = HashMap::from([
            ("gpui-action-0".to_string(), SharedString::from("default")),
            ("gpui-action-1".to_string(), SharedString::from("__closed")),
        ]);

        assert_eq!(response_action_id(DEFAULT_ACTION, &action_ids), Some(None));
        assert_eq!(
            response_action_id("gpui-action-0", &action_ids),
            Some(Some("default".into()))
        );
        assert_eq!(
            response_action_id("gpui-action-1", &action_ids),
            Some(Some("__closed".into()))
        );
        assert_eq!(response_action_id("unknown", &action_ids), None);
    }

    /// Vessel patch: the D-Bus half, against the stand-in notification
    /// server of ADR 0114's Linux proof (`tests/fake_notifyd.py`, which
    /// records each `Notify` and `CloseNotification`) on a private session
    /// bus; never the desktop's. The bus address is set only in a child run
    /// of [`shows_replaces_answers_once_and_closes`], so no other test in
    /// this process sees it. Needs `dbus-daemon`, `dbus-send` and Python's
    /// `dbus` and `gi` modules.
    #[test]
    fn a_tag_replaces_its_notification_and_dismiss_closes_it_on_the_fake_server() {
        use std::io::{BufRead as _, BufReader};
        use std::process::{Command, Stdio};

        let dir = std::env::temp_dir().join(format!(
            "gpui-linux-notifications-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let socket = dir.join("bus");
        let log = dir.join("notifications.jsonl");
        let address = format!("unix:path={}", socket.display());
        let mut daemon = Command::new("dbus-daemon")
            .args(["--session", "--nofork", "--nopidfile"])
            .arg(format!("--address={address}"))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .expect("dbus-daemon runs the private bus");
        let started = std::time::Instant::now();
        while !socket.exists() {
            assert!(started.elapsed().as_secs() < 10, "the private bus did not start");
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let script = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fake_notifyd.py");
        let mut server = Command::new("python3")
            .arg("-I")
            .arg(script)
            .arg(&log)
            .env("DBUS_SESSION_BUS_ADDRESS", &address)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .expect("python3 runs the stand-in server");
        let mut ready = String::new();
        BufReader::new(server.stdout.take().unwrap())
            .read_line(&mut ready)
            .unwrap();
        assert_eq!(ready.trim(), "FAKE_NOTIFYD_READY");

        let child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "linux::system_notifications::tests::shows_replaces_answers_once_and_closes",
                "--ignored",
                "--test-threads=1",
            ])
            .env("DBUS_SESSION_BUS_ADDRESS", &address)
            .env("GPUI_LINUX_FAKE_NOTIFY_ADDRESS", &address)
            .env("GPUI_LINUX_FAKE_NOTIFY_LOG", &log)
            .output()
            .unwrap();
        server.kill().ok();
        server.wait().ok();
        daemon.kill().ok();
        daemon.wait().ok();
        let recorded = std::fs::read_to_string(&log).unwrap_or_default();
        std::fs::remove_dir_all(&dir).ok();
        assert!(
            child.status.success() && String::from_utf8_lossy(&child.stdout).contains("1 passed"),
            "child run failed:\n{}\n{}",
            String::from_utf8_lossy(&child.stdout),
            String::from_utf8_lossy(&child.stderr)
        );

        let lines: Vec<&str> = recorded.lines().collect();
        // The first notification is new, the second replaces it with the
        // same id, the third (another tag) is new and is closed by dismiss.
        assert_eq!(lines.len(), 4, "recorded:\n{recorded}");
        assert!(lines[0].contains(r#""id": 1, "#) && lines[0].contains(r#""replaces": 0"#));
        assert!(lines[1].contains(r#""id": 1, "#) && lines[1].contains(r#""replaces": 1"#));
        assert!(lines[2].contains(r#""id": 2, "#) && lines[2].contains(r#""replaces": 0"#));
        assert_eq!(lines[3], r#"{"closed": 2}"#);
    }

    /// The child half of the test above; ignored unless that test runs it on
    /// the private bus.
    #[test]
    #[ignore = "run by a_tag_replaces_its_notification_and_dismiss_closes_it_on_the_fake_server"]
    fn shows_replaces_answers_once_and_closes() {
        let Ok(address) = std::env::var("GPUI_LINUX_FAKE_NOTIFY_ADDRESS") else {
            return;
        };
        let wait_for = |what: &str, ready: &dyn Fn() -> bool| {
            let started = std::time::Instant::now();
            while !ready() {
                assert!(started.elapsed().as_secs() < 10, "timed out waiting for {what}");
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        };
        let notification = |tag: &str, title: &str| SystemNotification {
            tag: tag.to_string().into(),
            title: title.to_string().into(),
            body: "body".into(),
            actions: Vec::new(),
        };
        let mut state = SystemNotificationState::new();
        let mut responses = state.response_receiver.take().unwrap();
        let generation_of = |state: &SystemNotificationState, tag: &str| {
            lock(&state.shown).get(tag).map(|showing| showing.generation)
        };

        state.show(Some("Vessel"), notification("a", "first"));
        wait_for("the first notification", &|| {
            generation_of(&state, "a") == Some(1)
        });
        state.show(Some("Vessel"), notification("a", "second"));
        wait_for("the replacement", &|| generation_of(&state, "a") == Some(2));
        let id = lock(&state.shown).get("a").map(|showing| showing.id).unwrap();

        // A click on the body: both waiters hear the signal for the one id;
        // only the replacement's answers.
        let invoked = std::process::Command::new("dbus-send")
            .arg(format!("--bus={address}"))
            .args([
                "--type=method_call",
                "--dest=org.freedesktop.Notifications",
                "/org/freedesktop/Notifications",
                "org.vessel.FakeNotifications.Invoke",
            ])
            .arg(format!("uint32:{id}"))
            .arg("string:default")
            .status()
            .unwrap();
        assert!(invoked.success());
        wait_for("the closed notification to be forgotten", &|| {
            generation_of(&state, "a").is_none()
        });
        std::thread::sleep(std::time::Duration::from_millis(200));
        let mut answered = Vec::new();
        while let Ok(response) = responses.try_recv() {
            answered.push(response);
        }
        assert_eq!(
            answered,
            vec![SystemNotificationResponse {
                tag: "a".into(),
                action_id: None,
            }]
        );

        state.show(Some("Vessel"), notification("b", "other"));
        wait_for("the other tag", &|| generation_of(&state, "b") == Some(3));
        state.dismiss("b");
        assert!(generation_of(&state, "b").is_none());
        // The close is a call on another thread; the server records it (the
        // parent checks the whole record).
        let log = std::env::var("GPUI_LINUX_FAKE_NOTIFY_LOG").unwrap();
        wait_for("the close", &|| {
            std::fs::read_to_string(&log).is_ok_and(|recorded| recorded.contains("closed"))
        });
    }
}
