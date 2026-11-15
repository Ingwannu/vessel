//! One catalogue table owns decoding and encoding; model selection never
//! matches raw text. This module does not download or load model weights.

use super::DictationDecodeError;
use super::settings_values::{MAX_MODEL_CHARS, MIN_TEXT_CHARS};
use crate::key::serde_via_text;
use std::{fmt, str::FromStr};

/// A future catalogue name, validated before it can survive a downgrade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownLocalModelId(String);

macro_rules! catalogue {
    ($( $variant:ident => $wire:literal, $why:literal; )*) => {
        /// Known local STT models, plus a validated future catalogue name.
        #[derive(Debug, Clone, PartialEq, Eq)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
        pub enum LocalModelId {
            $( #[doc = $why] $variant, )*
            /// A newer node's ID, retained on read and refused on write.
            Unknown(UnknownLocalModelId),
        }

        impl LocalModelId {
            /// Keep the saved default aligned with Paseo before linking an engine.
            pub const DEFAULT: Self = Self::ParakeetV2;
            /// Typed catalogue members pin which values this build accepts on write.
            #[cfg(test)]
            pub(super) const KNOWN: &[Self] = &[$( Self::$variant, )*];

            /// The default must not silently switch to an endpoint.
            #[must_use]
            pub const fn default_model() -> Self { Self::DEFAULT }

            /// Encode the validated catalogue ID at the wire/provider edge.
            #[must_use]
            pub fn as_str(&self) -> &str {
                match self {
                    $( Self::$variant => $wire, )*
                    Self::Unknown(id) => &id.0,
                }
            }

            /// Future values remain readable but may not be selected on write.
            #[must_use]
            pub const fn known(&self) -> bool {
                match self {
                    $( Self::$variant )|* => true,
                    Self::Unknown(_) => false,
                }
            }
        }

        impl FromStr for LocalModelId {
            type Err = DictationDecodeError;
            fn from_str(text: &str) -> Result<Self, Self::Err> {
                if !(MIN_TEXT_CHARS..=MAX_MODEL_CHARS).contains(&text.len()) || !text.bytes().all(|b|
                    b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-')) {
                    return Err(DictationDecodeError::LocalModel);
                }
                Ok(match text {
                    $( $wire => Self::$variant, )*
                    _ => Self::Unknown(UnknownLocalModelId(text.to_owned())),
                })
            }
        }
    };
}

catalogue! {
    ParakeetV2 => "parakeet-tdt-0.6b-v2-int8", "Paseo's default English model.";
    ParakeetSmall => "parakeet-tdt-110m-en-int8", "A smaller English Parakeet model.";
    ParakeetV3 => "parakeet-tdt-0.6b-v3-int8", "Parakeet's multilingual model.";
    ParakeetJapanese => "parakeet-tdt-ctc-0.6b-ja-int8", "Parakeet's Japanese model.";
    WhisperTinyEnglish => "whisper-tiny-en", "English Whisper Tiny.";
    WhisperBase => "whisper-base", "Multilingual Whisper Base.";
    WhisperTurbo => "whisper-turbo", "Multilingual Whisper Turbo.";
    SenseVoice => "sense-voice-int8", "`SenseVoice` for five languages.";
    Dolphin => "dolphin-base-int8", "Dolphin for Asian and Middle Eastern languages.";
    GigaAm => "giga-am-v3-ru", "`GigaAM` for Russian.";
}

impl fmt::Display for LocalModelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
serde_via_text!(LocalModelId);
