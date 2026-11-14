//! Fleet operations on the selected node. No local membership access;
//! the invitation is read through the same confirmation as offline join.
use crate::args::Target;
use crate::connect::connect;
use crate::error::CliError;
use crate::{io, pair};
use std::fmt::Write as _;
use vessel_proto::{FleetRole, FleetSyncProblem, Scope};

pub(super) async fn join(target: &Target, yes: bool, grant: Vec<Scope>) -> Result<(), CliError> {
    let question = super::join_question(&grant);
    let offer = pair::offer_from_stdin(yes, "vessel fleet invite", &question).await?;
    let link = vessel_proto::PairingLink::encode(&offer).map_err(CliError::PairingEncode)?;
    let client = connect(target).await?.client;
    let joined = client.join_fleet(link, grant).await?;
    io::note(&format!(
        "joined the fleet of {} ({}); status reports this machine's apply history\n",
        joined.home.name, joined.home.id
    ))
    .await?;
    Ok(())
}

pub(super) async fn leave(target: &Target) -> Result<(), CliError> {
    connect(target).await?.client.leave_fleet().await?;
    io::note("this machine no longer follows a fleet; materialized files stay as they are\n")
        .await?;
    Ok(())
}

pub(super) async fn sync(target: &Target) -> Result<(), CliError> {
    connect(target).await?.client.sync_fleet().await?;
    io::note("sync requested; use fleet status on this machine to see what was applied\n").await?;
    Ok(())
}

pub(super) async fn status(target: &Target) -> Result<(), CliError> {
    let status = connect(target).await?.client.fleet_status().await?;
    let mut text = match status.role {
        FleetRole::Standalone => "role  no fleet\n".to_owned(),
        FleetRole::Home => "role  home\n".to_owned(),
        FleetRole::Member { home, joined_at } => {
            format!(
                "role  member of {home}\njoined  {}\n",
                local_time(joined_at)
            )
        }
        FleetRole::Unknown => "role  unknown; update this client\n".to_owned(),
    };
    if let Some(generation) = status.local.generation {
        let _infallible = writeln!(text, "last applied  {}", generation.get());
    } else {
        text.push_str("last applied  none\n");
    }
    if let Some(at) = status.local.applied_at {
        let _infallible = writeln!(text, "applied at  {}", local_time(at));
    }
    if let Some(at) = status.local.contacted_at {
        let _infallible = writeln!(text, "home last answered  {}", local_time(at));
    }
    for path in status.local.conflicts {
        let _infallible = writeln!(text, "kept here  {path}");
    }
    for problem in status.local.problems {
        let word = match problem {
            FleetSyncProblem::OperationFailed => "the last sync failed; check this machine's logs",
            FleetSyncProblem::ApplyFailed => "some destinations could not be written",
            FleetSyncProblem::IncompleteSources => "some sources were left out of the bundle",
            FleetSyncProblem::GrantFailed => {
                "the home could not receive the authority granted here"
            }
            FleetSyncProblem::Unknown => "a problem this client does not know; update it",
        };
        let _infallible = writeln!(text, "problem  {word}");
    }
    if let Some(members) = status.members {
        text.push_str(&super::render_members(&members));
    }
    io::out(text.as_bytes()).await?;
    Ok(())
}

/// `vessel fleet published`: what the selected machine shares, by name.
pub(super) async fn published(target: &Target) -> Result<(), CliError> {
    let published = connect(target).await?.client.fleet_published().await?;
    io::out(render_published(&published).as_bytes()).await?;
    Ok(())
}

fn render_published(published: &vessel_proto::FleetPublished) -> String {
    use vessel_proto::FleetPublished;
    let (mut text, bundle) = match published {
        FleetPublished::Standalone => {
            return "role  no fleet; this machine shares nothing\n".to_owned();
        }
        FleetPublished::Home { bundle } => ("role  home\n".to_owned(), bundle),
        FleetPublished::Member {
            home,
            bundle,
            applied,
        } => {
            let applied = applied.map_or_else(|| "none".to_owned(), |at| at.get().to_string());
            (
                format!("role  member of {home}\nlast applied  {applied}\n"),
                bundle,
            )
        }
        FleetPublished::Unknown => return "role  unknown; update this client\n".to_owned(),
    };
    let Some(bundle) = bundle else {
        text.push_str("bundle  none yet\n");
        return text;
    };
    let _infallible = writeln!(text, "version  {}", bundle.generation.get());
    for file in &bundle.files {
        let kind = match file.kind {
            vessel_proto::SharedFileKind::Instructions => "instructions",
            vessel_proto::SharedFileKind::Skill => "skill",
            vessel_proto::SharedFileKind::Other => "file",
        };
        let executable = if file.executable { "  executable" } else { "" };
        let _infallible = writeln!(
            text,
            "{kind}  {}  {} bytes{executable}",
            file.path, file.size
        );
    }
    for server in &bundle.servers {
        let _infallible = writeln!(text, "tool server  {server}");
    }
    for secret in &bundle.secrets {
        let _infallible = writeln!(text, "secret named  {secret}");
    }
    text
}

fn local_time(at: chrono::DateTime<chrono::Utc>) -> String {
    at.with_timezone(&chrono::Local)
        .format("%Y-%m-%d %H:%M:%S UTC%:z")
        .to_string()
}

#[cfg(test)]
mod tests {
    use vessel_proto::{
        FleetPublished, Generation, NodeId, PublishedBundle, SharedFile, SharedFileKind,
    };

    use super::render_published;

    #[test]
    fn published_says_the_role_and_names_without_values() {
        assert_eq!(
            render_published(&FleetPublished::Standalone),
            "role  no fleet; this machine shares nothing\n"
        );
        assert_eq!(
            render_published(&FleetPublished::Home { bundle: None }),
            "role  home\nbundle  none yet\n"
        );
        let home = NodeId::random();
        let member = FleetPublished::Member {
            home,
            bundle: Some(PublishedBundle {
                generation: Generation::new(3),
                files: vec![SharedFile {
                    path: "skills/build/run.sh".parse().unwrap(),
                    kind: SharedFileKind::Skill,
                    size: 20,
                    executable: true,
                }],
                servers: vec!["github".parse().unwrap()],
                secrets: vec!["GITHUB_TOKEN".parse().unwrap()],
            }),
            applied: Some(Generation::new(2)),
        };
        assert_eq!(
            render_published(&member),
            format!(
                "role  member of {home}\nlast applied  2\nversion  3\n\
                 skill  skills/build/run.sh  20 bytes  executable\n\
                 tool server  github\nsecret named  GITHUB_TOKEN\n"
            )
        );
    }
}
