//! TTS identifiers reuse bounded metadata validation, without runtime/model selection.

use super::VoiceSettingsError;
use crate::{
    dictation::{EndpointModel, LocalModelId},
    key::serde_via_text,
};
use std::{fmt, str::FromStr};

macro_rules! endpoint_value {
    ($name:ident, $default:literal, $why:literal) => {
        #[doc = $why]
        #[derive(Clone, Default, PartialEq, Eq)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
        pub struct $name(Option<EndpointModel>);
        impl $name {
            /// The default is represented without a fallible parse or allocation.
            #[must_use]
            pub fn as_str(&self) -> &str {
                self.0.as_ref().map_or($default, EndpointModel::as_str)
            }
        }
        impl FromStr for $name {
            type Err = VoiceSettingsError;
            fn from_str(text: &str) -> Result<Self, Self::Err> {
                let value = text
                    .parse::<EndpointModel>()
                    .map_err(VoiceSettingsError::Metadata)?;
                Ok(Self((value.as_str() != $default).then_some(value)))
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!(stringify!($name), "([redacted])"))
            }
        }
        serde_via_text!($name);
    };
}
endpoint_value!(
    TtsModel,
    "tts-1",
    "A bounded endpoint synthesis model, distinct from a voice identifier."
);
endpoint_value!(
    TtsVoice,
    "alloy",
    "A bounded endpoint voice identifier, never interchangeable with a synthesis model."
);

/// A validated future catalogue token; no unvalidated arbitrary string survives a read.
#[derive(Clone, PartialEq, Eq)]
pub struct UnknownTtsModelId(LocalModelId);

macro_rules! tts_catalogue {
    ($( $variant:ident => $wire:literal, $why:literal; )*) => {
        /// Reference TTS catalogue metadata only. No variant permits download, load or readiness.
        #[derive(Clone, PartialEq, Eq)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
        pub enum LocalTtsModelId {
            $( #[doc = $why] $variant, )*
            /// A future or wrong-role ID stays readable and is refused on a new selection.
            Unknown(UnknownTtsModelId),
        }
        impl LocalTtsModelId {
            /// Metadata default from the pinned reference, not an installed model.
            pub const DEFAULT: Self = Self::KokoroEnglish;
            /// Closed options for a settings editor; readiness is a separate decision.
            pub const KNOWN: &[Self] = &[$(Self::$variant,)*];
            /// Encode a validated catalogue token only at an explicit wire/display edge.
            #[must_use]
            pub fn as_str(&self) -> &str {
                match self { $( Self::$variant => $wire, )* Self::Unknown(id) => id.0.as_str() }
            }
            /// Future values must not be newly selected by an older build.
            #[must_use]
            pub const fn known(&self) -> bool {
                match self { $(Self::$variant)|* => true, Self::Unknown(_) => false }
            }
        }
        impl Default for LocalTtsModelId { fn default() -> Self { Self::DEFAULT } }
        impl FromStr for LocalTtsModelId {
            type Err = VoiceSettingsError;
            fn from_str(text: &str) -> Result<Self, Self::Err> {
                // The existing catalogue parser owns ASCII/length/token rules.
                let token = text.parse::<LocalModelId>().map_err(VoiceSettingsError::Metadata)?;
                Ok(match text { $( $wire => Self::$variant, )* _ => Self::Unknown(UnknownTtsModelId(token)) })
            }
        }
    };
}
tts_catalogue! {
    KokoroEnglish => "kokoro-en-v0_19", "Reference English metadata; espeak data is not approved for distribution.";
    KokoroEnglishInt8 => "kokoro-int8-en-v0_19", "Reference quantized English metadata only.";
    KokoroChineseEnglishInt8 => "kokoro-int8-multi-lang-v1_1", "Reference Chinese/English metadata only.";
    KittenNanoEnglish => "kitten-nano-en-v0_2-fp16", "Reference Kitten v0.2 metadata only.";
    PiperUsEnglish => "piper-en-us-libritts-r", "Reference American English metadata only.";
    PiperBritishEnglish => "piper-en-gb-alba", "Reference British English metadata only.";
    PiperGerman => "piper-de-thorsten", "Reference German metadata only.";
    PiperFrench => "piper-fr-siwis", "Reference French metadata only.";
    PiperSpanish => "piper-es-davefx", "Reference Spanish metadata only.";
}
impl fmt::Debug for UnknownTtsModelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("UnknownTtsModelId([redacted])")
    }
}
impl fmt::Debug for LocalTtsModelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LocalTtsModelId([redacted])")
    }
}
impl fmt::Display for LocalTtsModelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
serde_via_text!(LocalTtsModelId);
