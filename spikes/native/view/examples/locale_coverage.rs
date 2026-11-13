//! Print missing messages with the same parser and validator as cargo test.
#[path = "../build_support/mod.rs"]
mod build_support;

use std::io::Write as _;
use std::path::Path;

use thiserror::Error;
use vessel_app_view::localization::{InitError, LANGUAGES, Translator};

#[derive(Debug, Error)]
enum CoverageError {
    #[error(transparent)]
    Source(#[from] build_support::BuildError),
    #[error(transparent)]
    Runtime(#[from] InitError),
    #[error(transparent)]
    Output(#[from] std::io::Error),
}

fn main() -> Result<(), CoverageError> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    let languages = build_support::read(&root)?;
    let coverage = build_support::validate(&languages)?;
    let _generated = build_support::generate(&languages, &coverage)?;
    let _rules = build_support::generate_format_rules(&languages)?;
    let mut output = std::io::stdout().lock();
    // Read the compiled registry too: adding a folder must affect what a
    // chooser sees, not only a tool's independently discovered file list.
    for locale in LANGUAGES {
        let translated = Translator::get(*locale)?;
        writeln!(
            output,
            "{} ({}) complete={}: {} / {}",
            locale.tag(),
            locale.native_name(),
            locale.is_complete(),
            translated.meta().language(),
            translated.meta().system()
        )?;
    }
    for language in coverage {
        writeln!(
            output,
            "{}: {} missing",
            language.tag,
            language.missing.len()
        )?;
        for key in language.missing {
            writeln!(output, "  {key}")?;
        }
    }
    Ok(())
}
