//! Conversation state extraction retains connection recovery.

use super::ConversationArea;

#[gpui::test]
fn opening_or_receiving_this_session_clears_its_lost_connection(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        let first = "00000000-0000-4000-8000-000000000003".parse().unwrap();
        let other = "00000000-0000-4000-8000-000000000004".parse().unwrap();
        let mut area = ConversationArea::new(cx);
        area.open("machine", first);
        area.lost = Some("offline".to_owned());
        assert!(!area.take(&other, &[]));
        assert_eq!(area.lost.as_deref(), Some("offline"));
        assert!(!area.take(&first, &[]));
        assert_eq!(area.lost, None);
        area.lost = Some("offline".to_owned());
        area.open("machine", other);
        assert_eq!(area.lost, None);
        assert_eq!(area.shown.as_ref().unwrap().session, other);
    });
}

#[gpui::test]
fn the_lost_line_clears_once_when_the_link_comes_back(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        let session = "00000000-0000-4000-8000-000000000003".parse().unwrap();
        let mut area = ConversationArea::new(cx);
        area.open("machine", session);
        area.lost = Some("offline".to_owned());
        // No earlier sample: an online read is not an edge.
        assert!(!super::intake::follow_edge(false, false, true, true));
        assert!(!area.link_edge(true));
        assert_eq!(area.lost.as_deref(), Some("offline"));
        // Still online, including after the follow fails again.
        assert!(!super::intake::follow_edge(true, true, true, true));
        assert!(!area.link_edge(true));
        assert_eq!(area.lost.as_deref(), Some("offline"));
        // Offline leaves the line.
        assert!(!super::intake::follow_edge(true, true, false, true));
        assert!(!area.link_edge(false));
        assert_eq!(area.lost.as_deref(), Some("offline"));
        // The one offline→online edge clears it.
        assert!(super::intake::follow_edge(true, false, true, true));
        assert!(area.link_edge(true));
        assert_eq!(area.lost, None);
        // The follow failed while the link stayed online: the line stays.
        area.lost = Some("gone".to_owned());
        assert!(!area.link_edge(true));
        assert_eq!(area.lost.as_deref(), Some("gone"));
        // Online with no lost line is not an edge.
        assert!(!super::intake::follow_edge(true, false, true, false));
        area.lost = None;
        assert!(!area.link_edge(false));
        assert!(!area.link_edge(true));
        assert_eq!(area.lost, None);
        // A new session forgets the sample, so its first online read
        // is not an edge either.
        let other = "00000000-0000-4000-8000-000000000004".parse().unwrap();
        area.open("machine", other);
        area.lost = Some("offline".to_owned());
        assert!(!area.link_edge(true));
        assert_eq!(area.lost.as_deref(), Some("offline"));
    });
}
