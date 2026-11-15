//! Settings output distinguishes an unavailable feature from a confirmed Off.
use crate::args::{ResumeOnStartup, Target};
use crate::connect::connect;
use crate::error::CliError;
use crate::{io, render};
use vessel_client::CallError;
use vessel_proto::restart::SetRestartRecovery;
use vessel_proto::settings::{AgentDriver, GetNodeSettings, SetAgentDrivers, SetNodeSettings};
use vessel_proto::{HarnessKind, automation::PromptText, parking::ParkAfter};

pub(crate) async fn run(
    target: &Target,
    park_after: Option<ParkAfter>,
    drivers: &[(HarnessKind, AgentDriver)],
    startup: Option<ResumeOnStartup>,
    prompt: Option<PromptText>,
) -> Result<(), CliError> {
    let client = connect(target).await?.client;
    let drivers = driver_change(drivers)?;
    let recovery = (startup.is_some() || prompt.is_some()).then(|| SetRestartRecovery {
        enabled: startup.map(ResumeOnStartup::enabled),
        prompt,
    });
    let changing = park_after.is_some() || drivers.is_some() || recovery.is_some();
    let changing_recovery = recovery.is_some();
    let settings = if changing {
        client
            .call(SetNodeSettings {
                park_after,
                drivers,
                restart_recovery: recovery,
            })
            .await?
    } else {
        client.call(GetNodeSettings::default()).await?
    };
    let mut text = render::node_settings(&settings);
    if changing_recovery {
        text.push_str(&recovery_text(&settings.restart_recovery));
    } else if !changing {
        match client.restart_recovery().await {
            Ok(policy) => text.push_str(&recovery_text(&policy)),
            Err(CallError::UnassignedProtocol { .. } | CallError::NotInProtocol { .. }) => {
                text.push_str("startup recovery: unsupported by this protocol\n");
            }
            Err(error) => return Err(error.into()),
        }
    }
    Ok(io::out(text.as_bytes()).await?)
}
fn recovery_text(policy: &vessel_proto::restart::RestartRecovery) -> String {
    format!(
        "startup recovery: {}\nresume prompt: {}\n",
        if policy.enabled { "on" } else { "off" },
        policy.prompt.as_str()
    )
}

/// `--driver` values as a change; `None` without any.
///
/// # Errors
/// An agent with no protocol of its own (Gemini).
fn driver_change(
    drivers: &[(HarnessKind, AgentDriver)],
) -> Result<Option<SetAgentDrivers>, CliError> {
    if drivers.is_empty() {
        return Ok(None);
    }
    let mut change = SetAgentDrivers::default();
    for (kind, driver) in drivers {
        let slot = match kind {
            HarnessKind::ClaudeCode => &mut change.claude_code,
            HarnessKind::Codex => &mut change.codex,
            HarnessKind::OpenCode => &mut change.open_code,
            // `--driver` refuses it already.
            HarnessKind::Gemini => {
                return Err(CliError::Usage(
                    "gemini has no protocol of its own; it always runs over ACP",
                ));
            }
        };
        *slot = Some(*driver);
    }
    Ok(Some(change))
}
