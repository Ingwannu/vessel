//! `vessel pr …` (protocol 11, ADR 0029): a session's pull request on its
//! forge, opened, merged and watched by the node. What is printed is the
//! node's answer; nothing here talks to a forge.

use std::fmt::Write as _;
use std::path::Path;

use vessel_proto::forge::{
    ForgeAccess, ForgeStatus, GetForgeStatus, GetPullRequest, ListPullRequests, MergeMethod,
    MergeOutcome, MergePullRequest, MergeRefusal, OpenOutcome, OpenPullRequest, OpenRefusal,
    PullBody, PullNumber, PullRequest, PullSummary, SessionPullRequest, SetFeedback,
};
use vessel_proto::workspace::GitRefusal;

use crate::args::{Method, PrCommand, Switch, Target};
use crate::connect::connect;
use crate::error::CliError;
use crate::io;

pub(crate) async fn pr(target: &Target, command: PrCommand) -> Result<(), CliError> {
    let client = connect(target).await?.client;
    let text = match command {
        PrCommand::List => {
            let list = client.call(ListPullRequests {}).await?;
            let mut text = String::new();
            for entry in &list.entries {
                let _written = writeln!(text, "{}  {}", entry.session, line(&entry.pull_request));
            }
            text
        }
        PrCommand::Show { session, refresh } => {
            let session = session.resolve(&client).await?.id;
            let view = client.call(GetPullRequest { session, refresh }).await?;
            let mut text = format!("{}\n", access(&view.access));
            if let Some(pull) = &view.pull_request {
                text.push_str(&whole(pull));
            }
            let _written = writeln!(text, "feedback: {}", on_off(view.feedback));
            text
        }
        PrCommand::Open {
            session,
            title,
            body_file,
            base,
            draft,
        } => {
            let session = session.resolve(&client).await?.id;
            let body = match body_file {
                Some(path) => Some(body(&path).await?),
                None => None,
            };
            let request = OpenPullRequest {
                session,
                title: parse(&title, "title")?,
                body,
                base: base.map(|base| parse(&base, "base")).transpose()?,
                draft,
            };
            match client.call(request).await? {
                OpenOutcome::Opened { pull_request } => {
                    format!("opened\n{}", whole(&pull_request))
                }
                OpenOutcome::Exists { pull_request } => {
                    format!("already open\n{}", whole(&pull_request))
                }
                OpenOutcome::Refused { refusal } => {
                    return Err(CliError::Workspace(open_refusal(&refusal)));
                }
                OpenOutcome::Unknown => {
                    "the node answered something this client does not know\n".to_owned()
                }
            }
        }
        PrCommand::Merge {
            session,
            number,
            head,
            method,
        } => {
            let session = session.resolve(&client).await?.id;
            let request = MergePullRequest {
                session,
                number: PullNumber::new(number)
                    .map_err(|error| CliError::Workspace(format!("number: {error}")))?,
                method: match method {
                    Method::Merge => MergeMethod::Merge,
                    Method::Squash => MergeMethod::Squash,
                    Method::Rebase => MergeMethod::Rebase,
                },
                head: parse(&head, "head")?,
            };
            match client.call(request).await? {
                MergeOutcome::Merged { commit } => format!("merged as {commit}\n"),
                MergeOutcome::Refused { refusal } => {
                    return Err(CliError::Workspace(merge_refusal(&refusal)));
                }
                MergeOutcome::Unknown => {
                    "the node answered something this client does not know\n".to_owned()
                }
            }
        }
        PrCommand::Feedback { session, state } => {
            let session = session.resolve(&client).await?.id;
            let enabled = state == Switch::On;
            let now = client.call(SetFeedback { session, enabled }).await?;
            format!("{}\n", line(&now))
        }
        PrCommand::Status => status(&client.call(GetForgeStatus {}).await?),
    };
    Ok(io::out(text.as_bytes()).await?)
}

/// A pull request's description from `path` (`-`: standard input).
async fn body(path: &Path) -> Result<PullBody, CliError> {
    let text = if path == Path::new("-") {
        let mut text = String::new();
        let mut lines = io::input_lines()?;
        while let Some(line) = lines.recv().await {
            text.push_str(&line);
            text.push('\n');
        }
        text
    } else {
        tokio::fs::read_to_string(path).await?
    };
    parse(&text, "body")
}

fn parse<T: std::str::FromStr>(text: &str, what: &str) -> Result<T, CliError>
where
    T::Err: std::fmt::Display,
{
    text.parse()
        .map_err(|error| CliError::Workspace(format!("{what}: {error}")))
}

const fn on_off(on: bool) -> &'static str {
    if on { "on" } else { "off" }
}

/// One line of a session's pull request.
fn line(pull: &SessionPullRequest) -> String {
    match &pull.summary {
        Some(summary) => format!(
            "{}  feedback {}",
            summary_line(summary),
            on_off(pull.feedback)
        ),
        None => format!(
            "{}  feedback {}",
            access(&pull.access),
            on_off(pull.feedback)
        ),
    }
}

fn summary_line(summary: &PullSummary) -> String {
    format!(
        "#{} {:?}{} checks {:?}, review {:?}, {:?}  {}  {}",
        summary.number,
        summary.state,
        if summary.draft { " draft" } else { "" },
        summary.checks,
        summary.review,
        summary.mergeable,
        summary.title,
        summary.url,
    )
}

/// The whole pull request, for `show`, `open` and the merge's head.
fn whole(pull: &PullRequest) -> String {
    let summary = &pull.summary;
    let mut text = format!("{}\n", summary_line(summary));
    let _written = writeln!(text, "head: {} {}", pull.head_branch, summary.head_commit);
    let _written = writeln!(text, "base: {}", pull.base);
    let checks = &pull.checks;
    let _written = writeln!(
        text,
        "checks: {} passed, {} failed, {} pending{}",
        checks.passed,
        checks.failed,
        checks.pending,
        if checks.truncated {
            " (more not read)"
        } else {
            ""
        }
    );
    for check in &checks.failing {
        let _written = writeln!(text, "  failing: {}", check.name);
    }
    let reviews = &pull.reviews;
    let _written = writeln!(
        text,
        "reviews: approved {}, changes requested {}, {} unresolved",
        reviews.approved, reviews.changes_requested, reviews.unresolved
    );
    let methods: Vec<String> = pull
        .merge_methods
        .iter()
        .map(|method| format!("{method:?}").to_ascii_lowercase())
        .collect();
    let _written = writeln!(text, "merge methods: {}", methods.join(", "));
    text
}

/// Why the node can or cannot see a session's pull request.
fn access(access: &ForgeAccess) -> String {
    match access {
        ForgeAccess::Ready { login: Some(login) } => format!("signed in as {login}"),
        ForgeAccess::Ready { login: None } => "signed in".to_owned(),
        ForgeAccess::NotPlaced => "not in a project or worktree".to_owned(),
        ForgeAccess::NoBranch => "its folder is on no branch".to_owned(),
        ForgeAccess::OnBase => "its folder is on the project's default branch".to_owned(),
        ForgeAccess::NoRemote => "its branch has no remote a forge is named by".to_owned(),
        ForgeAccess::NotSupported { host } => format!("no forge on this node for {host}"),
        ForgeAccess::SignedOut { host, gh } => {
            format!("not signed in to {host} (gh: {gh:?}); run `gh auth login` on the node")
        }
        ForgeAccess::Refused {
            host,
            reason,
            login,
        } => match login {
            Some(login) => format!("{host} refused {login}: {reason:?}"),
            None => format!("{host} refused the node's token: {reason:?}"),
        },
        ForgeAccess::RateLimited { host, until } => format!("{host} limits requests until {until}"),
        ForgeAccess::Unreachable { host, retry_at } => {
            format!("{host} could not be reached; tried again at {retry_at}")
        }
        ForgeAccess::Limit => "the node watches as many as it allows".to_owned(),
        ForgeAccess::Ended => "the session has ended".to_owned(),
        ForgeAccess::Unknown => "unknown to this client".to_owned(),
    }
}

fn open_refusal(refusal: &OpenRefusal) -> String {
    match refusal {
        OpenRefusal::NotPushed => {
            "the branch is not on its remote at the folder's commit".to_owned()
        }
        OpenRefusal::NoCommits => {
            "nothing to merge: no commits between the base and the branch".to_owned()
        }
        OpenRefusal::Push { refusal } => format!("the push was refused: {}", git(refusal)),
        OpenRefusal::Access { access: why } => access(why),
        OpenRefusal::Rejected { message } => format!("the forge refused: {message}"),
        OpenRefusal::Unknown => "refused, for a reason this client does not know".to_owned(),
    }
}

fn merge_refusal(refusal: &MergeRefusal) -> String {
    match refusal {
        MergeRefusal::NoPullRequest => "the session's branch has no pull request".to_owned(),
        MergeRefusal::OtherPullRequest { number } => {
            format!("the session's pull request is #{number}")
        }
        MergeRefusal::NotOpen { state } => format!("it is {state:?}"),
        MergeRefusal::Draft => "it is a draft".to_owned(),
        MergeRefusal::HeadMoved { head } => {
            format!("its head is now {head}; nothing was merged")
        }
        MergeRefusal::NotMergeable {
            message: Some(message),
        } => {
            format!("it cannot be merged now: {message}")
        }
        MergeRefusal::NotMergeable { message: None } => "it cannot be merged now".to_owned(),
        MergeRefusal::MethodNotAllowed => "the repository does not allow that method".to_owned(),
        MergeRefusal::MergeQueue => "the repository merges through its merge queue".to_owned(),
        MergeRefusal::CannotPin => "this forge cannot pin the head for that method".to_owned(),
        MergeRefusal::Access { access: why } => access(why),
        MergeRefusal::Rejected { message } => format!("the forge refused: {message}"),
        MergeRefusal::Unknown => "refused, for a reason this client does not know".to_owned(),
    }
}

fn git(refusal: &GitRefusal) -> String {
    format!("{refusal:?}")
}

fn status(status: &ForgeStatus) -> String {
    let mut text = format!("watching {} of at most {}\n", status.watched, status.limit);
    for host in &status.hosts {
        let source = host
            .source
            .map(|source| format!(" (token from {source:?})"))
            .unwrap_or_default();
        let _written = writeln!(text, "{}: {}{source}", host.host, access(&host.access));
    }
    text
}

#[cfg(test)]
mod tests {
    use vessel_proto::forge::PullTitle;

    use super::*;

    #[test]
    fn a_title_is_one_line() {
        assert!(parse::<PullTitle>("A change", "title").is_ok());
        assert!(parse::<PullTitle>("two\nlines", "title").is_err());
    }
}
