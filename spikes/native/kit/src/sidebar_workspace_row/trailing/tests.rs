use super::*;
use crate::{Scheme, Tokens};
fn data() -> AnyElement {
    div().w(px(50.0)).h(px(16.0)).into_any_element()
}
#[test]
fn workspace_trailing_hint_and_compact_menu_policy_preserves_the_source_slot() {
    let content = Trailing::default().content(data());
    assert_eq!(
        layout(&content, Hint::Absent, Density::Desktop),
        Layout {
            content: Content::Visible,
            action: Action::Absent,
            reserve: Reserve::Minimum
        }
    );
    assert_eq!(
        layout(&content, Hint::Shown, Density::Desktop),
        Layout {
            content: Content::Hidden,
            action: Action::Absent,
            reserve: Reserve::Minimum
        }
    );
    let action = Trailing::default().action(data());
    assert_eq!(
        layout(&action, Hint::Absent, Density::Desktop),
        Layout {
            content: Content::Absent,
            action: Action::Hover,
            reserve: Reserve::Zero
        }
    );
    let both = Trailing::default().content(data()).action(data());
    assert_eq!(
        layout(&both, Hint::Absent, Density::Compact),
        Layout {
            content: Content::Absent,
            action: Action::Shown,
            reserve: Reserve::Minimum
        }
    );
    assert_eq!(
        layout(&both, Hint::Shown, Density::Compact),
        Layout {
            content: Content::Absent,
            action: Action::Absent,
            reserve: Reserve::Minimum
        }
    );
    let retained = both.visibility(MenuVisibility::Retained);
    assert_eq!(
        layout(&retained, Hint::Absent, Density::Desktop),
        Layout {
            content: Content::Visible,
            action: Action::Shown,
            reserve: Reserve::Minimum
        }
    );
}
#[test]
fn workspace_scrim_uses_the_actual_row_rgb_and_only_changes_its_alpha_to_fifty_five_percent() {
    assert_eq!(SCRIM_WIDTH.to_bits(), 48.0f32.to_bits());
    for scheme in [Scheme::Dark, Scheme::Light] {
        let tokens = Tokens::of(scheme);
        for color in [
            tokens.neutral.surface_sidebar,
            tokens.neutral.surface1,
            tokens.neutral.surface2,
            tokens.neutral.surface3,
            tokens.neutral.surface4,
        ] {
            let [clear, solid] = stops(color);
            assert_eq!(clear.color, color.opacity(0.0));
            assert_eq!(solid.color, color);
            assert_eq!(clear.percentage.to_bits(), 0.0f32.to_bits());
            assert_eq!(solid.percentage.to_bits(), 0.55f32.to_bits());
        }
    }
}
