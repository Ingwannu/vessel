use super::*;

#[test]
fn the_rows_follow_paseos_widths() {
    assert_eq!(rows(0.0, true), ROWS, "not measured yet");
    assert_eq!(rows(299.0, true), THREE_COLUMN_ROWS);
    assert_eq!(rows(300.0, true), ROWS);
    assert_eq!(rows(659.0, false), ROWS);
    assert_eq!(rows(660.0, false), WIDE_ROWS);
    assert_eq!(rows(880.0, true), ROWS, "compact keeps two rows");
}

#[test]
fn every_table_holds_the_same_controls() {
    let mut controls: Vec<Vec<Control>> = [ROWS, WIDE_ROWS, THREE_COLUMN_ROWS]
        .iter()
        .map(|table| {
            let mut all: Vec<Control> = table.iter().flat_map(|row| row.iter().copied()).collect();
            all.sort_by_key(|control| format!("{control:?}"));
            all
        })
        .collect();
    let first = controls.remove(0);
    assert_eq!(first.len(), 12);
    for other in controls {
        assert_eq!(other, first);
    }
}

#[test]
fn a_set_modifier_goes_with_the_next_bar_key_then_clears() {
    let mut sticky = Sticky::default();
    sticky.toggle(Ctrl);
    assert!(sticky.is_set(Ctrl));
    assert_eq!(
        press(Up, &mut sticky, CursorKeys::Normal),
        "\x1b[1;5A"
    );
    assert!(sticky.is_empty());
    assert_eq!(
        press(Up, &mut sticky, CursorKeys::Application),
        "\x1bOA"
    );
    sticky.toggle(Shift);
    assert_eq!(
        press(Tab, &mut sticky, CursorKeys::Normal),
        "\x1b[Z"
    );
    assert_eq!(
        press(Escape, &mut sticky, CursorKeys::Normal),
        "\x1b"
    );
    assert_eq!(press(Enter, &mut sticky, CursorKeys::Normal), "\r");
}

#[test]
fn toggling_twice_clears_it() {
    let mut sticky = Sticky::default();
    sticky.toggle(Alt);
    sticky.toggle(Alt);
    assert!(sticky.is_empty());
}

#[test]
fn typed_text_takes_the_set_modifiers_as_paseo_maps_it() {
    let mut sticky = Sticky::default();
    assert_eq!(
        typed("c", &mut sticky, CursorKeys::Normal),
        None,
        "none set"
    );
    sticky.toggle(Ctrl);
    assert_eq!(
        typed("c", &mut sticky, CursorKeys::Normal).as_deref(),
        Some("\x03")
    );
    assert!(sticky.is_empty());
    sticky.toggle(Alt);
    assert_eq!(
        typed("x", &mut sticky, CursorKeys::Normal).as_deref(),
        Some("\x1bx")
    );
    sticky.toggle(Ctrl);
    assert_eq!(
        typed("\r", &mut sticky, CursorKeys::Normal).as_deref(),
        Some("\r")
    );
    // Not one mapped character: as typed, and the modifiers clear.
    sticky.toggle(Ctrl);
    assert_eq!(
        typed("ab", &mut sticky, CursorKeys::Normal).as_deref(),
        Some("ab")
    );
    assert!(sticky.is_empty());
    sticky.toggle(Ctrl);
    assert_eq!(
        typed("한", &mut sticky, CursorKeys::Normal).as_deref(),
        Some("한")
    );
    assert!(sticky.is_empty());
}

#[test]
fn a_keyboard_key_takes_the_set_modifiers_with_those_held() {
    let mut sticky = Sticky::default();
    assert_eq!(
        pressed(Key::Char('c'), Mods::NONE, &mut sticky, CursorKeys::Normal),
        None
    );
    sticky.toggle(Ctrl);
    assert_eq!(
        pressed(
            Key::Named(Named::Left),
            Mods::SHIFT,
            &mut sticky,
            CursorKeys::Normal
        )
        .as_deref(),
        Some("\x1b[1;6D")
    );
    assert!(sticky.is_empty());
}
