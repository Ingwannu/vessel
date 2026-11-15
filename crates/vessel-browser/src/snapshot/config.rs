//! Typed configuration is the one bridge from host bounds to our static script.
use super::graph::Role;
use super::{MAX_SNAPSHOT_DEPTH, MAX_SNAPSHOT_NODES};
use crate::references::{MAX_REFERENCES, SnapshotNamespace};
use serde::Serialize;
use vessel_proto::browser::MAX_PAGE_TEXT_BYTES;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Config {
    first_ref: String,
    limits: Limits,
    roles: &'static [&'static str],
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Limits {
    nodes: usize,
    references: usize,
    depth: u8,
    text_bytes: usize,
}
impl Config {
    pub(crate) fn new(namespace: SnapshotNamespace) -> Self {
        Self {
            first_ref: namespace.start_decimal(),
            limits: Limits {
                nodes: MAX_SNAPSHOT_NODES,
                references: MAX_REFERENCES,
                depth: MAX_SNAPSHOT_DEPTH,
                text_bytes: MAX_PAGE_TEXT_BYTES,
            },
            roles: Role::NAMES,
        }
    }
}
/// Our independently written function, never caller/reference JavaScript.
pub(crate) const SOURCE: &str = include_str!("traverse.js");
