//! Aggregate desktop idle only; never inspect input content or a terminal.
//! X11's existing protocol client runs on one dedicated worker per process.
//! A timeout permanently disables that worker's consumer, so an unresponsive
//! display cannot accumulate blocked tasks. Other backends remain unknown.
use crate::shell::Shell;
use std::time::Duration;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::inbox::presence::{IdleAge, IdleSample};

const INTERVAL: Duration = Duration::from_secs(5);
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
const DEADLINE: Duration = Duration::from_millis(500);

pub(super) fn listen(window: &mut Window, cx: &mut Context<'_, Shell>) {
    let mut probe = Probe::new();
    cx.spawn_in(window, async move |this, cx| {
        loop {
            cx.background_executor().timer(INTERVAL).await;
            let age = probe.sample(cx).await;
            if this
                .update_in(cx, |shell, _window, _cx| {
                    shell.areas.inbox.presence.system_idle(age);
                })
                .is_err()
            {
                break;
            }
        }
    })
    .detach();
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
type Request = tokio::sync::oneshot::Sender<Option<IdleAge>>;

#[derive(Debug)]
struct Probe {
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    sender: Option<std::sync::mpsc::SyncSender<Request>>,
}
impl Probe {
    fn new() -> Self {
        Self {
            #[cfg(any(target_os = "linux", target_os = "freebsd"))]
            sender: worker(),
        }
    }
    async fn sample(&mut self, cx: &mut gpui::AsyncWindowContext) -> Option<IdleSample> {
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            let sender = self.sender.as_ref()?;
            let observed = std::time::Instant::now();
            let (reply, receive) = tokio::sync::oneshot::channel();
            if sender.try_send(reply).is_err() {
                self.sender = None;
                return None;
            }
            let result =
                futures_lite::future::race(async { receive.await.ok().flatten() }, async {
                    cx.background_executor().timer(DEADLINE).await;
                    None
                })
                .await;
            if result.is_none() {
                self.sender = None;
            }
            result.map(|age| IdleSample::new(age, observed))
        }
        #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
        {
            let _context = cx;
            None
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn worker() -> Option<std::sync::mpsc::SyncSender<Request>> {
    static WORKER: std::sync::OnceLock<Option<std::sync::mpsc::SyncSender<Request>>> =
        std::sync::OnceLock::new();
    WORKER.get_or_init(start).clone()
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn start() -> Option<std::sync::mpsc::SyncSender<Request>> {
    // Xwayland sees only its own clients. Calling that global desktop idle
    // would suppress notifications after activity in unrelated Wayland apps.
    if std::env::var_os("WAYLAND_DISPLAY").is_some() || std::env::var_os("DISPLAY").is_none() {
        return None;
    }
    let (sender, receive) = std::sync::mpsc::sync_channel::<Request>(1);
    let worker = std::thread::Builder::new()
        .name("desktop-idle".into())
        .spawn(move || {
            use x11rb::connection::Connection as _;
            use x11rb::protocol::screensaver::ConnectionExt as _;
            let connection = x11rb::connect(None).ok();
            while let Ok(reply) = receive.recv() {
                let age = connection.as_ref().and_then(|(connection, screen)| {
                    let root = connection.setup().roots.get(*screen)?.root;
                    let info = connection.screensaver_query_info(root).ok()?.reply().ok()?;
                    Some(IdleAge::new(info.ms_since_user_input))
                });
                let _sent = reply.send(age);
                if age.is_none() {
                    break;
                }
            }
        })
        .ok()?;
    drop(worker);
    Some(sender)
}
