//! Transport and pure projection have a fakeable typed file boundary.
use super::{Answer, Ask, Failure, Request, RootAnswer, RootAsk, Suggestion};
use crate::connections::{RequestFailure, SelectedConnection};
use std::{collections::VecDeque, time::Duration};
use tokio::{sync::mpsc, task::JoinHandle};
use vessel_app_view::composer::{
    drafts::DraftKey,
    mentions::{Query, RESULT_LIMIT, SuggestionKind},
};
use vessel_client::CallError;
use vessel_proto::workspace::{DirListing, EntryKind, FindFiles, FoundFiles, ListDir, RelPath};

const DEADLINE: Duration = Duration::from_secs(15);
const ACTIVE: usize = 2;
const LIMIT: u32 = 50;

/// Only current visible composers need a lookup; superseded work is cancelled.
#[derive(Debug, Default)]
pub(crate) struct Hub {
    jobs: VecDeque<(DraftKey, JoinHandle<()>)>,
    root: Option<JoinHandle<()>>,
}
impl Hub {
    pub(crate) fn root(
        &mut self,
        client: Option<SelectedConnection>,
        ask: RootAsk,
        events: mpsc::Sender<crate::Event>,
    ) {
        if let Some(old) = self.root.take() {
            old.abort();
        }
        self.root = Some(tokio::spawn(async move {
            let result = match client {
                Some(client) => {
                    match tokio::time::timeout(
                        DEADLINE,
                        client.call(vessel_proto::GetSession {
                            session: ask.session,
                        }),
                    )
                    .await
                    {
                        Ok(Ok(info)) => Ok(vessel_app_view::workspace::root::session_root(&info)),
                        Ok(Err(error)) => Err(failure(&error)),
                        Err(_) => Err(Failure::TimedOut),
                    }
                }
                None => Err(Failure::Offline),
            };
            let _closed = events
                .send(crate::Event::Files(crate::attachments::Answer::Root(
                    RootAnswer { ask, result },
                )))
                .await;
        }));
    }

    pub(crate) fn ask(
        &mut self,
        client: Option<SelectedConnection>,
        request: Request,
        events: mpsc::Sender<crate::Event>,
    ) {
        self.jobs.retain(|(key, job)| {
            if *key == request.scope.draft {
                job.abort();
                false
            } else {
                !job.is_finished()
            }
        });
        while self.jobs.len() >= ACTIVE {
            if let Some((_, job)) = self.jobs.pop_front() {
                job.abort();
            }
        }
        let key = request.scope.draft;
        let job = tokio::spawn(async move {
            let result = match client {
                Some(client) => tokio::time::timeout(DEADLINE, lookup(&client, &request))
                    .await
                    .unwrap_or(Err(Failure::TimedOut)),
                None => Err(Failure::Offline),
            };
            let _closed = events
                .send(crate::Event::Files(
                    crate::attachments::Answer::Suggestions(Answer { request, result }),
                ))
                .await;
        });
        self.jobs.push_back((key, job));
    }
}

trait Source: Sync {
    fn list(&self, ask: ListDir) -> impl Future<Output = Result<DirListing, Failure>> + Send;
    fn find(&self, ask: FindFiles) -> impl Future<Output = Result<FoundFiles, Failure>> + Send;
}
impl Source for SelectedConnection {
    async fn list(&self, ask: ListDir) -> Result<DirListing, Failure> {
        self.call(ask).await.map_err(|error| failure(&error))
    }
    async fn find(&self, ask: FindFiles) -> Result<FoundFiles, Failure> {
        self.call(ask).await.map_err(|error| failure(&error))
    }
}
fn failure(error: &RequestFailure) -> Failure {
    let RequestFailure::Call(error) = error else {
        return Failure::Offline;
    };
    match error {
        CallError::Remote(wire) => Failure::Refused(wire.code),
        CallError::NotInProtocol { .. } | CallError::UnassignedProtocol { .. } => {
            Failure::Unsupported
        }
        CallError::Disconnected | CallError::RequestIdsExhausted => Failure::Offline,
        CallError::UnexpectedReply(_) => Failure::InvalidReply,
    }
}

async fn lookup(source: &impl Source, request: &Request) -> Result<Vec<Suggestion>, Failure> {
    let root = request.scope.root;
    match request.mention.query() {
        Query::Children => {
            let listing = source
                .list(ListDir {
                    root,
                    dir: RelPath::root(),
                    ignored: false,
                })
                .await?;
            project_listing(listing, &RelPath::root())
        }
        Query::Find(query) => {
            let (parent, leaf) = query
                .as_str()
                .rsplit_once('/')
                .unwrap_or(("", query.as_str()));
            let dir: RelPath = parent.parse().map_err(|_invalid| Failure::InvalidReply)?;
            let (files, listing) = tokio::join!(
                source.find(FindFiles {
                    root,
                    query: query.clone(),
                    key: None,
                    limit: Some(LIMIT)
                }),
                source.list(ListDir {
                    root,
                    dir: dir.clone(),
                    ignored: false
                }),
            );
            let mut rows: Vec<_> = files?
                .matches
                .into_iter()
                .map(|file| Suggestion {
                    path: file.path,
                    kind: SuggestionKind::File,
                })
                .take(RESULT_LIMIT)
                .collect();
            let lower = leaf.to_lowercase();
            for row in project_listing(listing?, &dir)?
                .into_iter()
                .filter(|row| row.kind == SuggestionKind::Directory)
            {
                let name = row
                    .path
                    .as_str()
                    .rsplit('/')
                    .next()
                    .unwrap_or_default()
                    .to_lowercase();
                if name.contains(&lower) && rows.len() < RESULT_LIMIT {
                    rows.push(row);
                }
            }
            Ok(rows)
        }
    }
}
fn project_listing(listing: DirListing, parent: &RelPath) -> Result<Vec<Suggestion>, Failure> {
    let mut rows = Vec::new();
    for entry in listing.entries {
        let kind = match entry.kind {
            EntryKind::File => SuggestionKind::File,
            EntryKind::Dir => SuggestionKind::Directory,
            EntryKind::Symlink | EntryKind::Other | EntryKind::Unknown => continue,
        };
        if entry.ignored {
            continue;
        }
        if entry.name.is_empty() || entry.name.contains(['/', '\\']) {
            return Err(Failure::InvalidReply);
        }
        let text = if parent.as_str().is_empty() {
            entry.name
        } else {
            format!("{}/{}", parent.as_str(), entry.name)
        };
        let path = text.parse().map_err(|_invalid| Failure::InvalidReply)?;
        if rows.len() < RESULT_LIMIT {
            rows.push(Suggestion { path, kind });
        }
    }
    Ok(rows)
}

#[cfg(test)]
mod tests;

impl crate::State {
    pub(crate) fn mention(&mut self, ask: Ask) {
        let context = match &ask {
            Ask::Root(ask) => &ask.connection,
            Ask::Suggestions(request) => &request.scope.connection,
        };
        let client = self.selected_connection(context).ok();
        match ask {
            Ask::Root(ask) => self.mentions.root(client, ask, self.events.clone()),
            Ask::Suggestions(request) => self.mentions.ask(client, request, self.events.clone()),
        }
    }
}
