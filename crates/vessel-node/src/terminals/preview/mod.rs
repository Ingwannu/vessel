//! Staged Preview matcher/context values do not grant private network access.
//! Real canonical producers and the browser integration gate must qualify
//! admission before this module can enter the product configuration.

mod context;
mod origin;
#[cfg(test)]
mod tests;

pub(crate) use context::{PreviewContext, PreviewScope};
pub(crate) use origin::{PreviewOrigin, PreviewScheme};

/// Typed matcher errors do not include a caller URL or process evidence.
#[derive(Debug, thiserror::Error)]
pub(crate) enum PreviewError {
    #[error("service Preview requires a literal loopback origin")]
    NotLoopback,
    #[error("service Preview requires a TCP service port")]
    Port(#[source] vessel_proto::workspace::InvalidServicePort),
}
