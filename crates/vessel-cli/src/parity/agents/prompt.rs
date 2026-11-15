//! Prompt creation/delivery keeps unsupported launch settings ahead of all effects.

use super::{labels, receipt, structured};
use crate::parity::observe;
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
    if args.prompt.len() > crate::parity::prompt_input::MAX_BYTES {
        return Err(CliError::Usage("prompt exceeds the CLI input byte budget"));
    }
    if args.prompt.trim().is_empty() {
        return Err(CliError::Usage("the prompt is empty"));
    }
    let labels = labels(&args.label)?;
    let (provider, shorthand_model) = args.provider.into_parts();
    let model = match (args.model, shorthand_model) {
        (Some(explicit), Some(shorthand)) if explicit != shorthand => {
            return Err(CliError::Usage(
                "--model conflicts with the provider/model shorthand",
            ));
        }
        (Some(explicit), _) => Some(explicit),
        (None, shorthand) => shorthand,
    };
    if args.placement.requires_contract() {
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
        provider,
        SessionMode::Structured {
            model,
            resume: None,
            options: Vec::new(),
        },
    )
    .with_cwd(cwd)
    .with_labels(labels);
    spec.title = args.title;
    let client = connect(target).await?.client;
    let info = client.create_session(spec).await?;
    let delivery = client
        .prompt(
            info.id,
            vec![ContentBlock::Text { text: args.prompt }],
            Delivery::Start,
        )
        .await?;
    if args.no_wait {
        return receipt::run_started(presentation, &info, delivery).await;
    }
    let result = observe::wait(&client, info.id, args.wait_timeout).await?;
    receipt::run_finished(presentation, &info, result.status).await
}

pub(super) async fn send(
    target: &Target,
    presentation: Presentation,
    args: AgentSend,
) -> Result<(), CliError> {
    let text = match (args.text, args.prompt, args.prompt_file) {
        (Some(text), None, None) | (None, Some(text), None) => text,
        (None, None, Some(path)) => crate::parity::prompt_input::file(&path).await?,
        _ => {
            return Err(CliError::Usage(
                "give exactly one positional prompt, --prompt or --prompt-file",
            ));
        }
    };
    if text.len() > crate::parity::prompt_input::MAX_BYTES {
        return Err(CliError::Usage("prompt exceeds the CLI input byte budget"));
    }
    if text.trim().is_empty() {
        return Err(CliError::Usage("the prompt is empty"));
    }
    let images = crate::conversation::Images::open(&args.image).await?;
    let delivery = args.behavior.map_or_else(
        || {
            if std::env::var_os("VESSEL_AGENT_SESSION").is_some() {
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
    let mut content = images.stage(&client, info.id).await?;
    if !text.is_empty() {
        content.insert(0, ContentBlock::Text { text });
    }
    let receipt = client.prompt(info.id, content, delivery).await?;
    if args.no_wait {
        return receipt::send_started(presentation, info.id, receipt).await;
    }
    let result = observe::wait(&client, info.id, WaitBudget::DEFAULT).await?;
    receipt::send_finished(presentation, result).await
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
    receipt::imported(presentation, &info).await
}
