//! A bounded project connection decoded through the same summary policy
//! as a whole read. Partial GraphQL errors are handled before this path;
//! malformed rows or continuation metadata never produce a partial page.

use std::fmt;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use vessel_proto::forge::{OpenPullPage, PullCursor, RefusedReason, RepoRef};

use super::summary;
use crate::provider::ForgeError;

pub(in crate::github) const LIST: &str = "query($owner:String!,$name:String!,$first:Int!,$after:String,$contexts:Int!,$heads:Int!){\
repository(owner:$owner,name:$name){pullRequests(first:$first,after:$after,states:[OPEN],\
orderBy:{field:UPDATED_AT,direction:DESC}){pageInfo{hasNextPage endCursor}\
nodes{number url title state isDraft updatedAt headRefOid mergeable mergeStateStatus reviewDecision \
commits(last:$heads){nodes{commit{statusCheckRollup{contexts(first:$contexts){totalCount nodes{__typename \
...on CheckRun{name status conclusion detailsUrl startedAt} \
...on StatusContext{context state targetUrl createdAt}}}}}}}}}}}";

#[derive(Debug, Deserialize)]
pub(in crate::github) struct PageData {
    repository: Option<Repository>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Repository {
    pull_requests: Connection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Connection {
    nodes: Vec<summary::Fields>,
    page_info: PageInfo,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    has_next_page: bool,
    end_cursor: Option<EndCursor>,
}

// The unused final-page cursor is not retained. A continuation is parsed
// into the validating public cursor only when the provider says it is
// needed; neither raw nor parsed cursor content belongs in diagnostics.
#[derive(Deserialize)]
#[serde(transparent)]
struct EndCursor(String);

impl fmt::Debug for EndCursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("EndCursor(..)")
    }
}

pub(in crate::github) fn page(
    repo: &RepoRef,
    data: PageData,
    after: Option<&PullCursor>,
    read_at: DateTime<Utc>,
) -> Result<OpenPullPage, ForgeError> {
    let connection = data
        .repository
        .ok_or(ForgeError::Refused {
            reason: RefusedReason::NoAccess,
        })?
        .pull_requests;
    if connection.nodes.len() > OpenPullPage::MAX_PULLS {
        return Err(ForgeError::Page(
            vessel_proto::forge::InvalidOpenPullPage::TooMany,
        ));
    }
    let next = if connection.page_info.has_next_page {
        let cursor: PullCursor = connection
            .page_info
            .end_cursor
            .ok_or(ForgeError::Unexpected {
                what: "a project page's next cursor",
            })?
            .0
            .parse()
            .map_err(ForgeError::PageCursor)?;
        if Some(&cursor) == after {
            return Err(ForgeError::Unexpected {
                what: "a project page cursor that advances",
            });
        }
        Some(cursor)
    } else {
        None
    };
    let pulls = connection
        .nodes
        .into_iter()
        .map(|fields| {
            let head = fields.head()?;
            summary::read(repo, fields, head).map(|(summary, _checks)| summary)
        })
        .collect::<Result<Vec<_>, _>>()?;
    OpenPullPage::new(repo.clone(), pulls, next, read_at).map_err(ForgeError::Page)
}
