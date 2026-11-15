//! Provider identity is not a HarnessRef; W22 owns its canonical node decoding.
use crate::error::CliError;
use clap::Subcommand;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub(crate) struct ProviderSelector(String);
impl FromStr for ProviderSelector {
    type Err = CliError;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        const MAX_PROVIDER_UNITS: usize = 1024;
        if text.trim().is_empty()
            || text.encode_utf16().count() > MAX_PROVIDER_UNITS
            || text.chars().any(char::is_control)
        {
            return Err(CliError::Usage(
                "provider selector must be bounded nonempty text without controls",
            ));
        }
        Ok(Self(text.to_owned()))
    }
}
impl ProviderSelector {
    pub(crate) fn text(&self) -> &str {
        &self.0
    }
}
#[derive(Debug, Subcommand)]
pub(crate) enum ProviderCommand {
    Ls,
    Models {
        provider: ProviderSelector,
        #[arg(long)]
        thinking: bool,
    },
    Diagnostic {
        provider: ProviderSelector,
    },
}
impl ProviderCommand {
    /// Keep a useful distinction between snapshots, model thinking and diagnostics.
    pub(crate) fn pending(self) -> CliError {
        match self {
            Self::Ls => CliError::ParityBlocked(
                "provider ls awaits W22's reviewed snapshot contract; an installed manifest is not an available driver",
            ),
            Self::Models { provider, thinking } => {
                let _selected = provider.text();
                if thinking {
                    CliError::ParityBlocked(
                        "per-model thinking choices await W22's reviewed provider contract",
                    )
                } else {
                    CliError::ParityBlocked(
                        "provider model identity awaits W22's reviewed contract; harness.options alone is not the full provider model projection",
                    )
                }
            }
            Self::Diagnostic { provider } => {
                let _selected = provider.text();
                CliError::ParityBlocked(
                    "provider diagnostic awaits W22's reviewed contract; agent readiness alone is not a provider diagnostic",
                )
            }
        }
    }
}
