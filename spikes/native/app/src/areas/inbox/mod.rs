//! One attention list from every paired machine; existing conversation cards
//! answer a freshly confirmed request without manufacturing timeline events.
mod actions;
mod draw;
mod notifications;
mod place;
mod receive;
pub(crate) mod settings;
pub(crate) use draw::render;

use super::conversation::ConversationArea;
use crate::shell::Shell;
use vessel_app_kit::ui::Context;
use vessel_app_link::inbox::Target;
use vessel_app_view::inbox::{Detail, Inbox, Problem};
use vessel_app_view::timeline::ToolView;

pub(crate) struct InboxArea {
    pub(crate) store: Inbox,
    pub(crate) detail: Detail,
    pub(crate) conversation: ConversationArea,
    pub(crate) targets: Vec<Target>,
    pub(crate) problem: Option<Problem>,
    pub(crate) tool: Option<ToolView>,
    pub(crate) notice_routes: vessel_app_view::inbox::notices::NoticeRoutes,
    pub(crate) notices: vessel_app_view::inbox::notices::NoticeQueue,
}
impl InboxArea {
    pub(crate) fn new(window: &vessel_app_kit::ui::Window, cx: &mut Context<'_, Shell>) -> Self {
        notifications::listen(window, cx);
        Self {
            store: Inbox::default(),
            detail: Detail::default(),
            conversation: ConversationArea::new(cx),
            targets: Vec::new(),
            problem: None,
            tool: None,
            notice_routes: vessel_app_view::inbox::notices::NoticeRoutes::default(),
            notices: vessel_app_view::inbox::notices::NoticeQueue::default(),
        }
    }
}
