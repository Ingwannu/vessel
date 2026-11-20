//! Preserve the reference device menu without treating an arbitrary string as a preset.
//!
//! Only reference-defined fixed sizes are constructed here. A custom agent
//! resize is validated by the node's wire type and converted by link; this
//! menu cannot silently clamp that reply to the nearest branded device.

use serde::{Deserialize, Serialize};

/// CSS pixels remain distinct from the pane's physical device pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CssPixels(u16);

impl CssPixels {
    /// Layout adapters can scale this explicitly for the surface being drawn.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// Menu sizes are positive constants, with no caller-controlled constructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedViewport {
    width: CssPixels,
    height: CssPixels,
}

impl FixedViewport {
    const fn new(width: u16, height: u16) -> Self {
        Self {
            width: CssPixels(width),
            height: CssPixels(height),
        }
    }

    /// The width to request from the shared browser target.
    #[must_use]
    pub const fn width(self) -> CssPixels {
        self.width
    }

    /// The height to request from the shared browser target.
    #[must_use]
    pub const fn height(self) -> CssPixels {
        self.height
    }
}

macro_rules! device_presets {
    ($($variant:ident => $getter:ident, $label:literal, $width:literal, $height:literal;)+) => {
        /// A stable preset identity survives language and pane-size changes.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum DevicePreset { $(#[doc = $label] $variant,)+ }

        impl DevicePreset {
            /// Reference order, shared by every menu implementation.
            pub const ALL: &'static [Self] = &[$(Self::$variant,)+];

            /// Branded device/model names are not fragmented translated sentences.
            #[must_use]
            pub fn label(self, words: crate::localization::Browser<'_>) -> &str {
                match self { $(Self::$variant => words.$getter(),)+ }
            }

            /// Dimensions exist once in this table, not in each screen or request.
            #[must_use]
            pub const fn size(self) -> FixedViewport {
                match self { $(Self::$variant => FixedViewport::new($width, $height),)+ }
            }
        }
    };
}

device_presets! {
    IphoneSe => device_iphone_se, "iPhone SE", 375, 667;
    Iphone14 => device_iphone14, "iPhone 14", 390, 844;
    Iphone14ProMax => device_iphone14_pro_max, "iPhone 14 Pro Max", 430, 932;
    Pixel7 => device_pixel7, "Pixel 7", 412, 915;
    GalaxyS20 => device_galaxy_s20, "Galaxy S20", 360, 800;
    IpadMini => device_ipad_mini, "iPad Mini", 768, 1024;
    IpadAir => device_ipad_air, "iPad Air", 820, 1180;
    IpadPro11 => device_ipad_pro11, "iPad Pro 11\"", 834, 1194;
    IpadPro12 => device_ipad_pro12, "iPad Pro 12.9\"", 1024, 1366;
    SurfacePro => device_surface_pro, "Surface Pro", 912, 1368;
    Laptop => device_laptop, "Laptop", 1366, 768;
    Desktop1080 => device_desktop1080, "Desktop 1080p", 1920, 1080;
    Desktop1440 => device_desktop1440, "Desktop 1440p", 2560, 1440;
}

/// Responsive remains responsive when pane geometry changes or is restored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "mode", content = "device", rename_all = "snake_case")]
pub enum Viewport {
    /// The pane supplies its latest bounds; no stale dimensions are persisted.
    #[default]
    Responsive,
    /// A fixed device is centered and clipped by the screen, never stretched.
    Device(DevicePreset),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn responsive_restores_without_freezing_a_previous_pane_size() {
        let viewport: Viewport = serde_json::from_str(r#"{"mode":"responsive"}"#).unwrap();
        assert_eq!(viewport, Viewport::default());
        let fixed = Viewport::Device(DevicePreset::Iphone14);
        assert_eq!(
            serde_json::from_str::<Viewport>(&serde_json::to_string(&fixed).unwrap()).unwrap(),
            fixed
        );
        assert!(
            serde_json::from_str::<Viewport>(r#"{"mode":"device","device":"unknown-phone"}"#)
                .is_err()
        );
    }

    #[test]
    fn prepared_device_labels_use_the_selected_catalog() {
        use crate::localization::{ENGLISH, LanguagePreference, Translator, resolve_language};
        let english = Translator::get(ENGLISH).unwrap().browser();
        let korean = Translator::get(resolve_language(
            LanguagePreference::System,
            Some("ko"),
            None,
        ))
        .unwrap()
        .browser();
        assert_eq!(DevicePreset::Laptop.label(english), "Laptop");
        assert_eq!(DevicePreset::Laptop.label(korean), "노트북");
        assert_ne!(
            DevicePreset::Desktop1080.label(english),
            DevicePreset::Desktop1080.label(korean)
        );
    }

    #[test]
    fn branded_device_menu_has_no_zero_size_or_duplicate_label() {
        let words = crate::localization::Translator::get(crate::localization::ENGLISH)
            .unwrap()
            .browser();
        let mut names = std::collections::HashSet::new();
        for device in DevicePreset::ALL {
            assert!(names.insert(device.label(words)));
            assert!(device.size().width().get() > 0);
            assert!(device.size().height().get() > 0);
        }
        assert_eq!(DevicePreset::ALL.first(), Some(&DevicePreset::IphoneSe));
        assert_eq!(DevicePreset::ALL.last(), Some(&DevicePreset::Desktop1440));
    }
}
