//! The one root receipt cache serves mentions without opening the Explorer.
//! A session id alone is never a cache key: node, session and original
//! connection must all agree before a receipt can supply request facts.
use super::scope::Acquisition;
use crate::shell::Shell;
use std::collections::HashMap;
use vessel_app_link::workspace::{FileAsk, scoped::Request};
use vessel_app_view::connection::ConnectionContext;
use vessel_app_view::workspace::{file_scope::FileScope, problem::Problem, root::session_root};
use vessel_proto::workspace::{FindFiles, FindQuery, FoundFiles, SearchKey, WorkspaceRoot};
use vessel_proto::{NodeId, SessionId, SessionInfo};
const MAX_ROOT_RECEIPTS: usize = 64;

/// Why the existing mention lookup could not be dispatched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FindRefused {
    NoSession,
    Reading,
    Outside,
    Query,
    Failed(String),
}
struct Waiting {
    context: ConnectionContext,
    session: SessionId,
    key: SearchKey,
    query: FindQuery,
    limit: u32,
}
struct RootReceipt {
    context: ConnectionContext,
    root: Result<Option<WorkspaceRoot>, String>,
}
#[derive(Default)]
pub(super) struct Finder {
    roots: HashMap<(NodeId, SessionId), RootReceipt>,
    asked: HashMap<(NodeId, SessionId), Acquisition>,
    waiting: Option<Waiting>,
}
impl Shell {
    #[expect(
        dead_code,
        reason = "B1 mention consumer uses the canonical file-root seam"
    )]
    pub(crate) fn code_find_files(
        &mut self,
        key: SearchKey,
        query: &str,
        limit: u32,
    ) -> Result<(), FindRefused> {
        let (machine, session) = self.code_session().ok_or(FindRefused::NoSession)?;
        let context = self
            .file_connection(&machine)
            .ok_or_else(|| FindRefused::Failed("the original connection is unavailable".into()))?;
        let query: FindQuery = query
            .trim()
            .parse()
            .map_err(|_invalid_query| FindRefused::Query)?;
        let cache_key = (context.node(), session);
        let shown = self.file_workspace_scope().map(|(_, root)| Ok(Some(root)));
        let root = shown.or_else(|| {
            self.areas
                .workspace
                .code
                .finder
                .roots
                .get(&cache_key)
                .filter(|receipt| receipt.context == context)
                .map(|receipt| receipt.root.clone())
        });
        match root {
            Some(Ok(Some(root))) => {
                self.send_file_scope(
                    FileScope::new(context, root),
                    find_ask(root, query, key, limit),
                );
                Ok(())
            }
            Some(Ok(None)) => Err(FindRefused::Outside),
            Some(Err(why)) => {
                let _removed = self.areas.workspace.code.finder.roots.remove(&cache_key);
                Err(FindRefused::Failed(why))
            }
            None => {
                self.areas.workspace.code.finder.waiting = Some(Waiting {
                    context: context.clone(),
                    session,
                    key,
                    query,
                    limit,
                });
                let already = self
                    .areas
                    .workspace
                    .code
                    .finder
                    .asked
                    .get(&cache_key)
                    .is_some_and(|read| read.context == context);
                if !already {
                    let ticket = self
                        .next_file_ticket()
                        .ok_or_else(|| FindRefused::Failed("file identities exhausted".into()))?;
                    let finder = &mut self.areas.workspace.code.finder;
                    if finder.asked.len() >= MAX_ROOT_RECEIPTS {
                        return Err(FindRefused::Failed("file root lookups are busy".into()));
                    }
                    let _previous = finder.asked.insert(
                        cache_key,
                        Acquisition {
                            context: context.clone(),
                            ticket,
                        },
                    );
                    self.command(vessel_app_link::Command::FileOperation(Request::root(
                        context, session, ticket,
                    )));
                }
                Err(FindRefused::Reading)
            }
        }
    }
    pub(super) fn finder_scoped_root(
        &mut self,
        context: &ConnectionContext,
        session: SessionId,
        ticket: vessel_app_view::workspace::file_ticket::FileTicket,
        result: &Result<Box<SessionInfo>, Problem>,
        live: bool,
    ) {
        let words = self.words();
        let finder = &mut self.areas.workspace.code.finder;
        let key = (context.node(), session);
        let owns = finder
            .asked
            .get(&key)
            .is_some_and(|read| read.context == *context && read.ticket == ticket);
        if !owns {
            return;
        }
        let _settled = finder.asked.remove(&key);
        if !live {
            let _stale = finder
                .waiting
                .take_if(|waiting| waiting.context == *context && waiting.session == session);
            return;
        }
        let root = result
            .as_ref()
            .map(|info| session_root(info))
            .map_err(|problem| words.problem(problem));
        if finder.roots.len() >= MAX_ROOT_RECEIPTS && !finder.roots.contains_key(&key) {
            finder.roots.clear();
        }
        let _previous = finder.roots.insert(
            key,
            RootReceipt {
                context: context.clone(),
                root: root.clone(),
            },
        );
        let waiting = finder
            .waiting
            .take_if(|waiting| waiting.context == *context && waiting.session == session);
        if let (Some(waiting), Ok(Some(root))) = (waiting, root) {
            self.send_file_scope(
                FileScope::new(context.clone(), root),
                find_ask(root, waiting.query, waiting.key, waiting.limit),
            );
        }
    }
}
impl Finder {
    pub(super) fn machine_gone(&mut self, name: &str) {
        self.roots
            .retain(|_, receipt| receipt.context.machine().as_str() != name);
        self.asked
            .retain(|_, read| read.context.machine().as_str() != name);
        let _old = self
            .waiting
            .take_if(|waiting| waiting.context.machine().as_str() == name);
    }
}
fn find_ask(root: WorkspaceRoot, query: FindQuery, key: SearchKey, limit: u32) -> FileAsk {
    FileAsk::Find(FindFiles {
        root,
        query,
        key: Some(key),
        limit: Some(limit),
    })
}
pub(super) fn found_elsewhere(
    _shell: &mut Shell,
    _ask: &FindFiles,
    _result: Result<FoundFiles, Problem>,
) {
}
