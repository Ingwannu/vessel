//! Clone shorthand selects a transport explicitly; complete remotes retain their own scheme.
use clap::{Args, ValueEnum};
use std::path::PathBuf;
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum CloneProtocol {
    Https,
    Ssh,
}
#[derive(Debug, Args)]
pub(crate) struct Clone {
    pub(crate) repo: String,
    #[arg(long)]
    pub(crate) dir: PathBuf,
    #[arg(long)]
    pub(crate) protocol: Option<CloneProtocol>,
}
