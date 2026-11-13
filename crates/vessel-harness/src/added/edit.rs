//! [`add_agent`] and [`remove_agent`]: changing `agents.toml`'s text in
//! place (`toml_edit`), so a person's comments, order and other agents stay
//! as they were written. The caller parses the result ([`super::parse_agents`])
//! before writing it, so a change the file's rules refuse is never kept.

use vessel_proto::AgentName;
use vessel_proto::agents::{AgentDefinition, AgentSource};
use vessel_proto::{ConfigValue, EnvName, McpServerName};

/// Why `agents.toml`'s text could not be changed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EditError {
    /// The text is not TOML (it would not load either); never quoted.
    #[error("agents.toml is not valid TOML")]
    NotToml,
    /// `agents` is there but is not a table.
    #[error("`agents` in agents.toml is not a table")]
    NotATable,
    /// An agent of that name, ignoring case, is defined already.
    #[error("an agent named `{0}` is defined already")]
    Taken(String),
    /// No agent of that name.
    #[error("no agent named `{0}` is defined")]
    NotFound(AgentName),
}

/// `text` (the current `agents.toml`, possibly empty) with `name` defined
/// as `definition`, from `source`. With `replace`, an agent of that name
/// (ignoring case) is replaced; otherwise it is [`EditError::Taken`].
///
/// # Errors
/// See [`EditError`].
pub fn add_agent(
    text: &str,
    name: &AgentName,
    definition: &AgentDefinition,
    source: &AgentSource,
    replace: bool,
) -> Result<String, EditError> {
    let mut document: toml_edit::DocumentMut = text.parse().map_err(|_error| EditError::NotToml)?;
    let agents = agents_table(&mut document)?;
    let taken: Vec<String> = agents
        .iter()
        .map(|(defined, _item)| defined.to_owned())
        .filter(|defined| defined.eq_ignore_ascii_case(name.as_str()))
        .collect();
    if let Some(defined) = taken.first()
        && !replace
    {
        return Err(EditError::Taken(defined.clone()));
    }
    for defined in &taken {
        let _removed = agents.remove(defined);
    }
    let _none = agents.insert(
        name.as_str(),
        toml_edit::Item::Table(table(definition, source)),
    );
    Ok(document.to_string())
}

/// `text` without the agent `name`.
///
/// # Errors
/// See [`EditError`].
pub fn remove_agent(text: &str, name: &AgentName) -> Result<String, EditError> {
    let mut document: toml_edit::DocumentMut = text.parse().map_err(|_error| EditError::NotToml)?;
    let agents = agents_table(&mut document)?;
    let _removed = agents
        .remove(name.as_str())
        .ok_or_else(|| EditError::NotFound(name.clone()))?;
    Ok(document.to_string())
}

/// The document's `agents` table, made (implicit, so the file shows only
/// `[agents.<name>]`) when there is none.
fn agents_table(document: &mut toml_edit::DocumentMut) -> Result<&mut toml_edit::Table, EditError> {
    document
        .entry("agents")
        .or_insert_with(|| {
            let mut table = toml_edit::Table::new();
            table.set_implicit(true);
            toml_edit::Item::Table(table)
        })
        .as_table_mut()
        .ok_or(EditError::NotATable)
}

/// One agent's table, keys in the order the file's documentation shows.
fn table(definition: &AgentDefinition, source: &AgentSource) -> toml_edit::Table {
    let mut table = toml_edit::Table::new();
    if let Some(label) = &definition.label {
        let _none = table.insert("label", toml_edit::value(label.as_str()));
    }
    let _none = table.insert("program", toml_edit::value(definition.program.as_str()));
    if !definition.args.is_empty() {
        let args: toml_edit::Array = definition.args.iter().map(String::as_str).collect();
        let _none = table.insert("args", toml_edit::value(args));
    }
    if !definition.env.is_empty() {
        let mut env = toml_edit::InlineTable::new();
        for variable in &definition.env {
            let value: toml_edit::Value = match &variable.value {
                ConfigValue::Text(text) => text.as_str().into(),
                ConfigValue::Secret(secret) => {
                    let mut reference = toml_edit::InlineTable::new();
                    let _none = reference.insert("secret", secret.as_str().into());
                    reference.into()
                }
            };
            let _none = env.insert(variable.name.as_str(), value);
        }
        let _none = table.insert("env", toml_edit::value(env));
    }
    if !definition.inherit.is_empty() {
        let inherit: toml_edit::Array = definition.inherit.iter().map(EnvName::as_str).collect();
        let _none = table.insert("inherit", toml_edit::value(inherit));
    }
    if !definition.mcp.is_empty() {
        let mcp: toml_edit::Array = definition.mcp.iter().map(McpServerName::as_str).collect();
        let _none = table.insert("mcp", toml_edit::value(mcp));
    }
    match source {
        AgentSource::Command => {}
        AgentSource::Registry { id, version } => {
            let mut from = toml_edit::InlineTable::new();
            let _none = from.insert("registry", id.as_str().into());
            let _none = from.insert("version", version.as_str().into());
            let _none = table.insert("source", toml_edit::value(from));
        }
    }
    table
}
