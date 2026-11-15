//! Reported catalogue caps are shared by driver conversion and wire decoding.

use super::*;
use crate::{ChoiceValue, HarnessKind, OptionChoice};

fn toggle(id: &str) -> SessionOption {
    SessionOption {
        id: OptionId::new(id),
        name: "control".to_owned(),
        description: None,
        category: OptionCategory::Other,
        kind: OptionKind::Toggle { on: true },
    }
}
fn decode(json: &str) -> Result<LaunchCatalogueOptions, serde_json::Error> {
    serde_json::from_str(json)
}

#[test]
fn categories_are_hints_and_do_not_change_the_opaque_choice() {
    let raw = OptionChoice {
        id: OptionId::new("opaque"),
        value: ChoiceValue::Toggle { on: true },
    };
    let choice = super::super::LaunchChoice::try_from(raw.clone())
        .unwrap()
        .with_category(OptionCategory::Mode);
    let encoded = serde_json::to_string(&choice).unwrap();
    assert_eq!(
        serde_json::from_str::<super::super::LaunchChoice>(&encoded).unwrap(),
        choice
    );
    assert_eq!(choice.as_choice(), &raw);
    let legacy: super::super::LaunchChoice =
        serde_json::from_str(r#"{"id":"opaque","kind":"toggle","on":true}"#).unwrap();
    assert_eq!(legacy.category(), OptionCategory::Other);
    assert_eq!(legacy.as_choice(), &raw);
}

#[test]
fn controls_values_text_and_whole_payload_have_finite_admission() {
    let full = (0..LaunchCatalogueOptions::MAX_OPTIONS)
        .map(|n| toggle(&format!("id{n}")))
        .collect::<Vec<_>>();
    let value = LaunchCatalogueOptions::try_from(full.clone()).unwrap();
    assert_eq!(
        decode(&serde_json::to_string(&value).unwrap()).unwrap(),
        value
    );
    let mut too_many = full;
    too_many.push(toggle("extra"));
    assert!(LaunchCatalogueOptions::try_from(too_many.clone()).is_err());
    assert!(decode(&serde_json::to_string(&too_many).unwrap()).is_err());
    let mut long = toggle("id");
    long.name = "x".repeat(LaunchCatalogueOptions::MAX_LABEL_BYTES);
    assert!(LaunchCatalogueOptions::try_from(vec![long.clone()]).is_ok());
    long.name.push('x');
    assert!(LaunchCatalogueOptions::try_from(vec![long]).is_err());
    let values = (0..LaunchCatalogueOptions::MAX_VALUES)
        .map(|n| OptionValue {
            id: OptionValueId::new(format!("v{n}")),
            name: "name".to_owned(),
            description: None,
            group: None,
        })
        .collect::<Vec<_>>();
    let mut select = toggle("select");
    select.kind = OptionKind::Select {
        current: OptionValueId::new("v0"),
        values: values.clone(),
    };
    let full = LaunchCatalogueOptions::try_from(vec![select.clone()]).unwrap();
    assert_eq!(
        decode(&serde_json::to_string(&full).unwrap()).unwrap(),
        full
    );
    let mut extra = values;
    extra.push(OptionValue {
        id: OptionValueId::new("extra"),
        name: "extra".to_owned(),
        description: None,
        group: None,
    });
    select.kind = OptionKind::Select {
        current: OptionValueId::new("v0"),
        values: extra,
    };
    assert!(LaunchCatalogueOptions::try_from(vec![select.clone()]).is_err());
    assert!(decode(&serde_json::to_string(&vec![select]).unwrap()).is_err());
}

#[test]
fn inconsistent_unknown_or_null_fields_fail_whole_without_echoing_input() {
    for body in [
        r#"[{"id":"i","name":"n","category":"other","kind":"toggle","on":true,"current":"bad"}]"#,
        r#"[{"id":"i","name":"n","category":"other","kind":"toggle","on":null}]"#,
        r#"[{"id":"i","name":"n","category":"other","kind":"select","current":"v","values":null}]"#,
        r#"[{"id":"i","name":"n","category":"other","kind":"toggle","on":true,"private_unknown":1}]"#,
    ] {
        let error = decode(body).unwrap_err();
        assert!(!error.to_string().contains("private_unknown"));
    }
    let unknown = r#"[{"id":"i","name":"n","category":"future","kind":"future"}]"#;
    let read = decode(unknown).unwrap();
    assert_eq!(read.as_slice()[0].kind, OptionKind::Unknown);
    let empty = LaunchCatalogueOptions::try_from(Vec::new()).unwrap();
    let result = LaunchCatalogue::new(
        LaunchSelection::new(
            HarnessKind::Codex.into(),
            None,
            super::super::LaunchChoices::default(),
        ),
        LaunchAccount::new("codex".parse().unwrap(), LaunchAccountKind::Default),
        empty,
        DateTime::UNIX_EPOCH,
    )
    .unwrap();
    assert_eq!(
        serde_json::from_str::<LaunchCatalogue>(&serde_json::to_string(&result).unwrap()).unwrap(),
        result
    );
}
