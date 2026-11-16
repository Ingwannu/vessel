//! Ordinary-page function results, independently implemented from documented behavior.
//!
//! This is intentionally separate from isolated snapshot/wait realms. Serialized
//! function input cannot select a CDP method, private realm, host bridge or file.
//! An optional element requires the future checked element resolver; this path
//! accepts no element and must not be used to ignore a requested reference.
use serde::{Deserialize, Serialize};
use vessel_proto::browser::{
    CappedText, MAX_PAGE_TEXT_BYTES, PageText, RequiredInput, TextCompleteness,
};

const SOURCE: &str = include_str!("function.js");
/// Preserve the reference's smaller preview while measuring both limits in UTF-8 bytes.
const MAX_EVALUATE_PREVIEW_BYTES: usize = 79_000;

#[derive(Serialize)]
struct Configuration<'a> {
    source: &'a RequiredInput,
    limit: usize,
    preview: usize,
}

/// Construct one ordinary-page expression from fixed source and serialized input.
/// Caller code is intentionally executable only inside the supplied function.
pub(crate) fn expression(source: &RequiredInput) -> Result<RequiredInput, EvaluationError> {
    let configuration = serde_json::to_string(&Configuration {
        source,
        limit: MAX_PAGE_TEXT_BYTES,
        preview: MAX_EVALUATE_PREVIEW_BYTES,
    })
    .map_err(EvaluationError::Encode)?;
    RequiredInput::try_from(format!("({SOURCE})({configuration})")).map_err(EvaluationError::Input)
}

/// The remote result is page-controlled data, never an authority or diagnostic.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Outcome {
    Value {
        text: PageText,
        completeness: TextCompleteness,
    },
    NotFunction,
    Exception,
}
#[derive(Deserialize)]
struct Returned {
    value: Outcome,
}

pub(crate) fn finish(
    result: &crate::cdp::requests::Evaluated,
) -> Result<CappedText, EvaluationError> {
    match result
        .remote_object::<Returned>()
        .map_err(EvaluationError::Remote)?
        .value
    {
        Outcome::Value { text, completeness } => {
            // Validate syntax without a generic JSON tree or interpreting user fields.
            let _json: &serde_json::value::RawValue =
                serde_json::from_str(text.as_str()).map_err(EvaluationError::Result)?;
            Ok(CappedText { text, completeness })
        }
        Outcome::NotFunction => Err(EvaluationError::NotFunction),
        Outcome::Exception => Err(EvaluationError::Exception),
    }
}

#[derive(thiserror::Error)]
pub(crate) enum EvaluationError {
    #[error("page function configuration could not be encoded")]
    Encode(#[source] serde_json::Error),
    #[error("page function expression exceeds the input boundary")]
    Input(#[source] vessel_proto::browser::InvalidBrowserInput),
    #[error("page function response could not be decoded")]
    Remote(#[source] crate::cdp::requests::EvaluationError),
    #[error("page function result is not valid JSON")]
    Result(#[source] serde_json::Error),
    #[error("browser evaluation requires a function")]
    NotFunction,
    #[error("page function threw or could not serialize its result")]
    Exception,
}
impl std::fmt::Debug for EvaluationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BrowserFunctionError({self})")
    }
}

#[cfg(test)]
mod tests;
