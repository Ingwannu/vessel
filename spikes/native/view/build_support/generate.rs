//! English defines a closed API; generated plain getters only borrow a field.
//! Rust identifiers are derived from checked Fluent identifiers, never source
//! text pasted into code. Literal values use Rust's escaping via Debug.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use super::{BuildError, Coverage, Kind, Language, source, validate::expanded};

mod plain;

pub(crate) fn generate(
    languages: &[Language],
    coverage: &[Coverage],
) -> Result<String, BuildError> {
    let english = languages
        .iter()
        .find(|l| l.tag == "en")
        .ok_or_else(|| source("en", "missing English"))?;
    let mut output = String::new();
    registry(&mut output, languages, coverage)?;
    api(&mut output, english)?;
    Ok(output)
}

type Contracts = BTreeMap<String, super::Contract>;

// The same production emitter is compiled in isolation by the compiler-contract
// regression; registry discovery is tested independently by the catalog gate.
pub(super) fn api(output: &mut String, english: &Language) -> Result<(), BuildError> {
    let contracts: BTreeMap<_, _> = english
        .messages
        .keys()
        .map(|key| Ok((key.clone(), expanded(english, key)?)))
        .collect::<Result<_, BuildError>>()?;
    let paths = plain::generate(output, english, &contracts)?;
    for area_name in english.files.keys() {
        area(output, english, area_name, &contracts, &paths)?;
    }
    Ok(())
}

fn area(
    output: &mut String,
    english: &Language,
    area: &str,
    contracts: &Contracts,
    paths: &plain::Paths,
) -> Result<(), BuildError> {
    let area_type = pascal(area);
    writeln!(
        output,
        "/// The messages of {area}, generated from English.\n#[derive(Debug, Clone, Copy)]\npub struct {area_type}<'a> {{ translator: &'a Translator }}"
    )?;
    writeln!(
        output,
        "impl Translator {{\n/// Borrow the messages of {area}.\n#[must_use]\npub const fn r#{}(&self) -> {area_type}<'_> {{ {area_type} {{ translator: self }} }}\n}}",
        snake(area)
    )?;
    let prefix = format!("{area}-");
    for (key, shape) in contracts {
        if english
            .messages
            .get(key)
            .is_none_or(|message| message.area != area)
        {
            continue;
        }
        for (name, kind) in shape.variables() {
            if let Kind::Choice(choices) = kind {
                let enum_name = format!("{}{}", pascal(key), pascal(name));
                writeln!(
                    output,
                    "/// The choices English passes as ${name} to {key}.\n#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum {enum_name} {{"
                )?;
                for choice in choices {
                    writeln!(output, "/// The {choice} choice.\n{},", pascal(choice))?;
                }
                writeln!(output, "}}")?;
            }
        }
    }
    for (key, shape) in contracts {
        if english.messages.get(key).is_some_and(|m| m.area == area) && shape.variables().len() > 5
        {
            let lifetime = if shape
                .variables()
                .values()
                .any(|k| matches!(k, Kind::Text | Kind::Date))
            {
                "<'a>"
            } else {
                ""
            };
            writeln!(
                output,
                "/// Typed parts of {key}; all fields are required.\n#[derive(Debug, Clone)]\npub struct {}Args{lifetime} {{",
                pascal(key)
            )?;
            for (name, kind) in shape.variables() {
                let ty = argument_type(key, name, kind, "&'a str");
                writeln!(
                    output,
                    "/// The ${name} part.\npub r#{}: {ty},",
                    snake(name)
                )?;
            }
            writeln!(output, "}}")?;
        }
    }
    let has_plain = english.messages.iter().any(|(key, message)| {
        message.area == area
            && contracts
                .get(key)
                .is_some_and(|shape| shape.variables().is_empty())
    });
    if has_plain {
        writeln!(output, "impl<'a> {area_type}<'a> {{")?;
    } else {
        writeln!(output, "impl {area_type}<'_> {{")?;
    }
    for (key, message) in &english.messages {
        if message.area != area {
            continue;
        }
        let Some(method) = key.strip_prefix(&prefix) else {
            continue;
        };
        emit_message(output, key, method, message, contracts, paths)?;
    }
    writeln!(output, "}}")?;
    Ok(())
}

fn emit_message(
    output: &mut String,
    key: &str,
    method: &str,
    message: &super::Message,
    contracts: &Contracts,
    paths: &plain::Paths,
) -> Result<(), BuildError> {
    let shape = contracts
        .get(key)
        .ok_or_else(|| source(key, "missing generated shape"))?;
    for line in message.context.lines() {
        writeln!(output, "/// {line}")?;
    }
    if shape.variables().is_empty() {
        writeln!(
            output,
            "#[must_use]\npub fn r#{}(self) -> &'a str {{ &self.translator.plain{} }}",
            snake(method),
            paths.projection(&message.area, key)?
        )?;
    } else {
        writeln!(
            output,
            "///\n/// # Errors\n/// Returns a formatting failure if both the translation and English fail.\npub fn r#{}(self,",
            snake(method)
        )?;
        arguments(output, key, shape.variables())?;
        writeln!(output, "self.translator.format({key:?}, &arguments)\n}}")?;
    }
    Ok(())
}

fn arguments(
    output: &mut String,
    key: &str,
    variables: &BTreeMap<super::contract::Name, Kind>,
) -> Result<(), BuildError> {
    let grouped = variables.len() > 5;
    if grouped {
        let lifetime = if variables
            .values()
            .any(|k| matches!(k, Kind::Text | Kind::Date))
        {
            "<'_>"
        } else {
            ""
        };
        writeln!(output, "parts: &{}Args{lifetime},", pascal(key))?;
    } else {
        for (name, kind) in variables {
            let ty = argument_type(key, name, kind, "&str");
            writeln!(output, "r#{}: {ty},", snake(name))?;
        }
    }
    writeln!(
        output,
        ") -> Result<String, FormatError> {{\nlet mut arguments = FluentArgs::new();"
    )?;
    for (name, kind) in variables {
        let argument = format!("{}r#{}", if grouped { "parts." } else { "" }, snake(name));
        let name = name.as_str();
        match kind {
            Kind::Text => writeln!(output, "arguments.set({name:?}, {argument});")?,
            Kind::Number => {
                let owned = if grouped {
                    format!("{argument}.clone()")
                } else {
                    argument
                };
                writeln!(output, "arguments.set({name:?}, {owned}.fluent());")?;
            }
            Kind::Date => {
                writeln!(output, "arguments.set({name:?}, {argument}.fluent()?);")?;
            }
            Kind::Choice(choices) => {
                let enum_name = format!("{}{}", pascal(key), pascal(name));
                writeln!(output, "arguments.set({name:?}, match {argument} {{")?;
                for choice in choices {
                    let choice = choice.as_str();
                    writeln!(output, "{enum_name}::{} => {choice:?},", pascal(choice))?;
                }
                writeln!(output, "}});")?;
            }
        }
    }
    Ok(())
}

fn argument_type(key: &str, name: &str, kind: &Kind, text: &str) -> String {
    match kind {
        Kind::Text => text.to_owned(),
        Kind::Number => "Number".to_owned(),
        Kind::Date => text.replace("str", "Date"),
        Kind::Choice(_) => format!("{}{}", pascal(key), pascal(name)),
    }
}

fn registry(
    output: &mut String,
    languages: &[Language],
    coverage: &[Coverage],
) -> Result<(), BuildError> {
    let count = languages.len().saturating_add(1);
    writeln!(
        output,
        "static CACHES: [std::sync::OnceLock<Result<Translator, InitError>>; {count}] = [const {{ std::sync::OnceLock::new() }}; {count}];\nstatic EMBEDDED: &[Embedded] = &["
    )?;
    for language in languages {
        writeln!(output, "Embedded {{ sources: &[")?;
        for area in language.files.keys() {
            writeln!(
                output,
                "include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/locales/{}/{area}.ftl\")),",
                language.tag
            )?;
        }
        writeln!(output, "] }},")?;
    }
    writeln!(
        output,
        "];\n/// Languages in the chooser, by their own names; the pseudo-language is excluded.\npub const LANGUAGES: &[Locale] = &["
    )?;
    for (index, language) in languages.iter().enumerate() {
        let name = language
            .messages
            .get("meta-name")
            .and_then(|m| m.plain.as_deref())
            .ok_or_else(|| source(&language.tag, "missing native name"))?;
        let complete = coverage
            .iter()
            .find(|c| c.tag == language.tag)
            .is_some_and(|c| c.missing.is_empty());
        writeln!(
            output,
            "Locale {{ index: {index}, tag: {:?}, native_name: {name:?}, complete: {complete} }},",
            language.tag
        )?;
    }
    writeln!(
        output,
        "];\n/// Accented, expanded English for checking layouts; override only.\npub const PSEUDO: Locale = Locale {{ index: {}, tag: \"en-XA\", native_name: \"Accented English\", complete: true }};",
        languages.len()
    )?;
    let index = languages
        .iter()
        .position(|l| l.tag == "en")
        .ok_or_else(|| source("en", "missing English"))?;
    let native_name = languages
        .get(index)
        .and_then(|language| language.messages.get("meta-name"))
        .and_then(|message| message.plain.as_deref())
        .ok_or_else(|| source("en", "missing native name"))?;
    writeln!(
        output,
        "/// English, the final per-message fallback.\npub const ENGLISH: Locale = Locale {{ index: {index}, tag: \"en\", native_name: {native_name:?}, complete: true }};"
    )?;
    Ok(())
}

fn snake(value: &str) -> String {
    value.replace('-', "_")
}
fn pascal(value: &str) -> String {
    value
        .split('-')
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect()
}

#[cfg(test)]
mod tests;
