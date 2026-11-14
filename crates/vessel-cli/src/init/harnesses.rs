//! The harnesses this machine has, and install offers for what it lacks:
//! the vendor's command, run only after a yes (plan decision 9), with its
//! output shown as it runs.

use std::process::Stdio;

use vessel_harness::{InstallPart, discover, missing};
use vessel_proto::{HarnessKind, StructuredSupport};

use super::{Init, Outcome};
use crate::error::CliError;

/// The agents a first run reports as missing and offers to install: the
/// four Vessel began with. The seven of ADR 0058 are reported when found
/// and installed from the agent list (`agent.install`) when a person asks
/// there, so a first run neither asks seven more questions nor runs
/// another package manager (Bun) on a yes meant for the others.
const OFFERED: [HarnessKind; 4] = [
    HarnessKind::ClaudeCode,
    HarnessKind::Codex,
    HarnessKind::Gemini,
    HarnessKind::OpenCode,
];

pub(super) async fn run(init: &mut Init) -> Result<(), CliError> {
    let found = discover(&init.lookup).await;
    for harness in HarnessKind::ALL {
        let step = format!("harness {harness}");
        match found.iter().find(|installed| installed.kind == harness) {
            Some(installed) => {
                let version = installed.version.as_deref().unwrap_or("version unknown");
                let structured = match &installed.structured {
                    StructuredSupport::Available => "structured sessions ready",
                    StructuredSupport::Unavailable { .. } => "terminal sessions only",
                };
                init.report(Outcome::Already, &step, &format!("{version}; {structured}"))
                    .await?;
            }
            None if OFFERED.contains(&harness) => {
                init.report(Outcome::Skipped, &step, "not installed")
                    .await?;
            }
            None => {}
        }
    }
    let offered = missing(&init.lookup)
        .into_iter()
        .filter(|install| OFFERED.contains(&install.harness));
    for install in offered {
        let what = match install.part {
            InstallPart::Harness => install.harness.to_string(),
            InstallPart::Adapter => format!("{}'s ACP adapter", install.harness),
        };
        let step = format!("install {what}");
        let Some(program) = init.lookup.find(install.program) else {
            init.report(
                Outcome::Skipped,
                &step,
                &format!("needs `{}`, which is not installed", install.program),
            )
            .await?;
            init.todo
                .push(format!("install {what}: `{}`", install.command));
            continue;
        };
        let question = format!("Install {what} with `{}`?", install.command);
        match init.confirm_install(&question).await? {
            Some(true) => {
                let status = tokio::process::Command::new(&program)
                    .args(install.args())
                    .stdin(Stdio::null())
                    .status()
                    .await?;
                if status.success() {
                    init.report(Outcome::Done, &step, install.command).await?;
                } else {
                    init.report(
                        Outcome::Skipped,
                        &step,
                        &format!("`{}` failed ({status})", install.command),
                    )
                    .await?;
                    init.todo
                        .push(format!("install {what}: `{}`", install.command));
                }
            }
            Some(false) => {
                init.report(Outcome::Skipped, &step, "not run (you said no)")
                    .await?;
                init.todo
                    .push(format!("install {what}: `{}`", install.command));
            }
            None => {
                init.report(
                    Outcome::Skipped,
                    &step,
                    "not run (--yes alone never installs; add --install)",
                )
                .await?;
                init.todo
                    .push(format!("install {what}: `{}`", install.command));
            }
        }
    }
    Ok(())
}
