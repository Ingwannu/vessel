//! Typed test fixture shared by the two candidate arithmetic probes.

const SOURCE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/number-format/intl-en-ko.json"
));

#[derive(Debug, serde::Deserialize)]
pub(in super::super) struct Oracle {
    pub(in super::super) amounts: Vec<Amount>,
    pub(in super::super) counts: Vec<Count>,
}

impl Oracle {
    pub(in super::super) fn read() -> Result<Self, serde_json::Error> {
        serde_json::from_str(SOURCE)
    }

    pub(in super::super) fn read_extended() -> Result<Self, serde_json::Error> {
        serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/number-format/intl-en-ko-v2.json"
        )))
    }
}

#[derive(Debug, serde::Deserialize)]
pub(in super::super) struct Amount {
    pub(in super::super) name: String,
    pub(in super::super) bits: String,
    pub(in super::super) output: Outputs,
}

#[derive(Debug, serde::Deserialize)]
pub(in super::super) struct Outputs {
    pub(in super::super) en: Styles,
    pub(in super::super) ko: Styles,
}

#[derive(Debug, serde::Deserialize)]
pub(in super::super) struct Styles {
    pub(in super::super) usd: String,
    pub(in super::super) tokens: String,
    pub(in super::super) decimal: String,
}

#[derive(Debug, serde::Deserialize)]
pub(in super::super) struct Count {
    pub(in super::super) unsigned: String,
    pub(in super::super) output: CountOutputs,
}

#[derive(Debug, serde::Deserialize)]
pub(in super::super) struct CountOutputs {
    pub(in super::super) en: String,
    pub(in super::super) ko: String,
}
