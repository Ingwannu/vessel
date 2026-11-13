//! Report shipped catalogs or a candidate/fixture root with the gate's parser.
#[path = "../build_support/mod.rs"]
mod build_support;

use std::io::Write as _;
use std::path::{Path, PathBuf};
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
    #[error("usage: locale_coverage [--root CATALOG_ROOT]")]
    Usage,
    #[error("{0}: native name absent after validation")]
    NativeName(String),
}

#[derive(Debug)]
enum CatalogRoot {
    Shipped,
    Preview(PathBuf),
}

fn requested_root() -> Result<CatalogRoot, CoverageError> {
    let mut arguments = std::env::args_os().skip(1);
    match (arguments.next(), arguments.next(), arguments.next()) {
        (None, None, None) => Ok(CatalogRoot::Shipped),
        (Some(flag), Some(path), None) if flag == "--root" => {
            Ok(CatalogRoot::Preview(PathBuf::from(path)))
        }
        _ => Err(CoverageError::Usage),
    }
}

fn main() -> Result<(), CoverageError> {
    let shipped = Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    let requested = requested_root()?;
    let root = match &requested {
        CatalogRoot::Shipped => shipped.as_path(),
        CatalogRoot::Preview(path) => path,
    };
    let languages = build_support::read(root)?;
    let coverage = build_support::validate(&languages)?;
    let _generated = build_support::generate(&languages, &coverage)?;
    // Small grammar fixtures need no numeric formatter settings. A candidate
    // copied from the current source has them and gets the same extra checks.
    if languages
        .iter()
        .any(|l| l.tag == "en" && l.files.contains_key("formats"))
    {
        let _rules = build_support::generate_format_rules(&languages)?;
    }
    let mut output = std::io::stdout().lock();
    match requested {
        CatalogRoot::Shipped => {
            writeln!(output, "Shipped languages:")?;
            // Inspect the compiled registry too, not only an independent scan.
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
        }
        CatalogRoot::Preview(_) => {
            writeln!(output, "Catalog preview (not compiled or shipped):")?;
            for language in &languages {
                let name = language
                    .messages
                    .get("meta-name")
                    .and_then(|m| m.plain.as_deref())
                    .ok_or_else(|| CoverageError::NativeName(language.tag.clone()))?;
                let complete = coverage
                    .iter()
                    .find(|c| c.tag == language.tag)
                    .is_some_and(|c| c.missing.is_empty());
                writeln!(output, "{} ({name}) complete={complete}", language.tag)?;
            }
        }
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
