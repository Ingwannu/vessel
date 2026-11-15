//! Presentation flags are decoded once; connection selection is owned separately.

use clap::{Args, ValueEnum};

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
