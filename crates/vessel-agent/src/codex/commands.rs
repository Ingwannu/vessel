//! Codex skills/list and slash invocation, decoded once at the protocol edge.
//! No filesystem fallback reads another home or fabricates native commands.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use vessel_proto::composer_commands::CommandKind;
use vessel_proto::conversation::CommandName;
use vessel_proto::{AgentCommand, ContentBlock};

use super::mapper;
use super::protocol::UserInput;
use crate::error::InputError;

/// `SkillsListParams`, from the pinned app-server schema.
#[derive(Debug, Serialize)]
pub(super) struct SkillsListParams {
    pub(super) cwds: Vec<String>,
}

/// `SkillsListResponse`: entries are per requested directory.
#[derive(Debug, Deserialize)]
pub(super) struct SkillsListResponse {
    data: Vec<SkillsListEntry>,
}

#[derive(Debug, Deserialize)]
struct SkillsListEntry {
    skills: Vec<SkillMetadata>,
}

#[derive(Debug, Deserialize)]
struct SkillMetadata {
    name: String,
    description: String,
    path: String,
    enabled: bool,
}

/// A path is only echoed to the same provider; this module never opens it.
#[derive(Debug)]
struct Skill {
    command: AgentCommand,
    path: String,
}

/// A session's successful catalog keeps the path required for native invocation.
#[derive(Debug, Default)]
pub(super) struct Catalog {
    skills: Vec<Skill>,
}

impl Catalog {
    pub(super) fn reported(response: SkillsListResponse) -> Self {
        let mut seen = HashSet::new();
        let skills = response
            .data
            .into_iter()
            .flat_map(|entry| entry.skills)
            .filter(|skill| skill.enabled)
            .filter_map(|skill| {
                let name = skill.name.parse::<CommandName>().ok()?;
                if skill.path.trim().is_empty()
                    || skill.path.chars().any(char::is_control)
                    || !seen.insert(name.clone())
                {
                    return None;
                }
                let mut description = skill.description;
                description
                    .truncate(description.floor_char_boundary(AgentCommand::MAX_DESCRIPTION));
                Some(Skill {
                    command: AgentCommand {
                        name,
                        description,
                        hint: None,
                        kind: Some(CommandKind::Skill),
                        native_command: None,
                    },
                    path: skill.path,
                })
            })
            .take(AgentCommand::MAX_COMMANDS)
            .collect();
        Self { skills }
    }

    pub(super) fn commands(&self) -> Vec<AgentCommand> {
        self.skills
            .iter()
            .map(|skill| skill.command.clone())
            .collect()
    }

    pub(super) fn input(&self, content: &[ContentBlock]) -> Result<Vec<UserInput>, InputError> {
        if let [ContentBlock::Text { text }] = content
            && let Some((skill, args)) = self.invocation(text)
        {
            let name = skill.command.name.to_string();
            let text = if args.is_empty() {
                format!("${name}")
            } else {
                format!("${name} {args}")
            };
            return Ok(vec![
                UserInput::Skill {
                    name,
                    path: skill.path.clone(),
                },
                UserInput::Text { text },
            ]);
        }
        mapper::user_input(content)
    }

    fn invocation<'a>(&'a self, text: &'a str) -> Option<(&'a Skill, &'a str)> {
        let command = text.trim().strip_prefix('/')?;
        let (name, args) = command
            .split_once(char::is_whitespace)
            .map_or((command, ""), |(name, args)| (name, args.trim()));
        let skill = self
            .skills
            .iter()
            .find(|skill| skill.command.name.as_str() == name)?;
        Some((skill, args))
    }
}

#[cfg(test)]
mod tests;
