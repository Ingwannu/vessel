//! Native discovery uses the canonical request and only implicit compatibility fallback.
use crate::{
    args::{Discovery, Target},
    connect::connect,
    error::CliError,
    io, render,
};
use vessel_client::{CallError, NodeClient};
use vessel_proto::{ListSessions, Request as _, SessionInfo, SessionSelection};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Intent {
    Implicit,
    Explicit,
}
struct Query {
    request: ListSessions,
    intent: Intent,
}
impl Query {
    fn new(args: &Discovery) -> Result<Self, CliError> {
        let explicit = args.background || !args.label.is_empty();
        let labels = crate::labels::parse(&args.label)?;
        let scope = if args.background {
            SessionSelection::all()
        } else {
            SessionSelection::shown()
        };
        Ok(Self {
            request: ListSessions::selected(scope.with_labels(labels)),
            intent: if explicit {
                Intent::Explicit
            } else {
                Intent::Implicit
            },
        })
    }
    async fn read(self, client: &NodeClient) -> Result<Vec<SessionInfo>, CallError> {
        match client.check_call(&self.request.clone().into_call()) {
            Ok(()) => client.call(self.request).await,
            Err(CallError::UnassignedProtocol { .. } | CallError::NotInProtocol { .. })
                if self.intent == Intent::Implicit =>
            {
                client.sessions().await
            }
            Err(error) => Err(error),
        }
    }
}
pub(crate) async fn list(target: &Target, args: &Discovery) -> Result<(), CliError> {
    let query = Query::new(args)?;
    let client = connect(target).await?.client;
    let sessions = query.read(&client).await?;
    Ok(io::out(render::sessions(&sessions).as_bytes()).await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::ListingScope;
    #[test]
    fn explicit_filters_preserve_their_scope_and_labels_in_the_actual_request() {
        for (background, scope) in [(false, ListingScope::Shown), (true, ListingScope::All)] {
            let query = Query::new(&Discovery {
                background,
                label: vec!["task=one".to_owned()],
            })
            .unwrap();
            assert_eq!(query.intent, Intent::Explicit);
            let selected = query.request.selection().unwrap();
            assert_eq!(selected.scope, scope);
            assert_eq!(selected.labels.iter().next().unwrap().0.as_str(), "task");
            assert_eq!(selected.labels.iter().next().unwrap().1.as_str(), "one");
        }
        let implicit = Query::new(&Discovery {
            background: false,
            label: Vec::new(),
        })
        .unwrap();
        assert_eq!(implicit.intent, Intent::Implicit);
        assert_eq!(
            implicit.request.selection().unwrap().scope,
            ListingScope::Shown
        );
    }
}
