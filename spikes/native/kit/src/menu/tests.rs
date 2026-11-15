//! Values and selection guards taken from the reference components, not tokens.
use super::*;
use super::{
    draw,
    item::{self, Leading, Paint},
    trigger,
};
use crate::{Scheme, Tokens, assets::Glyph, popup::Surfaces};
use gpui::{Styled as _, point, px};

#[test]
fn legacy_constructors_and_pending_labels_preserve_their_meaning() {
    let plain = MenuItem::new("Run now");
    assert_eq!(plain.tone, Tone::Plain);
    assert_eq!(plain.leading, None);
    assert_eq!(plain.status, ItemStatus::Idle);
    assert!(!plain.starts_group);
    assert!(plain.selectable());
    let danger = MenuItem::danger("Delete");
    assert_eq!(danger.tone, Tone::Danger);
    assert!(danger.starts_group);
    assert_eq!(danger.words().as_ref(), "Delete");
    let pending = plain.with_leading(Glyph::RotateCw).pending("Starting...");
    assert!(!pending.disabled);
    assert!(!pending.selectable());
    assert_eq!(pending.label.as_ref(), "Run now");
    assert_eq!(pending.words().as_ref(), "Starting...");
    assert_eq!(pending.leading(), Leading::Spinner);
    assert_eq!(
        MenuItem::new("Edit").with_leading(Glyph::Pencil).leading(),
        Leading::Glyph(Glyph::Pencil)
    );
}

#[test]
fn pending_items_are_skipped_and_cannot_be_committed_after_highlight() {
    let items = [
        MenuItem::new("Pause").pending("Pausing..."),
        MenuItem::new("Run now"),
        MenuItem::danger("Delete"),
    ];
    assert_eq!(next_index(&items, None, Direction::Next), Some(1));
    assert_eq!(next_index(&items, Some(2), Direction::Next), Some(1));
    assert_eq!(next_index(&items, Some(1), Direction::Previous), Some(2));
    assert_eq!(next_index(&items, None, Direction::Previous), Some(2));
    assert_eq!(selected_index(&items, Some(0)), None);
    assert_eq!(selected_index(&items, Some(1)), Some(1));
    assert_eq!(selected_index(&items, Some(99)), None);
    let blocked = [MenuItem::new("Pause").pending("Pausing...")];
    assert_eq!(next_index(&blocked, Some(0), Direction::Next), None);
    assert_eq!(next_index(&[], None, Direction::Previous), None);
    let mut changed = items;
    changed[1].disabled = true;
    assert_eq!(selected_index(&changed, Some(1)), None);
    changed[2] = changed[2].clone().pending("Deleting...");
    assert_eq!(next_index(&changed, None, Direction::Next), None);
}

#[test]
fn row_paint_matches_reference_colours_and_disabled_opacity_in_both_themes() {
    for (scheme, foreground, muted, wash, destructive) in [
        (
            Scheme::Light,
            0x001a_1a1e,
            0x0071_717a,
            0x00f4_f4f5,
            0x00b0_4138,
        ),
        (
            Scheme::Dark,
            0x00fa_fafa,
            0x00a1_a5a4,
            0x0027_2a29,
            0x00c6_4f43,
        ),
    ] {
        let surfaces = Surfaces::of(&Tokens::of(scheme));
        for (item, ink, glyph) in [
            (MenuItem::new("Edit"), foreground, muted),
            (MenuItem::danger("Delete"), destructive, destructive),
        ] {
            assert_eq!(
                item::paint(&item, false, &surfaces),
                Paint {
                    ink: gpui::rgb(ink).into(),
                    glyph: gpui::rgb(glyph).into(),
                    fill: gpui::transparent_black(),
                    opacity: 1.0
                }
            );
            assert_eq!(
                item::paint(&item, true, &surfaces),
                Paint {
                    ink: gpui::rgb(ink).into(),
                    glyph: gpui::rgb(glyph).into(),
                    fill: gpui::rgb(wash).into(),
                    opacity: 1.0
                }
            );
            let pending = item.clone().pending("Working...");
            assert_eq!(
                item::paint(&pending, true, &surfaces),
                Paint {
                    ink: gpui::rgb(ink).into(),
                    glyph: gpui::rgb(muted).into(),
                    fill: gpui::transparent_black(),
                    opacity: 0.5
                }
            );
            assert_eq!(pending.leading(), Leading::Spinner);
            let mut disabled = item;
            disabled.disabled = true;
            assert_eq!(
                item::paint(&disabled, true, &surfaces),
                Paint {
                    ink: gpui::rgb(ink).into(),
                    glyph: gpui::rgb(glyph).into(),
                    fill: gpui::transparent_black(),
                    opacity: 0.5
                }
            );
        }
        assert_eq!(
            trigger::paint(&Tokens::of(scheme), false),
            trigger::Paint {
                ink: gpui::rgb(muted).into(),
                fill: gpui::transparent_black()
            }
        );
        assert_eq!(
            trigger::paint(&Tokens::of(scheme), true),
            trigger::Paint {
                ink: gpui::rgb(foreground).into(),
                fill: gpui::rgb(wash).into()
            }
        );
    }
}

#[test]
fn menu_and_trigger_geometry_keep_the_reference_values() {
    assert_eq!(draw::MIN_WIDTH, px(180.0));
    assert_eq!(draw::MAX_WIDTH, px(320.0));
    assert_eq!(draw::LEADING_SLOT, px(16.0));
    assert_eq!(px(LeadingSize::Small.points()), px(14.0));
    assert_eq!(px(LeadingSize::Regular.points()), px(16.0));
    assert_eq!(crate::loading::Points::SIXTEEN.pixels(), px(16.0));
    assert_eq!(px(draw::ROW_LINE), px(18.0));
    assert_eq!(px(draw::OFFSET), px(4.0));
    assert_eq!(px(crate::popup::ROW_RADIUS), px(6.0));
    assert_eq!(Width::Content, Content::new("legacy", &[]).width);
    assert_eq!(
        Width::Fixed(px(220.0)),
        Content::new("schedules", &[]).with_width(px(220.0)).width
    );
    for (sizing, minimum) in [
        (TriggerSizing::Pointer, px(20.0)),
        (TriggerSizing::Touch, px(44.0)),
    ] {
        assert_eq!(
            trigger::geometry(sizing),
            trigger::Geometry {
                minimum,
                visual: match sizing {
                    TriggerSizing::Pointer => px(22.0),
                    TriggerSizing::Touch => px(44.0),
                },
                hit: match sizing {
                    TriggerSizing::Pointer => px(38.0),
                    TriggerSizing::Touch => px(60.0),
                },
                padding: px(4.0),
                glyph: px(14.0),
                radius: px(4.0)
            }
        );
    }
    assert_eq!(px(crate::popup::row_height(false)), px(28.0));
    assert_eq!(px(crate::popup::row_height(true)), px(44.0));
}

#[test]
fn end_alignment_uses_actual_trigger_bounds_not_the_click_location() {
    let bounds = Bounds {
        origin: point(px(100.0), px(20.0)),
        size: gpui::size(px(44.0), px(44.0)),
    };
    assert_eq!(
        Placement::End(bounds).geometry(gpui::Anchor::TopLeft),
        (point(px(144.0), px(64.0)), gpui::Anchor::TopRight)
    );
    assert_eq!(
        Placement::Pointer(point(px(5.0), px(8.0))).geometry(gpui::Anchor::TopLeft),
        (point(px(5.0), px(8.0)), gpui::Anchor::TopLeft)
    );
}

#[test]
fn a_removed_highlight_restarts_navigation_in_both_directions() {
    let items = [
        MenuItem::new("Pause").pending("Pausing..."),
        MenuItem::new("Run now"),
        MenuItem::danger("Delete"),
    ];
    for stale in [items.len(), 99, usize::MAX] {
        assert_eq!(next_index(&items, Some(stale), Direction::Next), Some(1));
        assert_eq!(
            next_index(&items, Some(stale), Direction::Previous),
            Some(2)
        );
        assert_eq!(selected_index(&items, Some(stale)), None);
    }
}

#[test]
fn git_header_visual_and_hit_regions_do_not_resize_the_schedule_profile() {
    for (sizing, visual, hit) in [
        (TriggerSizing::Pointer, 26.0, 42.0),
        (TriggerSizing::Touch, 32.0, 48.0),
    ] {
        assert_eq!(
            trigger::git_geometry(sizing),
            trigger::GitGeometry {
                visual: px(visual),
                hit: px(hit),
                outset: px(8.0),
                glyph: px(16.0),
                radius: px(6.0),
            }
        );
        let schedule = trigger::geometry(sizing);
        assert_eq!(schedule.glyph, px(14.0));
        assert_eq!(schedule.padding, px(4.0));
        assert_eq!(schedule.radius, px(4.0));
    }
}

#[test]
fn each_actual_svg_has_its_own_ink_in_both_themes_and_profiles() {
    for (scheme, muted) in [(Scheme::Light, 0x0071_717a), (Scheme::Dark, 0x00a1_a5a4)] {
        let tokens = Tokens::of(scheme);
        for profile in [TriggerProfile::Schedule, TriggerProfile::GitHeader] {
            for sizing in [TriggerSizing::Pointer, TriggerSizing::Touch] {
                for state in [crate::ButtonState::Enabled, crate::ButtonState::Disabled] {
                    let mut icon = trigger::icon(profile, sizing, state, &tokens);
                    assert_eq!(icon.style().text.color, Some(gpui::rgb(muted).into()));
                }
            }
        }
    }
}

#[gpui::test]
fn pointer_and_keyboard_share_close_and_keep_open_policy(cx: &mut gpui::TestAppContext) {
    // Paseo actions-split-button.tsx passes closeOnSelect=false to direct
    // overflow writes. Modal openers and existing callers keep the default.
    cx.update(|cx| {
        assert_eq!(MenuItem::new("Write").dismissal, Dismissal::Close);
        for keyboard in [false, true] {
            for (dismissal, remains_open) in
                [(Dismissal::Close, false), (Dismissal::KeepOpen, true)]
            {
                let item = MenuItem::new("Write").with_dismissal(dismissal);
                let mut state = MenuState::new(cx);
                state.at = Some(Placement::Pointer(point(px(10.0), px(20.0))));
                state.highlighted = Some(0);
                let selected = if keyboard {
                    keyboard_choice(&mut state, &[item])
                } else {
                    pointer_choice(&mut state, &item, 0)
                };
                assert_eq!(selected, Some(0));
                assert_eq!(state.is_open(), remains_open);
            }
        }
    });
}

#[gpui::test]
fn pending_disabled_and_stale_choices_do_not_activate_or_dismiss(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        let pending = MenuItem::new("Write")
            .with_dismissal(Dismissal::KeepOpen)
            .pending("Writing...");
        assert_eq!(pending.dismissal, Dismissal::KeepOpen);
        let mut disabled = MenuItem::new("Write");
        disabled.disabled = true;
        for item in [
            pending,
            disabled,
            MenuItem::new("Write")
                .with_dismissal(Dismissal::KeepOpen)
                .success("Done"),
        ] {
            for keyboard in [false, true] {
                let mut state = MenuState::new(cx);
                state.at = Some(Placement::Pointer(point(px(10.0), px(20.0))));
                state.highlighted = Some(0);
                let selected = if keyboard {
                    keyboard_choice(&mut state, std::slice::from_ref(&item))
                } else {
                    pointer_choice(&mut state, &item, 0)
                };
                assert_eq!(selected, None);
                assert!(state.is_open());
                state.highlighted = Some(99);
                assert_eq!(
                    keyboard_choice(&mut state, std::slice::from_ref(&item)),
                    None
                );
                state.close();
                assert!(!state.is_open());
            }
        }
    });
}

#[test]
fn success_uses_the_existing_circle_check_and_fixed_green_not_semantic_success() {
    for scheme in [Scheme::Light, Scheme::Dark] {
        let surfaces = Surfaces::of(&Tokens::of(scheme));
        for base in [MenuItem::new("Push"), MenuItem::danger("Delete")] {
            let item = base.with_dismissal(Dismissal::KeepOpen).success("Done");
            assert_eq!(item.words().as_ref(), "Done");
            assert_eq!(item.leading(), Leading::SuccessCheck);
            assert!(!item.selectable());
            assert_eq!(item.dismissal, Dismissal::KeepOpen);
            assert_eq!(draw::SUCCESS_GLYPH, px(16.0));
            assert_eq!(
                item::paint(&item, true, &surfaces),
                Paint {
                    ink: gpui::rgb(0x0022_c55e).into(),
                    glyph: gpui::rgb(0x0022_c55e).into(),
                    fill: gpui::transparent_black(),
                    opacity: 0.5,
                }
            );
            assert_eq!(selected_index(std::slice::from_ref(&item), Some(0)), None);
            assert_eq!(
                next_index(&[item, MenuItem::new("Other")], Some(1), Direction::Next),
                Some(1)
            );
        }
    }
    assert_eq!(crate::theme::palette::GREEN_500, 0x0022_c55e);
    assert_eq!(Glyph::CircleCheck.path(), "kit/circle-check.svg");
}

#[test]
fn both_legacy_entries_keep_their_point_corner_and_measured_ends_stay_right() {
    use gpui::{Anchor, Bounds, point, px, size};
    let at = point(px(5.0), px(8.0));
    assert_eq!(
        Placement::Pointer(at).geometry(Anchor::TopLeft),
        (at, Anchor::TopLeft)
    );
    assert_eq!(
        Placement::Pointer(at).geometry(Anchor::TopRight),
        (at, Anchor::TopRight)
    );
    let bounds = Bounds {
        origin: at,
        size: size(px(20.0), px(25.0)),
    };
    for legacy_corner in [Anchor::TopLeft, Anchor::TopRight] {
        assert_eq!(
            Placement::End(bounds).geometry(legacy_corner),
            (point(bounds.right(), bounds.bottom()), Anchor::TopRight)
        );
    }
}
