//! One attention list from every paired machine; existing conversation cards
//! answer a freshly confirmed request without manufacturing timeline events.
mod actions;
mod draw;
mod idle;
mod notifications;
mod place;
mod presence;
mod reading;
pub(crate) use reading::start;
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
    pub(crate) window: gpui::AnyWindowHandle,
    pub(crate) admission: vessel_app_view::inbox::admission::Gate,
    pub(crate) granted: Vec<vessel_app_view::inbox::admission::Pending>,
    pub(crate) detail: Detail,
    pub(crate) conversation: ConversationArea,
    pub(crate) targets: Vec<Target>,
    pub(crate) problem: Option<Problem>,
    pub(crate) presence: vessel_app_view::inbox::presence::Presence,
    pub(crate) reading: vessel_app_view::inbox::reading::Reader,
    pub(crate) read_open: Option<vessel_app_view::inbox::reading::Target>,
    pub(crate) read_last: Option<(
        vessel_app_view::inbox::Request,
        vessel_app_view::inbox::reading::Target,
    )>,
    pub(crate) read_problem: Option<Problem>,
    pub(crate) test_notice: vessel_app_view::inbox::settings::TestNotice,
    pub(crate) presence_problems: Vec<(vessel_proto::NodeId, Problem)>,
    pub(crate) tool: Option<ToolView>,
    pub(crate) notice_routes: vessel_app_view::inbox::notices::NoticeRoutes,
    pub(crate) notices: vessel_app_view::inbox::notices::NoticeQueue,
}
impl InboxArea {
    pub(crate) fn new(
        window: &mut vessel_app_kit::ui::Window,
        cx: &mut Context<'_, Shell>,
    ) -> Self {
        notifications::listen(window, cx);
        presence::listen(window, cx);
        idle::listen(window, cx);
        Self {
            store: Inbox::default(),
            window: window.window_handle(),
            admission: vessel_app_view::inbox::admission::Gate::default(),
            granted: Vec::new(),
            detail: Detail::default(),
            conversation: ConversationArea::new(cx),
            targets: Vec::new(),
            problem: None,
            presence: vessel_app_view::inbox::presence::Presence::default(),
            presence_problems: Vec::new(),
            reading: vessel_app_view::inbox::reading::Reader::default(),
            read_open: None,
            read_last: None,
            read_problem: None,
            test_notice: vessel_app_view::inbox::settings::TestNotice::Idle,
            tool: None,
            notice_routes: vessel_app_view::inbox::notices::NoticeRoutes::default(),
            notices: vessel_app_view::inbox::notices::NoticeQueue::default(),
        }
    }
}
