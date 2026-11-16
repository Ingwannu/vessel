//! The file view tolerates later nested keys; the RPC patch remains strict.

use serde::{Deserialize, Deserializer};
use vessel_proto::settings::SetNodeSettings;

mod view;

struct Stored(SetNodeSettings);

impl<'de> Deserialize<'de> for Stored {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        view::read(deserializer).map(Self)
    }
}

pub(super) fn decode(bytes: &[u8]) -> Result<SetNodeSettings, super::SettingsDecodeFailure> {
    // Keep the file-only view inside the common sealed diagnostic boundary.
    let stored: Stored = super::decode::settings(bytes)?;
    Ok(stored.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DataDir;
    use vessel_proto::{parking::ParkAfter, settings::MetadataEffort};

    #[test]
    fn later_nested_keys_keep_known_settings_and_the_shared_copy_policy() {
        let home = tempfile::tempdir().unwrap();
        let dir = DataDir::new(home.path());
        let raw = br#"{"park_after":{"kind":"never"},"metadata":{"future":true,"providers":[{"route":"fast","future":true,"effort":"future"}]}}"#;
        // Use the wire's own spelling for the unrelated setting.
        let mut value: serde_json::Value = serde_json::from_slice(raw).unwrap();
        value["park_after"] = serde_json::to_value(ParkAfter::Never).unwrap();
        std::fs::write(dir.settings_path(), serde_json::to_vec(&value).unwrap()).unwrap();
        let read = dir.read_settings().unwrap().unwrap();
        assert_eq!(read.park_after, Some(ParkAfter::Never));
        assert_eq!(
            read.metadata.as_ref().unwrap().providers()[0].effort,
            Some(MetadataEffort::Unknown)
        );
        assert!(dir.settings_hold_more_than(&read).unwrap());
        dir.write_settings(&read).unwrap();
        assert_eq!(dir.read_settings().unwrap(), Some(read));
        assert!(serde_json::from_value::<SetNodeSettings>(value).is_err());
    }

    #[test]
    fn known_bad_metadata_does_not_turn_into_automatic() {
        for raw in [
            br#"{"metadata":{"providers":true}}"#.as_slice(),
            br#"{"metadata":{"providers":[{"route":42}]}}"#,
        ] {
            assert!(decode(raw).is_err());
        }
    }
}

#[cfg(test)]
mod diagnostic_tests {
    use super::*;
    use std::error::Error as _;

    #[test]
    fn metadata_file_refusals_use_the_shared_sealed_diagnostic() {
        let home = tempfile::tempdir().unwrap();
        let dir = crate::DataDir::new(home.path());
        let marker = "synthetic-private-metadata-marker";
        let bytes = serde_json::to_vec(&serde_json::json!({
            "metadata": {"providers": marker}
        }))
        .unwrap();
        // This wrong string is quoted by serde itself: exercising the adapter
        // through the shared boundary must seal that very cause.
        let Err(raw) = serde_json::from_slice::<Stored>(&bytes) else {
            panic!("wrong metadata value parsed");
        };
        assert!(raw.to_string().contains(marker));
        std::fs::write(dir.settings_path(), bytes).unwrap();
        let error = dir.read_settings().unwrap_err();
        assert!(!format!("{error} {error:?}").contains(marker));
        let cause = error.source().unwrap();
        assert!(!format!("{cause} {cause:?}").contains(marker));
        assert!(cause.source().is_none());
    }

    #[test]
    fn ordinary_field_failures_keep_the_common_masked_path_in_either_order() {
        for bytes in [
            br#"{"drivers":"private-wrong-type"}"#.as_slice(),
            br#"{"metadata":{"providers":[]},"drivers":"private-wrong-type"}"#,
            br#"{"drivers":"private-wrong-type","metadata":{"providers":[]}}"#,
        ] {
            let error = decode(bytes).unwrap_err();
            assert_eq!(error.to_string(), "invalid field at $.<field>");
            assert!(error.source().is_none());
        }
    }

    #[test]
    fn a_root_type_failure_stays_at_the_root_and_duplicate_metadata_is_refused() {
        let error = decode(br#"[]"#).unwrap_err();
        assert_eq!(error.to_string(), "invalid field at $");
        for bytes in [
            br#"{"metadata":null,"metadata":null}"#.as_slice(),
            br#"{"metadata":{"providers":[]},"metadata":null}"#,
        ] {
            assert!(decode(bytes).is_err());
        }
    }
}
