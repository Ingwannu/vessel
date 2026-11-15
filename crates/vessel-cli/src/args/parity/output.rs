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
    #[arg(short = 'o', long, global = true, value_enum, value_parser = parse_format)]
    format: Option<Format>,
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
fn parse_format(raw: &str) -> Result<Format, String> {
    Format::from_str(raw.trim(), true)
}
impl OutputOptions {
    /// An explicit request must be consumed or refused before any command effect.
    pub(crate) fn requested(&self) -> Option<Presentation> {
        (self.format.is_some() || self.json || self.quiet || self.no_headers)
            .then(|| self.presentation())
    }

    pub(crate) fn presentation(&self) -> Presentation {
        if self.quiet {
            return Presentation::Quiet;
        }
        if self.json {
            return Presentation::Json;
        }
        match self.format.unwrap_or_default() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    #[derive(Parser)]
    struct Flags {
        #[command(flatten)]
        output: OutputOptions,
    }
    #[test]
    fn shared_output_precedence_and_normalization() {
        let read = |flags: &[&str]| Flags::try_parse_from(flags).unwrap().output.presentation();
        assert_eq!(read(&["vessel"]), Presentation::Table(Headers::Show));
        assert_eq!(
            read(&["vessel", "--no-headers"]),
            Presentation::Table(Headers::Hide)
        );
        assert_eq!(
            read(&["vessel", "-o", "cli"]),
            Presentation::Table(Headers::Show)
        );
        assert_eq!(
            read(&["vessel", "-o", "yaml", "--json"]),
            Presentation::Json
        );
        assert_eq!(
            read(&["vessel", "-o", "yaml", "--json", "--quiet"]),
            Presentation::Quiet
        );
        assert_eq!(read(&["vessel", "-o", " JSON "]), Presentation::Json);
        assert!(Flags::try_parse_from(["vessel", "-o", "invalid"]).is_err());
        assert!(
            Flags::try_parse_from(["vessel"])
                .unwrap()
                .output
                .requested()
                .is_none()
        );
        assert!(
            Flags::try_parse_from(["vessel", "-o", "table"])
                .unwrap()
                .output
                .requested()
                .is_some()
        );
    }
}
