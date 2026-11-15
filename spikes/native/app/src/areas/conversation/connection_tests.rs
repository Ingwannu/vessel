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
