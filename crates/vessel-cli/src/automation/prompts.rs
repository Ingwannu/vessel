//! `vessel prompt …`: saved prompts listed, saved under a name (made, or
//! the one of that name changed), and sent.

use vessel_proto::automation::{CreatePrompt, SavedPrompt, UpdatePrompt};
use vessel_proto::workspace::SessionPlace;
use vessel_proto::{ContentBlock, Delivery, ProjectId};

use super::{find_prompt, first_line, out, parse, short, text_of};
use crate::args::{PromptCommand, Target};
use crate::connect::connect;
use crate::error::CliError;
use crate::selector::SessionSelector;
use crate::workspace;

pub(crate) async fn prompt(target: &Target, command: PromptCommand) -> Result<(), CliError> {
    let client = connect(target).await?.client;
    match command {
        PromptCommand::List { project } => {
            let project = match project {
                Some(given) => Some(workspace::project_id(&client, &given).await?),
                None => None,
            };
            let text: String = client
                .saved_prompts(project)
                .await?
                .iter()
                .map(line)
                .collect();
            out(&text).await
        }
        PromptCommand::Show { prompt } => {
            let saved = find_prompt(&client, &prompt).await?;
            let mut text = saved.text.as_str().to_owned();
            if !text.ends_with('\n') {
                text.push('\n');
            }
            out(&text).await
        }
        PromptCommand::Save {
            name,
            text,
            file,
            project,
        } => {
            let project = match project {
                Some(given) => Some(workspace::project_id(&client, &given).await?),
                None => None,
            };
            let text = text_of(text, file.as_deref())
                .await?
                .ok_or(CliError::Usage("give `--text` or `--file`"))?;
            let name = parse(&name, "name")?;
            let text = parse(&text, "text")?;
            let same = client
                .saved_prompts(project)
                .await?
                .into_iter()
                .find(|saved| saved.name == name && saved.project == project);
            let saved = match same {
                Some(saved) => {
                    client
                        .update_prompt(UpdatePrompt {
                            prompt: saved.id,
                            revision: saved.revision,
                            name,
                            text,
                            session: saved.session,
                        })
                        .await?
                }
                None => {
                    client
                        .create_prompt(CreatePrompt {
                            name,
                            text,
                            project,
                            session: None,
                        })
                        .await?
                }
            };
            out(&line(&saved)).await
        }
        PromptCommand::Delete { prompt } => {
            let saved = find_prompt(&client, &prompt).await?;
            client.delete_prompt(saved.id).await?;
            out(&format!("deleted {}\n", saved.id)).await
        }
        PromptCommand::Run {
            prompt,
            session,
            new,
        } => {
            let saved = find_prompt(&client, &prompt).await?;
            let content = vec![ContentBlock::Text {
                text: saved.text.as_str().to_owned(),
            }];
            let session = match session {
                Some(given) => {
                    let selector: SessionSelector = parse(&given, "--session")?;
                    selector.resolve(&client).await?.id
                }
                None if new => {
                    let spec = started_from(&saved)?;
                    client.create_session(spec).await?.id
                }
                None => return Err(CliError::Usage("give `--session` or `--new`")),
            };
            let delivered = client.prompt(session, content, Delivery::Queue).await?;
            out(&format!("{session}  {delivered:?}\n")).await
        }
    }
}

/// What a session started from `saved` runs with: its `session`, in its
/// project when that names no place.
fn started_from(saved: &SavedPrompt) -> Result<vessel_proto::SessionSpec, CliError> {
    let mut spec = saved.session.as_deref().cloned().ok_or(CliError::Usage(
        "this saved prompt says nothing about a session to start; give `--session`",
    ))?;
    if spec.place.is_none()
        && spec.cwd.is_none()
        && let Some(project) = saved.project
    {
        spec = spec.with_place(SessionPlace::Project { project, dir: None });
    }
    Ok(spec)
}

/// One saved prompt on a line: id, name, where, the first of its text.
fn line(saved: &SavedPrompt) -> String {
    let scope = saved.project.as_ref().map_or_else(
        || "machine".to_owned(),
        |project: &ProjectId| format!("project {}", short(project)),
    );
    format!(
        "{}  {}  {scope}  {}\n",
        short(&saved.id),
        saved.name.as_str(),
        first_line(saved.text.as_str(), 60)
    )
}
