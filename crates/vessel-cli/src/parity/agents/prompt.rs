//! Prompt creation/delivery keeps unsupported launch settings ahead of all effects.

use super::{labels, structured};
use crate::parity::{observe, output};
use crate::{
    args::{
        Target,
        parity::{AgentRun, AgentSend, Presentation, WaitBudget},
    },
    connect::connect,
    error::CliError,
};
use std::path::PathBuf;
use vessel_proto::{
    ContentBlock, Delivery, HarnessRef, HarnessSessionId, SessionMode, SessionSpec,
};

pub(super) async fn run(
    target: &Target,
    presentation: Presentation,
    args: AgentRun,
) -> Result<(), CliError> {
    if args.prompt.trim().is_empty() {
        return Err(CliError::Usage("the prompt is empty"));
    }
    let labels = labels(&args.label)?;
    if args.workspace.is_some() {
        return Err(CliError::ParityBlocked(
            "--workspace requires the reviewed workspace placement contract",
        ));
    }
    if args.mode.is_some() || args.thinking.is_some() || !args.env.is_empty() {
        return Err(CliError::ParityBlocked(
            "launch mode/thinking/environment requires validated atomic provider creation",
        ));
    }
    if args.output_schema.is_some() {
        return Err(CliError::ParityBlocked(
            "--output-schema requires the bounded schema execution contract",
        ));
    }
    if !args.image.is_empty() {
        return Err(CliError::ParityBlocked(
            "initial image input requires validated creation attachment staging",
        ));
    }
    let cwd = args.cwd.unwrap_or(std::env::current_dir()?);
    if !cwd.is_absolute() {
        return Err(CliError::Usage("--cwd must be absolute on the target node"));
    }
    let mut spec = SessionSpec::new(
        args.provider,
        SessionMode::Structured {
            model: args.model,
            resume: None,
            options: Vec::new(),
        },
    )
    .with_cwd(cwd)
    .with_labels(labels);
    spec.title = args.title;
    let client = connect(target).await?.client;
    let info = client.create_session(spec).await?;
    let _delivery = client
        .prompt(
            info.id,
            vec![ContentBlock::Text { text: args.prompt }],
            Delivery::Start,
        )
        .await?;
    if args.no_wait {
        return output::single(presentation, &info.id.to_string(), &info).await;
    }
    let result = observe::wait(&client, info.id, args.wait_timeout).await?;
    output::single(presentation, &info.id.to_string(), &result).await
}

pub(super) async fn send(
    target: &Target,
    presentation: Presentation,
    args: AgentSend,
) -> Result<(), CliError> {
    if !args.image.is_empty() {
        return Err(CliError::ParityBlocked(
            "--image requires validated attachment staging before prompt delivery",
        ));
    }
    let text = match (args.text, args.prompt, args.prompt_file) {
        (Some(text), None, None) | (None, Some(text), None) => text,
        (None, None, Some(path)) => tokio::fs::read_to_string(path).await?,
        _ => {
            return Err(CliError::Usage(
                "give exactly one positional prompt, --prompt or --prompt-file",
            ));
        }
    };
    if text.trim().is_empty() {
        return Err(CliError::Usage("the prompt is empty"));
    }
    let delivery = args.behavior.map_or_else(
        || {
            if std::env::var_os("VESSEL_SESSION_ID").is_some() {
                Delivery::Steer
            } else {
                Delivery::Interrupt
            }
        },
        Into::into,
    );
    let client = connect(target).await?.client;
    let info = args.id.resolve(&client).await?;
    structured(&info)?;
    let receipt = client
        .prompt(info.id, vec![ContentBlock::Text { text }], delivery)
        .await?;
    if args.no_wait {
        return output::single(presentation, &info.id.to_string(), &receipt).await;
    }
    let result = observe::wait(&client, info.id, WaitBudget::DEFAULT).await?;
    output::single(presentation, &info.id.to_string(), &result).await
}

pub(super) async fn import(
    target: &Target,
    presentation: Presentation,
    id: HarnessSessionId,
    provider: HarnessRef,
    cwd: Option<PathBuf>,
    raw_labels: Vec<String>,
) -> Result<(), CliError> {
    let labels = labels(&raw_labels)?;
    let cwd = cwd.unwrap_or(std::env::current_dir()?);
    if !cwd.is_absolute() {
        return Err(CliError::Usage("--cwd must be absolute on the target node"));
    }
    let spec = SessionSpec::new(
        provider,
        SessionMode::Structured {
            model: None,
            resume: Some(id),
            options: Vec::new(),
        },
    )
    .with_cwd(cwd)
    .with_labels(labels);
    let client = connect(target).await?.client;
    let info = client.create_session(spec).await?;
    output::single(presentation, &info.id.to_string(), &info).await
}
