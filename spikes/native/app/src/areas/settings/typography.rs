//! The sole preference-to-native typography adapter. Font enumeration happens
//! on hydration/commit, never in paint; only available face names reach kit.
//! Unknown names fall back to embedded faces without opening paths or URLs.
use crate::shell::Shell;
use vessel_app_kit::{
    fonts::{FontSize, FontStack, TextSizes, Typeface},
    ui::{App, SharedString},
};
use vessel_app_view::{
    appearance::{FamilyFace, FontFamily, GenericFamily},
    prefs::Appearance,
};

pub(super) fn apply(shell: &Shell, cx: &mut App) {
    let appearance = &shell.prefs.settings.appearance;
    let Some(sizes) = sizes(appearance) else {
        tracing::error!("validated Appearance contains a zero text size");
        return;
    };
    let available = cx.text_system().all_font_names();
    let interface = stack(
        &appearance.interface_family,
        Typeface::Interface,
        &available,
    );
    let code = stack(&appearance.mono_family, Typeface::Code, &available);
    vessel_app_kit::set_font_stacks(cx, interface, code);
    vessel_app_kit::set_typography_sizes(cx, sizes);
    vessel_app_kit::set_syntax_palette(
        cx,
        super::appearance::syntax_palette(appearance.syntax_theme),
    );
}

fn sizes(appearance: &Appearance) -> Option<TextSizes> {
    Some(TextSizes {
        interface: FontSize::new(appearance.interface_size.get())?,
        content: FontSize::new(appearance.content_size.get())?,
        code: FontSize::new(appearance.code_size.get())?,
    })
}

pub(super) fn stack(family: &FontFamily, role: Typeface, available: &[String]) -> FontStack {
    use vessel_app_kit::fonts::{HANGUL, MONO, SANS};
    let names = family.faces().iter().filter_map(|face| match face {
        FamilyFace::Named(name) => available
            .iter()
            .find(|font| font.eq_ignore_ascii_case(name.as_str()))
            .map(|font| SharedString::from(font.to_owned())),
        FamilyFace::Generic(generic) => Some(match generic {
            GenericFamily::Sans => SANS.into(),
            GenericFamily::Mono => MONO.into(),
            GenericFamily::Readable => HANGUL.into(),
        }),
    });
    FontStack::from_names(names, role)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unavailable_quoted_names_fall_through_and_generics_are_actual_embedded_faces() {
        let available = ["Available Face".to_owned()];
        let family = FontFamily::parse("'Missing, Face', 'available face', monospace").unwrap();
        let resolved = stack(&family, Typeface::Interface, &available);
        assert_eq!(resolved.family(), "Available Face");
        assert_eq!(
            resolved.fallbacks().fallback_list(),
            &[vessel_app_kit::fonts::MONO, vessel_app_kit::fonts::HANGUL]
        );
        let missing = stack(
            &FontFamily::parse("Unknown").unwrap(),
            Typeface::Code,
            &available,
        );
        assert_eq!(missing.family(), vessel_app_kit::fonts::MONO);
        let generic = stack(
            &FontFamily::parse("serif").unwrap(),
            Typeface::Interface,
            &available,
        );
        assert_eq!(generic.family(), vessel_app_kit::fonts::HANGUL);
    }
    #[test]
    fn saved_axes_map_to_absolute_units_without_sharing_a_scale() {
        let mut appearance = Appearance::default();
        let before = sizes(&appearance).unwrap();
        appearance.interface_size = vessel_app_view::prefs::InterfaceSize::try_from(18).unwrap();
        let after = sizes(&appearance).unwrap();
        assert_eq!(after.interface.get(), 18);
        assert_eq!(after.content, before.content);
        assert_eq!(after.code, before.code);
    }
}
