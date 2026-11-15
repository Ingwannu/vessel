//! The kinds and paths a decode failure reports, from real serde errors.

use super::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sample {
    count: u32,
}

fn failure(text: &str) -> DecodeFailure {
    let mut frame = serde_json::Deserializer::from_str(text);
    let error = serde_path_to_error::deserialize::<_, Sample>(&mut frame).unwrap_err();
    DecodeFailure::new(Some(Method::NodeInfo), Some(error.path()), error.inner())
}

#[test]
fn each_mistake_has_its_kind_and_place_and_no_value() {
    let wrong = failure(r#"{"count":"seven-secret"}"#);
    assert_eq!(wrong.kind(), DecodeKind::WrongType);
    assert_eq!(
        wrong.to_string(),
        "`node.info`: a value of the wrong type at `<field>`"
    );
    assert!(!format!("{wrong:?}").contains("seven-secret"));

    assert_eq!(failure(r"{}").kind(), DecodeKind::MissingField);
    assert_eq!(
        failure(r#"{"count":1,"Secret-Name":2}"#).kind(),
        DecodeKind::UnknownName
    );
    assert_eq!(failure(r#"{"count":"#).kind(), DecodeKind::Truncated);
    assert_eq!(failure(r"{count:1}").kind(), DecodeKind::Syntax);
    assert_eq!(failure(r#"{"count":-1}"#).kind(), DecodeKind::InvalidValue);
}

#[test]
fn a_well_formed_sample_decodes() {
    let sample: Sample = serde_json::from_str(r#"{"count":7}"#).unwrap();
    assert_eq!(sample.count, 7);
}

#[test]
fn trailing_bytes_after_the_value_fail_it() {
    assert!(whole::<Sample>(r#"{"count":1}"#).is_ok());
    assert!(
        whole::<Sample>("{\"count\":1}  \n").is_ok(),
        "trailing whitespace is fine"
    );
    assert!(whole::<Sample>(r#"{"count":1} x"#).is_err());
    assert!(whole::<Sample>(r#"{"count":1}{"count":2}"#).is_err());
}
