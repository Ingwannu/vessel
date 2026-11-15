//! Native system observation schemas; they neither approve nor execute anything.
use serde::Deserialize;
use vessel_proto::TokenCount;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct Status {
    #[serde(default)]
    pub(crate) status: Option<StatusKind>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StatusKind {
    Compacting,
    #[serde(other)]
    Other,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct CompactBoundary {
    #[serde(default, alias = "compactMetadata", alias = "compactionMetadata")]
    pub(crate) compact_metadata: Option<CompactMetadata>,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct CompactMetadata {
    #[serde(default)]
    pub(crate) trigger: Option<Trigger>,
    #[serde(default, alias = "preTokens")]
    pub(crate) pre_tokens: Option<TokenCount>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Trigger {
    Auto,
    Manual,
    #[serde(other)]
    Other,
}
