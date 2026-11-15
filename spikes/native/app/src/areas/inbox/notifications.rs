//! OS delivery consumes only accepted new attention from the inbox store.
//! GPUI posts silent visual notifications; the separately selected system bell
//! is requested through the current window, without shell helpers or new deps.
use crate::routes::Route;
use crate::shell::Shell;
use vessel_app_kit::ui::{Context, SystemNotification, Window};
use vessel_app_view::inbox::delivery::{Channel, Delivery, Visibility};
use vessel_app_view::inbox::notices::{Coalescing, DeliveryTag};

const SUMMARY_TAG: &str = "vessel-inbox-summary";
const TAG_PREFIX: &str = "vessel-desktop-notice/";

pub(super) fn listen(window: &Window, cx: &mut Context<'_, Shell>) {
    let shell = cx.entity().downgrade();
    let handle = window.window_handle();
    cx.on_system_notification_response(move |response, cx| {
        let key = parse_tag(response.tag.as_ref());
        if key.is_none() && response.tag.as_ref() != SUMMARY_TAG {
            return;
        }
        let _closed = shell.update(cx, |shell, cx| {
            shell.route_to(Route::Inbox);
            if let Some(key) = key.and_then(|tag| shell.areas.inbox.notice_routes.take(tag)) {
                shell.inbox_select(key, cx);
            }
            cx.activate(true);
        });
        let _closed = handle.update(cx, |_root, window, _cx| window.activate_window());
    });
}

impl Shell {
    pub(crate) fn inbox_window(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.inbox_presence(window);
        let title = if self.prefs.settings.notifications.title {
            let count =
                usize::try_from(self.areas.inbox.store.counts().unread()).unwrap_or(usize::MAX);
            vessel_app_view::notices::title_with(self.t.app, count)
        } else {
            self.t.app.to_owned()
        };
        window.set_window_title(&title);
        let delivery = Delivery::of(self.prefs.settings.notifications.alerts);
        let (notices, coalescing) = self.areas.inbox.notices.take();
        let visible = self.inbox_visibility(window);
        if coalescing == Coalescing::Combined && !notices.is_empty() {
            if delivery.desktop == Channel::Enabled {
                cx.show_system_notification(SystemNotification {
                    tag: SUMMARY_TAG.into(),
                    title: self.words.inbox().title().to_owned().into(),
                    body: self.words.inbox().combined().to_owned().into(),
                    actions: Vec::new(),
                });
            }
            if delivery.sound == Channel::Enabled {
                window.play_system_bell();
            }
            return;
        }
        let mut sound = false;
        for key in notices {
            if !visible.permits(key) {
                continue;
            }
            let Some(entry) = self.areas.inbox.store.entry(key) else {
                continue;
            };
            if entry.read != vessel_app_view::inbox::Read::Unread || !entry.content.announces() {
                continue;
            }
            if delivery.desktop == Channel::Enabled {
                let token = DeliveryTag::new(uuid::Uuid::new_v4().into_bytes());
                self.areas.inbox.notice_routes.remember(token, key);
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

    fn inbox_visibility(&self, window: &Window) -> Visibility {
        if !window.is_window_active() || !window.is_visible() {
            return Visibility::Away;
        }
        if self.route == Route::Session
            && let Some(open) = &self.areas.session.open
            && let Some(node) = self
                .areas
                .inbox
                .store
                .nodes()
                .iter()
                .find(|node| node.source.name.as_str() == open.machine)
        {
            return Visibility::Viewing {
                node: node.source.node,
                session: open.session,
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

fn tag(token: DeliveryTag) -> String {
    format!("{TAG_PREFIX}{}", uuid::Uuid::from_bytes(token.bytes()))
}
fn parse_tag(text: &str) -> Option<DeliveryTag> {
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
