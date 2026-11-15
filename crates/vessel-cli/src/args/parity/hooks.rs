//! Hook payload interpretation belongs to W7's terminal-activity boundary.
use clap::Args;

#[derive(Debug, Args)]
pub(crate) struct HookInvocation {
    pub(crate) provider: String,
    pub(crate) event: String,
}
