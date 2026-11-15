//! Presentation flags and selector aliases are decoded once before any effect.

use crate::{args::Target, error::CliError};
use clap::{Args, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub(crate) enum Format {
    #[default]
    Table,
    Json,
    Yaml,
    Cli,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Headers {
    Show,
    Hide,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Presentation {
    Table(Headers),
    Json,
    Yaml,
    Quiet,
}

/// Private booleans belong only to clap's external decoding edge.
#[derive(Debug, Default, Args)]
pub(crate) struct OutputOptions {
    #[arg(short = 'o', long, global = true, value_enum, default_value = "table")]
    format: Format,
    #[arg(long, global = true)]
    json: bool,
    #[arg(short = 'q', long, global = true)]
    quiet: bool,
    #[arg(long, global = true)]
    no_headers: bool,
    #[command(flatten)]
    color: ColorOptions,
}
/// Color selection is separate from data format; current tables are monochrome.
#[derive(Debug, Default, Args)]
struct ColorOptions {
    #[arg(long, global = true)]
    no_color: bool,
}
impl OutputOptions {
    pub(crate) const fn error_presentation(&self) -> Presentation {
        if self.json {
            return Presentation::Json;
        }
        match self.format {
            Format::Json => Presentation::Json,
            Format::Yaml => Presentation::Yaml,
            Format::Cli | Format::Table => Presentation::Table(Headers::Hide),
        }
    }
    pub(crate) const fn presentation(&self) -> Presentation {
        if self.quiet {
            return Presentation::Quiet;
        }
        if self.json {
            return Presentation::Json;
        }
        match self.format {
            Format::Json => Presentation::Json,
            Format::Yaml => Presentation::Yaml,
            Format::Cli | Format::Table => Presentation::Table(if self.no_headers {
                Headers::Hide
            } else {
                Headers::Show
            }),
        }
    }
}

/// Keep legacy --node/--data-dir together usable while --host/--home conflict.
#[derive(Debug, Default, Args)]
pub(crate) struct ConnectionOptions {
    #[arg(long, global = true, conflicts_with_all = ["home", "node"])]
    host: Option<String>,
    #[arg(long, global = true, conflicts_with_all = ["host", "data_dir"])]
    home: Option<PathBuf>,
}
impl ConnectionOptions {
    pub(crate) fn apply(self, target: &mut Target) -> Result<(), CliError> {
        if let Some(host) = self.host {
            target.node = Some(host);
        }
        if let Some(home) = self.home {
            target.data_dir = Some(home);
        }
        if target.node.as_deref().is_some_and(str::is_empty) {
            return Err(CliError::Usage("--host must name a node"));
        }
        Ok(())
    }
}
