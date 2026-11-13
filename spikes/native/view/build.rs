//! Embed translations and generate the typed English API (ADR 0039).
mod build_support;

use std::io::Write as _;
use std::path::PathBuf;

fn main() -> Result<(), build_support::BuildError> {
    let mut output = std::io::stdout().lock();
    writeln!(output, "cargo:rerun-if-changed=locales")?;
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?).join("locales");
    let generated = PathBuf::from(std::env::var("OUT_DIR")?).join("translations.rs");
    let languages = build_support::read(&root)?;
    let coverage = build_support::validate(&languages)?;
    let api = build_support::generate(&languages, &coverage)?;
    let rules = build_support::generate_format_rules(&languages)?;
    std::fs::write(generated, format!("{api}\n{rules}"))?;
    Ok(())
}
