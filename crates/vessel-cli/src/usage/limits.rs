//! Typed CLI quota requests. The node, rather than the command, owns policy
//! and refresh state; a returned error never includes a login or API body.
use crate::{error::CliError, io};
use vessel_client::NodeClient;
use vessel_proto::limits::{
    ExternalLookup, GetUsageLimits, LimitAccount, LimitRefresh, LimitsScope, SetUsageLookup,
};

pub(super) async fn account(
    client: &NodeClient,
    account: LimitAccount,
    refresh: bool,
) -> Result<(), CliError> {
    read(client, LimitsScope::Account { account }, refresh).await
}
pub(super) async fn session(
    client: &NodeClient,
    session: vessel_proto::SessionId,
    refresh: bool,
) -> Result<(), CliError> {
    read(client, LimitsScope::Session { session }, refresh).await
}
async fn read(client: &NodeClient, scope: LimitsScope, refresh: bool) -> Result<(), CliError> {
    let refresh = if refresh {
        LimitRefresh::Refresh
    } else {
        LimitRefresh::Cached
    };
    let report = client
        .usage_limits(GetUsageLimits { scope, refresh })
        .await?;
    let text = super::limits_render::render(&report, chrono::Utc::now())?;
    Ok(io::out(text.as_bytes()).await?)
}
pub(super) async fn lookup(client: &NodeClient, policy: ExternalLookup) -> Result<(), CliError> {
    let saved = client.usage_lookup(SetUsageLookup { policy }).await?;
    let text = match saved {
        ExternalLookup::Off => {
            "External limit lookup is off. Cached external observations have been cleared.\n"
        }
        ExternalLookup::On => {
            "External limit lookup is enabled on this machine. A requested refresh reads the configured agent's saved access token (and Codex account identifier, when present) here and sends it only to Anthropic or ChatGPT to read account limits. Sign-in refresh stays with the agent.\n"
        }
    };
    Ok(io::out(text.as_bytes()).await?)
}
