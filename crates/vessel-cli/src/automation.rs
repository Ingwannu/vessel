//! `vessel schedule …` and `vessel prompt …` (protocol 11, ADR 0033).
//! Each is a call or two through `vessel-client`; the node checks every
//! rule (who may, the bounds, the time), so what is here is reading the
//! arguments, finding what a selector names, and printing.

mod prompts;
mod schedules;

use vessel_client::NodeClient;
use vessel_proto::automation::{SavedPrompt, ScheduleInfo};

use crate::error::CliError;
use crate::io;

pub(crate) use prompts::prompt;
pub(crate) use schedules::schedule;

/// The one item `given` names among `items`: by its id, a unique prefix
/// of one, or its name.
fn one<T>(
    items: Vec<T>,
    given: &str,
    what: &str,
    id: impl Fn(&T) -> String,
    name: impl Fn(&T) -> Option<String>,
) -> Result<T, CliError> {
    let named: Vec<usize> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| name(item).as_deref() == Some(given))
        .map(|(index, _)| index)
        .collect();
    let matching: Vec<usize> = if named.is_empty() {
        items
            .iter()
            .enumerate()
            .filter(|(_, item)| id(item).starts_with(given))
            .map(|(index, _)| index)
            .collect()
    } else {
        named
    };
    match matching.as_slice() {
        [index] => items
            .into_iter()
            .nth(*index)
            .ok_or_else(|| CliError::Automation(format!("no {what} matches `{given}`"))),
        [] => Err(CliError::Automation(format!("no {what} matches `{given}`"))),
        more => Err(CliError::Automation(format!(
            "`{given}` matches {} {what}s; type more of the id",
            more.len()
        ))),
    }
}

async fn find_schedule(client: &NodeClient, given: &str) -> Result<ScheduleInfo, CliError> {
    one(
        client.schedules().await?,
        given,
        "schedule",
        |schedule| schedule.id.to_string(),
        |schedule| {
            schedule
                .definition
                .name
                .as_ref()
                .map(|name| name.as_str().to_owned())
        },
    )
}

async fn find_prompt(client: &NodeClient, given: &str) -> Result<SavedPrompt, CliError> {
    one(
        client.saved_prompts(None).await?,
        given,
        "saved prompt",
        |prompt| prompt.id.to_string(),
        |prompt| Some(prompt.name.as_str().to_owned()),
    )
}

fn parse<T: std::str::FromStr>(text: &str, what: &str) -> Result<T, CliError>
where
    T::Err: std::fmt::Display,
{
    text.parse()
        .map_err(|error| CliError::Automation(format!("{what}: {error}")))
}

/// A prompt's text: given, or read from a file on this machine.
async fn text_of(
    given: Option<String>,
    file: Option<&std::path::Path>,
) -> Result<Option<String>, CliError> {
    match (given, file) {
        (Some(text), _) => Ok(Some(text)),
        (None, Some(path)) => tokio::fs::read_to_string(path)
            .await
            .map(Some)
            .map_err(|error| CliError::Automation(format!("{}: {error}", path.display()))),
        (None, None) => Ok(None),
    }
}

async fn out(text: &str) -> Result<(), CliError> {
    Ok(io::out(text.as_bytes()).await?)
}

/// The first line of `text`, at most `chars` characters.
fn first_line(text: &str, chars: usize) -> String {
    let line = text.lines().next().unwrap_or_default();
    let mut cut: String = line.chars().take(chars).collect();
    if line.chars().nth(chars).is_some() || text.lines().nth(1).is_some() {
        cut.push('…');
    }
    cut
}

fn short(id: &impl ToString) -> String {
    id.to_string().chars().take(8).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_wins_over_a_prefix_and_two_matches_are_refused() {
        let items = vec![
            ("abc123", "nightly"),
            ("abd456", "abc"),
            ("ffe789", "weekly"),
        ];
        let id = |item: &(&str, &str)| item.0.to_owned();
        let name = |item: &(&str, &str)| Some(item.1.to_owned());
        assert_eq!(
            one(items.clone(), "abc", "x", id, name).unwrap().0,
            "abd456"
        );
        assert_eq!(one(items.clone(), "ff", "x", id, name).unwrap().1, "weekly");
        assert!(one(items.clone(), "ab", "x", id, name).is_err());
        assert!(one(items, "zz", "x", id, name).is_err());
    }

    #[test]
    fn a_first_line_says_when_there_is_more() {
        assert_eq!(first_line("one line", 20), "one line");
        assert_eq!(first_line("first\nsecond", 20), "first…");
        assert_eq!(first_line("abcdef", 3), "abc…");
    }
}
