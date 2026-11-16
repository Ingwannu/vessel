//! Changing a live retention limit trims only evicted rows and never revives them.
use super::grid::{Glyph, Scroll};
use super::{GridSize, ScrollbackLines, TerminalModel, Viewport};

#[test]
fn history_limit_preserves_retained_anchor_then_clamps_and_zero_follows() {
    let mut model = TerminalModel::new(
        GridSize::new(12, 2).unwrap(),
        ScrollbackLines::new(8).unwrap(),
    );
    let _feed = model.replay(b"0\r\n1\r\n2\r\n3\r\n4\r\n5");
    let mut viewport = Viewport::default();
    viewport.earlier(Scroll(2), model.grid());
    let first = |model: &TerminalModel, viewport: Viewport| {
        model
            .snapshot(viewport.offset(model.grid()))
            .cells
            .first()
            .unwrap()
            .glyph()
    };
    assert_eq!(first(&model, viewport), Glyph::Scalar('2'));
    model.set_scrollback(ScrollbackLines::new(3).unwrap());
    viewport.refresh(model.grid());
    assert_eq!(model.grid().history_len(), 3);
    assert_eq!(first(&model, viewport), Glyph::Scalar('2'));
    model.set_scrollback(ScrollbackLines::new(1).unwrap());
    viewport.refresh(model.grid());
    assert_eq!(model.grid().history_len(), 1);
    assert_eq!(first(&model, viewport), Glyph::Scalar('3'));
    model.set_scrollback(ScrollbackLines::new(8).unwrap());
    assert_eq!(model.grid().history_len(), 1);
    let generation = model.grid().generation();
    model.set_scrollback(ScrollbackLines::new(8).unwrap());
    assert_eq!(model.grid().generation(), generation);
    model.set_scrollback(ScrollbackLines::new(0).unwrap());
    viewport.refresh(model.grid());
    assert_eq!(viewport.offset(model.grid()), Scroll(0));
    assert_eq!(model.grid().history_len(), 0);
    let _feed = model.replay(b"\r\n6\r\n7");
    assert_eq!(model.grid().history_len(), 0);
}

#[test]
fn scrollback_serialization_never_admits_an_invalid_persisted_count() {
    for count in [0, ScrollbackLines::DEFAULT.get(), ScrollbackLines::MAX] {
        let setting = ScrollbackLines::new(count).unwrap();
        let encoded = serde_json::to_string(&setting).unwrap();
        assert_eq!(encoded, count.to_string());
        assert_eq!(
            serde_json::from_str::<ScrollbackLines>(&encoded).unwrap(),
            setting
        );
    }
    let above = ScrollbackLines::MAX.checked_add(1).unwrap().to_string();
    for invalid in ["-1", above.as_str(), "null", "\"100\"", "1.5", "true"] {
        assert!(serde_json::from_str::<ScrollbackLines>(invalid).is_err());
    }
}
