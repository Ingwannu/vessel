//! Native discovery flags keep optional creation intent distinct from omission.
use clap::Args;
use vessel_proto::SessionListing;

#[derive(Debug, Args)]
pub(crate) struct CreationMetadata {
    /// Explicit session discovery classification (Vessel addition); omission inherits the caller.
    #[arg(long, value_name = "shown|background", value_parser = listing)]
    pub(crate) listing: Option<SessionListing>,
    /// Add a label as key=value (repeatable; same spelling and validation as agent run).
    #[arg(long, value_name = "key=value")]
    pub(crate) label: Vec<String>,
}
#[derive(Debug, Args)]
pub(crate) struct Discovery {
    /// Include background sessions (Vessel native listing filter, not a no-wait alias).
    #[arg(long)]
    pub(crate) background: bool,
    /// Require every current key=value label (repeatable; explicit metadata selection).
    #[arg(long, value_name = "key=value")]
    pub(crate) label: Vec<String>,
}
fn listing(text: &str) -> Result<SessionListing, &'static str> {
    match text {
        "shown" => Ok(SessionListing::Shown),
        "background" => Ok(SessionListing::Background),
        _ => Err("listing must be shown or background"),
    }
}
