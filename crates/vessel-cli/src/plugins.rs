//! `vessel plugin …` (ADR 0115): one call each to the node's plugin host,
//! printed as text, or as the node's answer with `--json` / `--format`.
//!
//! [Decision Log]
//! - Purpose: Paseo's `paseo plugin ls|logs|install|enable|disable|reload|
//!   remove|init` (`packages/cli/src/commands/plugin/index.ts:211-268`).
//!   Paseo's `update` reviews managed sources, which this node does not
//!   install yet (P4); it is added with them.
//! - A relative directory is made absolute here, from this shell's folder,
//!   as Paseo's CLI resolves it: the node has no caller's folder and
//!   refuses relative paths.
//! - `init` writes a client-only theme plugin and a README; a helper
//!   example lands with plugin RPC (ADR 0115, P3), when a helper has calls
//!   to answer. It writes files only and downloads nothing.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use vessel_proto::plugins::{
    DisablePlugin, EnablePlugin, GetPluginLogs, InstallPlugin, ListPlugins, LogStream, PluginId,
    PluginListItem, PluginLogs, PluginStatus, ReloadPlugin, RemovePlugin,
};

use crate::args::parity::Presentation;
use crate::args::parity::output::Headers;
use crate::args::{PluginCommand, Target};
use crate::connect::connect;
use crate::error::CliError;
use crate::io;
use crate::parity::output::{self, Row};

/// Run `vessel plugin …`.
pub(crate) async fn command(
    target: &Target,
    command: PluginCommand,
    presentation: Presentation,
) -> Result<(), CliError> {
    if let PluginCommand::Init { dir, id } = command {
        return init(dir, id).await;
    }
    let client = connect(target).await?.client;
    match command {
        PluginCommand::Ls { id } => {
            let list = client.call(ListPlugins {}).await?;
            let rows: Vec<PluginRow<'_>> = list
                .plugins
                .iter()
                .filter(|item| id.as_ref().is_none_or(|id| &item.id == id))
                .map(PluginRow)
                .collect();
            match (id, presentation) {
                (Some(id), _) if rows.is_empty() => Err(CliError::PluginInit(format!(
                    "plugin is not installed: {id}"
                ))),
                (Some(_), Presentation::Table(_)) => {
                    let text: String = rows.iter().map(|row| details(row.0)).collect();
                    Ok(io::out(text.as_bytes()).await?)
                }
                (_, Presentation::Table(Headers::Show)) if rows.is_empty() => {
                    Ok(io::out(b"no plugins installed\n").await?)
                }
                _ => output::list(presentation, &rows).await,
            }
        }
        PluginCommand::Install {
            source,
            id,
            reference,
        } => {
            let item = client
                .call(InstallPlugin {
                    source: absolute_source(&source)?,
                    id,
                    reference,
                })
                .await?;
            changed(presentation, &item, "installed").await
        }
        PluginCommand::Enable { id } => {
            let item = client.call(EnablePlugin { plugin: id }).await?;
            changed(presentation, &item, "enabled").await
        }
        PluginCommand::Disable { id } => {
            let item = client.call(DisablePlugin { plugin: id }).await?;
            changed(presentation, &item, "disabled").await
        }
        PluginCommand::Reload { id } => {
            let item = client.call(ReloadPlugin { plugin: id }).await?;
            changed(presentation, &item, "reloaded").await
        }
        PluginCommand::Remove { id } => {
            let _ack = client.call(RemovePlugin { plugin: id.clone() }).await?;
            match presentation {
                Presentation::Table(_) => Ok(io::out(format!("{id}: removed\n").as_bytes()).await?),
                Presentation::Json | Presentation::Yaml | Presentation::Quiet => {
                    output::single(
                        presentation,
                        id.as_str(),
                        &serde_json::json!({"removed": id}),
                    )
                    .await
                }
            }
        }
        PluginCommand::Logs { id } => {
            let logs = client.call(GetPluginLogs { plugin: id.clone() }).await?;
            match presentation {
                Presentation::Table(_) => Ok(io::out(log_text(&logs).as_bytes()).await?),
                Presentation::Json | Presentation::Yaml | Presentation::Quiet => {
                    output::single(presentation, id.as_str(), &logs).await
                }
            }
        }
        PluginCommand::Init { .. } => Ok(()),
    }
}

/// A lifecycle change's answer: `ID: STATUS` (and the reason when failed).
async fn changed(
    presentation: Presentation,
    item: &PluginListItem,
    done: &str,
) -> Result<(), CliError> {
    match presentation {
        Presentation::Table(_) => {
            let text = format!("{}: {done}, {}\n", item.id, status(item.status));
            Ok(io::out(text.as_bytes()).await?)
        }
        Presentation::Json | Presentation::Yaml | Presentation::Quiet => {
            output::single(presentation, item.id.as_str(), item).await
        }
    }
}

const fn status(status: PluginStatus) -> &'static str {
    match status {
        PluginStatus::Running => "running",
        PluginStatus::Disabled => "disabled",
        PluginStatus::Failed => "failed",
    }
}

/// One installation, every field the node gave.
fn details(item: &PluginListItem) -> String {
    let mut lines = vec![
        format!("id: {}", item.id),
        format!("status: {}", status(item.status)),
        format!("enabled: {}", if item.enabled { "yes" } else { "no" }),
    ];
    if let Some(name) = &item.name {
        lines.push(format!("name: {name}"));
    }
    if let Some(description) = &item.description {
        lines.push(format!("description: {description}"));
    }
    if let Some(path) = &item.path {
        lines.push(format!("directory: {path}"));
    }
    if let Some(error) = &item.error {
        lines.push(format!("error: {error}"));
    }
    let mut text = lines.join("\n");
    text.push('\n');
    text
}

/// Log lines, oldest first: time, where from, text.
fn log_text(logs: &PluginLogs) -> String {
    if logs.entries.is_empty() {
        return "(no log lines)\n".to_owned();
    }
    let mut text = String::new();
    for entry in &logs.entries {
        let stream = match entry.stream {
            LogStream::Host => "vessel",
            LogStream::Stderr => "stderr",
        };
        // Writing to a String cannot fail.
        let _written = writeln!(
            text,
            "{} {stream} {}",
            entry.at.format("%Y-%m-%dT%H:%M:%SZ"),
            entry.message.as_str()
        );
    }
    text
}

#[derive(serde::Serialize)]
#[serde(transparent)]
struct PluginRow<'a>(&'a PluginListItem);

impl Row for PluginRow<'_> {
    fn id(&self) -> String {
        self.0.id.to_string()
    }

    fn headers() -> &'static [&'static str] {
        &["ID", "STATUS", "ENABLED", "NAME"]
    }

    fn cells(&self) -> Vec<String> {
        vec![
            self.0.id.to_string(),
            status(self.0.status).to_owned(),
            if self.0.enabled { "yes" } else { "no" }.to_owned(),
            self.0
                .name
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
        ]
    }
}

/// `./x`, `../x` and `.` made absolute from this shell's folder; anything
/// else as typed (the node reads `~`, absolute paths and managed sources).
fn absolute_source(source: &str) -> Result<String, CliError> {
    let relative =
        source == "." || source == ".." || source.starts_with("./") || source.starts_with("../");
    if !relative {
        return Ok(source.to_owned());
    }
    let (path, subpath) = match source.rsplit_once(':') {
        Some((path, subpath)) if !subpath.contains("..") && !subpath.starts_with('/') => {
            (path, Some(subpath))
        }
        Some(_) | None => (source, None),
    };
    let absolute = std::path::absolute(path)
        .map_err(|error| CliError::PluginInit(format!("{path}: {error}")))?;
    let mut text = absolute.display().to_string();
    if let Some(subpath) = subpath {
        text.push(':');
        text.push_str(subpath);
    }
    Ok(text)
}

/// `vessel plugin init DIR`: a theme plugin to start from.
async fn init(dir: PathBuf, id: Option<PluginId>) -> Result<(), CliError> {
    let id = match id {
        Some(id) => id,
        None => default_id(&dir)?,
    };
    let written = tokio::task::spawn_blocking(move || scaffold(&dir, &id)).await??;
    let text = format!(
        "wrote {}\ninstall it with: vessel plugin install {}\n",
        written.display(),
        written.display()
    );
    Ok(io::out(text.as_bytes()).await?)
}

/// The directory's name as an id, when it is one.
fn default_id(dir: &Path) -> Result<PluginId, CliError> {
    dir.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.parse().ok())
        .ok_or_else(|| {
            CliError::PluginInit(
                "the directory's name is not a plugin id (a lowercase letter, then lowercase \
                 letters, digits or hyphens); give one with --id"
                    .to_owned(),
            )
        })
}

/// Write the files; refuse a directory that already has a manifest.
fn scaffold(dir: &Path, id: &PluginId) -> Result<PathBuf, CliError> {
    let manifest = dir.join("vessel-plugin.json");
    if manifest.exists() {
        return Err(CliError::PluginInit(format!(
            "{} already exists; nothing was written",
            manifest.display()
        )));
    }
    let failed =
        |error: std::io::Error| CliError::PluginInit(format!("{}: {error}", dir.display()));
    std::fs::create_dir_all(dir).map_err(failed)?;
    let files = [
        ("vessel-plugin.json", manifest_text(id)),
        ("client.json", client_text(id)),
        ("README.md", readme_text(id)),
    ];
    for (name, text) in files {
        std::fs::write(dir.join(name), text).map_err(failed)?;
    }
    std::path::absolute(dir).map_err(failed)
}

fn manifest_text(id: &PluginId) -> String {
    let manifest = serde_json::json!({
        "id": id,
        "name": format!("{id} theme"),
        "description": "A theme for Vessel.",
        "client": "client.json",
    });
    pretty(&manifest)
}

fn client_text(id: &PluginId) -> String {
    let client = serde_json::json!({
        "methods": [],
        "contributions": [],
        "themes": [{
            "id": id,
            "name": format!("{id} dark"),
            "appearance": "dark",
            "colors": {
                "background": "#15171c",
                "foreground": "#e6e8ee",
                "raised": "#1c1f26",
                "control": "#252932",
                "border": "#2f3440",
                "accent": "#6ea8fe",
                "mutedForeground": "#9aa3b2",
                "ring": "#6ea8fe",
            },
        }],
    });
    pretty(&client)
}

fn readme_text(id: &PluginId) -> String {
    format!(
        "# {id}\n\n\
         A Vessel plugin. `vessel-plugin.json` names it; `client.json` is the declarative data\n\
         the apps draw (here one theme with Paseo's eight colors).\n\n\
         Install it on a node with `vessel plugin install <this directory>`, then turn plugins\n\
         on with `vessel node settings --plugins on`.\n\n\
         A plugin may also run a helper program: `\"server\": {{\"command\": [\"./helper\"]}}` in the\n\
         manifest. It runs as the node's user, in this directory, with a minimal environment, and\n\
         speaks length-prefixed JSON frames on its standard input and output (`initialize`, then\n\
         `ready`). Plugins are trusted code: install only what you would run yourself.\n"
    )
}

fn pretty(value: &serde_json::Value) -> String {
    let mut text = serde_json::to_string_pretty(value).unwrap_or_default();
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::plugins::ClientDocument;

    #[test]
    fn init_writes_a_valid_theme_plugin_and_refuses_twice() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("night-sky");
        let id = default_id(&dir).unwrap();
        let _written = scaffold(&dir, &id).unwrap();
        let client = std::fs::read(dir.join("client.json")).unwrap();
        let _document: ClientDocument = serde_json::from_slice(&client).unwrap();
        let manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.join("vessel-plugin.json")).unwrap())
                .unwrap();
        assert_eq!(manifest["id"], "night-sky");
        assert_eq!(manifest["client"], "client.json");
        assert!(matches!(scaffold(&dir, &id), Err(CliError::PluginInit(_))));
        assert!(default_id(Path::new("/tmp/Not An Id")).is_err());
    }

    #[test]
    fn relative_directories_become_absolute_and_others_pass() {
        assert_eq!(absolute_source("~/p").unwrap(), "~/p");
        assert_eq!(absolute_source("/p").unwrap(), "/p");
        assert_eq!(absolute_source("npm:x").unwrap(), "npm:x");
        let here = std::env::current_dir().unwrap();
        assert_eq!(
            absolute_source("./p:plugins/x").unwrap(),
            format!("{}:plugins/x", here.join("p").display())
        );
    }
}
