//! Discover folders deterministically. Parse once at build time with Fluent,
//! preserving comments for generated documentation and translator context.

use std::collections::BTreeMap;
use std::path::Path;

use fluent_syntax::{
    ast::{Entry, Expression, InlineExpression, PatternElement},
    parser,
};

use super::{BuildError, Language, Message, Shape, pattern, source};

pub(crate) fn read(root: &Path) -> Result<Vec<Language>, BuildError> {
    let mut languages = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let tag = entry.file_name().to_string_lossy().into_owned();
        // The folder is the tag, not an instruction to search elsewhere.
        let parsed = tag
            .parse::<unic_langid::LanguageIdentifier>()
            .map_err(|e| source(&tag, e.to_string()))?;
        if parsed.to_string() != tag || matches!(tag.as_str(), "en-XA" | "system") {
            return Err(source(
                &tag,
                "use a canonical language tag; en-XA and system are reserved",
            ));
        }
        let mut files = BTreeMap::new();
        for file in std::fs::read_dir(entry.path())? {
            let file = file?;
            if file.path().extension().is_some_and(|ext| ext == "ftl") {
                if !file.file_type()?.is_file() {
                    return Err(source(
                        &tag,
                        "translation resources must be regular files, not links or directories",
                    ));
                }
                let area = file
                    .path()
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .ok_or_else(|| source(&tag, "missing area name"))?;
                check_identifier(&area, &tag)?;
                let _old = files.insert(area, std::fs::read_to_string(file.path())?);
            }
        }
        languages.push(parse(tag, files)?);
    }
    languages.sort_by(|a, b| a.tag.cmp(&b.tag));
    Ok(languages)
}

pub(crate) fn parse(tag: String, files: BTreeMap<String, String>) -> Result<Language, BuildError> {
    let mut messages = BTreeMap::new();
    let mut terms = BTreeMap::new();
    for (area, contents) in &files {
        let file = format!("{tag}/{area}.ftl");
        let resource = parser::parse(contents.as_str())
            .map_err(|(_, e)| source(&file, format!("invalid Fluent: {e:?}")))?;
        for entry in resource.body {
            match entry {
                Entry::Message(message) => {
                    let id = message.id.name;
                    let prefix = format!("{area}-");
                    let local = id
                        .strip_prefix(&prefix)
                        .ok_or_else(|| source(&file, format!("{id} must start with {prefix}")))?;
                    check_identifier(local, &file)?;
                    if !message.attributes.is_empty() {
                        return Err(source(&file, "use a key per context instead of attributes"));
                    }
                    let value = message
                        .value
                        .ok_or_else(|| source(&file, format!("{id} has no value")))?;
                    let mut shape = Shape::default();
                    pattern::scan(&value, &mut shape, &file)?;
                    let plain = value
                        .elements
                        .iter()
                        .map(|e| match e {
                            PatternElement::TextElement { value } => Some((*value).to_owned()),
                            PatternElement::Placeable {
                                expression:
                                    Expression::Inline(InlineExpression::StringLiteral { value }),
                            } => Some(
                                fluent_syntax::unicode::unescape_unicode_to_string(value)
                                    .into_owned(),
                            ),
                            PatternElement::Placeable { .. } => None,
                        })
                        .collect::<Option<String>>();
                    let context = message
                        .comment
                        .map(|c| c.content.join("\n"))
                        .unwrap_or_default();
                    if messages
                        .insert(
                            id.to_owned(),
                            Message {
                                area: area.clone(),
                                context,
                                shape,
                                plain,
                            },
                        )
                        .is_some()
                    {
                        return Err(source(&file, format!("duplicate message {id}")));
                    }
                }
                Entry::Term(term) => {
                    if !term.attributes.is_empty() {
                        return Err(source(&file, "term attributes are not supported"));
                    }
                    let mut shape = Shape::default();
                    pattern::scan(&term.value, &mut shape, &file)?;
                    if terms.insert(format!("-{}", term.id.name), shape).is_some() {
                        return Err(source(&file, "duplicate term"));
                    }
                }
                Entry::Comment(_) | Entry::GroupComment(_) | Entry::ResourceComment(_) => {}
                Entry::Junk { .. } => {
                    return Err(source(&file, "Fluent junk is not a translation"));
                }
            }
        }
    }
    Ok(Language {
        tag,
        files,
        messages,
        terms,
    })
}

pub(super) fn check_identifier(name: &str, file: &str) -> Result<(), BuildError> {
    if name.is_empty()
        || !name.starts_with(|c: char| c.is_ascii_lowercase())
        || !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        || matches!(name, "self" | "super" | "crate")
    {
        return Err(source(
            file,
            format!("{name}: use lowercase words separated by hyphens"),
        ));
    }
    Ok(())
}
