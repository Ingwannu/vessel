//! ADR0079 native author scaffolding, independent from node installation.
mod scaffold;
mod template;
#[cfg(test)]
mod tests;

use crate::{
    args::{Target, parity::Presentation, plugin::PluginCommand},
    error::CliError,
    parity::output,
};

pub(crate) use scaffold::InitError;

pub(crate) async fn run(
    _target: &Target,
    presentation: Presentation,
    command: PluginCommand,
) -> Result<(), CliError> {
    match command {
        PluginCommand::Init { directory, id } => {
            // The complete local operation owns its arguments through blocking
            // filesystem work, even if its awaiting caller is dropped.
            let created =
                tokio::task::spawn_blocking(move || scaffold::create(&directory, id)).await??;
            output::list(presentation, &[created]).await
        }
    }
}
