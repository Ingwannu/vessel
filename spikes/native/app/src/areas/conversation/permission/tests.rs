//! Disclosure belongs to one exact approval, even when its successor is also a plan.
use super::*;
use crate::shell::language_tests;
use vessel_proto::{PermissionRequest, SessionId, ToolCallId};

fn request() -> PermissionRequest {
    PermissionRequest {
        id: PermissionId::random(),
        tool_call: ToolCallId::new("plan"),
        options: Vec::new(),
    }
}

#[gpui::test]
fn a_folded_plan_does_not_fold_its_successor_or_accept_an_old_toggle(
    cx: &mut gpui::TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, _, cx| {
        let first = request();
        let next = request();
        shell
            .areas
            .conversation
            .open("scratch", SessionId::random());
        shell.areas.conversation.live.permissions = vec![first.clone()];
        shell.permit_plan_toggle(Seat::SESSION, first.id, cx);
        assert_eq!(shell.areas.conversation.plan_folded, Some(first.id));
        shell.permit_plan_toggle(Seat::SESSION, first.id, cx);
        assert_eq!(shell.areas.conversation.plan_folded, None);
        shell.permit_plan_toggle(Seat::SESSION, first.id, cx);
        shell.areas.conversation.live.permissions = vec![next.clone()];
        // The old retained flag does not describe the next request; neither can
        // an event queued from the previous chevron alter the new plan.
        assert_ne!(shell.areas.conversation.plan_folded, Some(next.id));
        shell.permit_plan_toggle(Seat::SESSION, first.id, cx);
        assert_eq!(shell.areas.conversation.plan_folded, Some(first.id));
        shell.permit_plan_toggle(Seat::SESSION, next.id, cx);
        assert_eq!(shell.areas.conversation.plan_folded, Some(next.id));
        shell.permit_plan_toggle(Seat::SESSION, first.id, cx);
        assert_eq!(shell.areas.conversation.plan_folded, Some(next.id));
    });
}
