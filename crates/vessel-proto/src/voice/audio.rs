//! Distinct capture/playback PCM prevents an output packet becoming microphone input.

use std::fmt;

use bytes::Bytes;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{MAX_CAPTURE_BYTES, MAX_PLAYBACK_BYTES, VoiceDecodeError};

macro_rules! pcm {
    ($(#[$meta:meta])* $name:ident, $cap:ident) => {
        $(#[$meta])*
        #[derive(Clone, PartialEq, Eq)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
        pub struct $name(Bytes);

        impl $name {
            /// Keep complete samples only; every accepted buffer has a known upper bound.
            ///
            /// # Errors
            /// Empty, odd or oversized buffers cannot become audio packets.
            pub fn new(bytes: impl Into<Bytes>) -> Result<Self, VoiceDecodeError> {
                let bytes = bytes.into();
                if bytes.is_empty() || !bytes.len().is_multiple_of(size_of::<i16>()) || bytes.len() > $cap {
                    return Err(VoiceDecodeError::Pcm);
                }
                Ok(Self(bytes))
            }

            /// Borrow PCM at codec/device edges without duplicating retained speech.
            #[must_use]
            pub fn as_slice(&self) -> &[u8] {
                &self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct(stringify!($name)).field("bytes", &self.0.len()).finish()
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                crate::base64_bytes::serialize(&self.0, serializer)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let bytes = crate::base64_bytes::deserialize_bounded::<D, $cap>(deserializer)?;
                Self::new(bytes).map_err(serde::de::Error::custom)
            }
        }
    };
}

pcm! {
    /// At most 100 ms of 16 kHz mono PCM16 microphone input, memory only.
    CapturePcm, MAX_CAPTURE_BYTES
}

pcm! {
    /// Bounded synthesized mono PCM16, interpreted only with its declared sample rate.
    PlaybackPcm, MAX_PLAYBACK_BYTES
}
