//! The `/` menu's placement over its composer card.

use vessel_app_kit::theme::spacing;

use super::*;

#[test]
fn the_list_comes_to_rest_at_paseos_offset_above_the_card() {
    let open = LIFT.mul_add(1.0, OFFSET_LOW);
    assert!((open - spacing::S3).abs() < 1e-6, "{open}");
}

#[test]
fn each_composer_s_list_has_its_own_id() {
    assert_ne!(Over::Session.id(), Over::Draft.id());
}

#[test]
fn the_menu_keeps_paseos_offset_from_the_top_of_the_content() {
    assert!((UNDER_BAR + chrome::HEIGHT + spacing::S3).abs() < 1e-6);
    assert!((AT_TOP + spacing::S3).abs() < 1e-6);
}

#[test]
fn the_room_is_what_lies_above_the_menu_and_never_less_than_none() {
    let taken = if chrome::system_titlebar() {
        AT_TOP
    } else {
        UNDER_BAR
    };
    assert_eq!(room_above(px(300.0)), px(300.0 + taken));
    assert_eq!(room_above(px(10.0)), px(0.0), "no room is no room");
}
