//! Checked opaque request bodies and pure validation; no key, file or socket work.

use super::{
    super::{
        config::{Provider, Routes},
        metadata::{self, SelectError, Selection},
    },
    text::{self, Material, TextError},
};
use bytes::Bytes;
use std::{fmt, num::NonZeroU32};
use vessel_llm::{
    client::Api,
    completion::{OutputTokens, RequestError},
    ir,
};
use vessel_proto::settings::MetadataSettings;
use vessel_workspace::project_file::metadata::StyleText;

/// The title exchange has one output budget, shared by validation and use.
pub(super) const TITLE_OUTPUT_TOKENS: u32 = 512;

/// Only the bounded encoder constructs this outside tests.
pub(super) struct Prepared {
    api: Api,
    body: Bytes,
}
impl Prepared {
    pub(super) const fn api(&self) -> Api {
        self.api
    }
    pub(super) fn into_body(self) -> Bytes {
        self.body
    }
    #[cfg(test)]
    pub(super) const fn fixture(api: Api, body: Bytes) -> Self {
        Self { api, body }
    }
}
impl fmt::Debug for Prepared {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Prepared(..)")
    }
}

#[derive(Debug, thiserror::Error)]
pub(super) enum PrepareError {
    #[error("{0}")]
    Candidate(#[from] SelectError),
    #[error("{0}")]
    Text(#[from] TextError),
    #[error("{0}")]
    Request(#[from] RequestError),
    #[error("title output budget {TITLE_OUTPUT_TOKENS} must be positive")]
    Budget,
}

/// Every manual candidate is checked before the later settings owner mutates state.
pub(super) fn validate(catalog: &Routes, settings: &MetadataSettings) -> Result<(), PrepareError> {
    for entry in settings.providers() {
        let selected = metadata::candidate(catalog, entry)?;
        let neutral = text::neutral(selected.model(), selected.effort())?;
        let _prepared = encode(selected.provider(), neutral)?;
    }
    Ok(())
}

/// Material/style add no second encoder policy after pure settings validation.
pub(super) fn title(
    selected: &Selection<'_>,
    material: &Material,
    style: Option<&StyleText>,
) -> Result<Prepared, PrepareError> {
    let prompt = text::request(selected.model(), selected.effort(), material, style)?;
    encode(selected.provider(), prompt.into_request())
}

fn output_limit() -> Result<OutputTokens, PrepareError> {
    NonZeroU32::new(TITLE_OUTPUT_TOKENS)
        .map(OutputTokens::new)
        .ok_or(PrepareError::Budget)
}

fn encode(provider: &Provider, neutral: ir::Request) -> Result<Prepared, PrepareError> {
    let body = provider
        .upstream_options()
        .complete(neutral, output_limit()?)?;
    Ok(Prepared {
        api: provider.flavor.api(),
        body,
    })
}

#[cfg(test)]
mod tests;
