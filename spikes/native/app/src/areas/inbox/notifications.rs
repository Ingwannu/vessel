//! OS delivery consumes only accepted new attention from the inbox store.
//! GPUI posts silent visual notifications; the separately selected system bell
//! is requested through the current window, without shell helpers or new deps.
use crate::routes::Route;
use crate::shell::Shell;
use gpui::AppContext as _;
use vessel_app_kit::ui::{Context, SystemNotification, Window};
use vessel_app_link::{Command, inbox::Ask};
use vessel_app_view::inbox::delivery::{Channel, Delivery, Visibility};
use vessel_app_view::inbox::notices::DeliveryTag;

pub(super) const SUMMARY_TAG: &str = "vessel-inbox-summary";
const TAG_PREFIX: &str = "vessel-desktop-notice/";

impl Shell {
    pub(crate) fn inbox_window(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.inbox_read_frame(window);
        self.inbox_presence(window);
        let title = if self.prefs.settings.notifications.title {
            let count =
                usize::try_from(self.areas.inbox.store.counts().unread()).unwrap_or(usize::MAX);
            let terminal_count = self
                .areas
                .inbox
                .store
                .nodes()
                .iter()
                .filter_map(|node| node.snapshot.as_ref())
                .flat_map(|snapshot| snapshot.terminals.iter().flatten())
                .filter(|row| {
                    row.attention
                        .is_some_and(vessel_app_view::inbox::terminals::announces)
                })
                .count();
            vessel_app_view::notices::title_with(self.t.app, count.saturating_add(terminal_count))
        } else {
            self.t.app.to_owned()
        };
        window.set_window_title(&title);
        self.inbox_claim_notices(window);
        self.inbox_terminal_claims(window);
        self.inbox_deliver_grants(window, cx);
        self.inbox_terminal_deliver(window, cx);
    }

    fn inbox_claim_notices(&mut self, window: &Window) {
        let delivery = Delivery::of(self.prefs.settings.notifications.alerts);
        let (notices, coalescing) = self.areas.inbox.notices.take();
        if coalescing == vessel_app_view::inbox::notices::Coalescing::Combined {
            self.areas.inbox.coalescing = coalescing;
        }
        let visible = self.inbox_visibility(window);
        let area = &mut self.areas.inbox;
        area.admission
            .retain(|source| area.targets.iter().any(|target| matches!(target, vessel_app_link::inbox::Target::Online(known) if known == source)));
        for key in notices {
            if !visible.permits(key)
                || (delivery.desktop == Channel::Disabled && delivery.sound == Channel::Disabled)
            {
                continue;
            }
            let Some(entry) = self.areas.inbox.store.entry(key) else {
                continue;
            };
            if entry.read != vessel_app_view::inbox::Read::Unread || !entry.content.announces() {
                continue;
            }
            let position = entry.position;
            let Some(source) = self.inbox_source(key) else {
                continue;
            };
            let request = match self.areas.inbox.store.ticket() {
                Ok(request) => request,
                Err(problem) => {
                    self.areas.inbox.problem = Some(problem);
                    continue;
                }
            };
            let pending = vessel_app_view::inbox::admission::Pending {
                source,
                key,
                position,
                request,
            };
            if self.areas.inbox.admission.begin(pending.clone()) {
                self.command(Command::Inbox(Ask::Claim(pending)));
            }
        }
    }

    fn inbox_deliver_grants(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let delivery = Delivery::of(self.prefs.settings.notifications.alerts);
        let visible = self.inbox_visibility(window);
        let granted = std::mem::take(&mut self.areas.inbox.granted);
        let mut sound = false;
        for pending in granted {
            let key = pending.key;
            if !self
                .areas
                .inbox
                .targets
                .iter()
                .any(|target| matches!(target, vessel_app_link::inbox::Target::Online(source) if *source == pending.source))
            {
                continue;
            }
            if !visible.permits(key) {
                continue;
            }
            let Some(entry) = self.areas.inbox.store.entry(key) else {
                continue;
            };
            if entry.position != pending.position
                || entry.read != vessel_app_view::inbox::Read::Unread
                || !entry.content.announces()
            {
                continue;
            }
            if delivery.desktop == Channel::Enabled {
                let token = DeliveryTag::new(uuid::Uuid::new_v4().into_bytes());
                super::notification_routes::remember(token, pending.source, key, window, cx);
                cx.show_system_notification(SystemNotification {
                    tag: tag(token).into(),
                    title: self.t.app.into(),
                    body: self.words.inbox().notification_body().to_owned().into(),
                    actions: Vec::new(),
                });
            }
            sound |= delivery.sound == Channel::Enabled;
        }
        if sound {
            window.play_system_bell();
        }
    }

    /// Delivery is driven by replies even when the OS is not painting a
    /// minimized window. The window update also supplies the real bell host.
    pub(super) fn inbox_notice_update(&self, cx: &mut Context<'_, Self>) {
        let window = self.areas.inbox.window;
        cx.spawn(async move |this, cx| {
            let _closed = cx.update_window(window, |root, window, cx| {
                let _current =
                    super::lifetime::update(&this, &root, window, cx, |shell, window, cx| {
                        shell.inbox_window(window, cx);
                    });
            });
        })
        .detach();
    }

    fn inbox_visibility(&self, window: &Window) -> Visibility {
        if !window.is_window_active() || !window.is_visible() {
            return Visibility::Away;
        }
        if self.route == Route::Session
            && let Some(open) = self
                .areas
                .session
                .open
                .as_ref()
                .filter(|open| open.structured)
            && let Some(target) = self
                .areas
                .inbox
                .read_open
                .filter(|target| target.session == open.session)
        {
            return Visibility::Viewing {
                node: target.node,
                session: target.session,
            };
        }
        if self.route == Route::Inbox
            && let Some(key) = self.areas.inbox.detail.key()
        {
            return Visibility::Viewing {
                node: key.node,
                session: key.session,
            };
        }
        Visibility::Away
    }
}

pub(super) fn tag(token: DeliveryTag) -> String {
    format!("{TAG_PREFIX}{}", uuid::Uuid::from_bytes(token.bytes()))
}
pub(super) fn parse_tag(text: &str) -> Option<DeliveryTag> {
    uuid::Uuid::parse_str(text.strip_prefix(TAG_PREFIX)?)
        .ok()
        .map(|id| DeliveryTag::new(id.into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_tags_round_trip_and_cannot_accept_extra_segments() {
        let key = DeliveryTag::new(uuid::Uuid::new_v4().into_bytes());
        assert_eq!(parse_tag(&tag(key)), Some(key));
        assert_eq!(parse_tag(&format!("{}/extra", tag(key))), None);
        assert_eq!(parse_tag("other-notification"), None);
    }
}
