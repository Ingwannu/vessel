//! The exported decoder must remain safe under ordinary whole-chain logging.
use std::error::Error;
use vessel_harness::{AccountUsageOutcome, decode_account_usage};
use vessel_proto::usage_reports::BuiltinUsageSource;

const MARKER: &str = "PRIVATE-VENDOR-CANARY";

fn safe_chain(error: &(dyn Error + 'static)) {
    let mut current = Some(error);
    while let Some(error) = current {
        assert!(!error.to_string().contains(MARKER));
        assert!(!format!("{error:?}").contains(MARKER));
        assert!(!format!("{error:#?}").contains(MARKER));
        assert!(error.downcast_ref::<serde_json::Error>().is_none());
        current = error.source();
    }
}

#[test]
fn every_vendor_root_string_is_sealed_at_the_public_decode_boundary() {
    let body = format!("\"{MARKER}\"");
    for source in [
        BuiltinUsageSource::Copilot,
        BuiltinUsageSource::Cursor,
        BuiltinUsageSource::Grok,
        BuiltinUsageSource::Kimi,
        BuiltinUsageSource::MiniMax,
        BuiltinUsageSource::OpenCodeGo,
        BuiltinUsageSource::Zai,
    ] {
        let error = decode_account_usage(source, body.as_bytes()).unwrap_err();
        safe_chain(&error);
        assert!(error.source().is_some());
    }
}

#[test]
fn wrong_known_field_and_parser_forms_cannot_escape_but_valid_reports_remain_valid() {
    for body in [
        format!(r#"{{"planUsage":"{MARKER}"}}"#),
        format!(r#"{{"planUsage":{{"totalPercentUsed":{{"{MARKER}":true}}}}}}"#),
        format!(r#"{{"planUsage":{{}},"{MARKER}":"unterminated"#),
        format!(r#"{{}} "{MARKER}""#),
    ] {
        let error = decode_account_usage(BuiltinUsageSource::Cursor, body.as_bytes()).unwrap_err();
        safe_chain(&error);
    }
    for (source, body) in [
        (
            BuiltinUsageSource::Copilot,
            br#"{"copilot_plan":"business"}"#.as_slice(),
        ),
        (
            BuiltinUsageSource::Cursor,
            br#"{"planUsage":{"totalPercentUsed":70}}"#.as_slice(),
        ),
    ] {
        assert!(matches!(
            decode_account_usage(source, body).unwrap(),
            AccountUsageOutcome::Available(_)
        ));
    }
}
