//! Existing inbox calls only. The node is the durable lifecycle authority;
//! projection here keeps exact source identity and safe failures for the view.
use vessel_app_view::inbox::{
    Authority, Content, Counts, Entry, Extent, Key, Pages, Position, Problem, Read, Snapshot,
};
use vessel_client::NodeClient;
use vessel_proto::NodeId;
use vessel_proto::inbox::{InboxDetail, InboxEntry, InboxItem, InboxRead, ListInbox};

/// Protocol 22 pages have this documented maximum; a faulty peer must not
/// turn a bounded prefix into unbounded app allocation.
const PAGE_ENTRIES: usize = 16;

pub(super) async fn list(client: &NodeClient, pages: Pages) -> Result<Snapshot, Problem> {
    let authority = Authority::of(&client.device_self().await.map_err(problem)?.scopes);
    let mut before = None;
    let mut entries = Vec::new();
    let mut counts = None;
    let mut extent = Extent::Complete;
    let mut seen = std::collections::HashSet::new();
    for _page in 0..pages.get() {
        let page = client.inbox(ListInbox { before }).await.map_err(problem)?;
        if page.entries.len() > PAGE_ENTRIES || (page.entries.is_empty() && page.next.is_some()) {
            return Err(Problem::InvalidReply);
        }
        if counts.is_none() {
            counts = Some(Counts::new(
                page.counts.total.get(),
                page.counts.waiting.get(),
                page.counts.unread.get(),
            ));
        }
        let mut boundary = before;
        for entry in page.entries {
            if boundary.is_some_and(|prior| entry.cursor >= prior) || !seen.insert(entry.key) {
                return Err(Problem::InvalidReply);
            }
            boundary = Some(entry.cursor);
            entries.push(project(client.node().id, entry));
        }
        let Some(next) = page.next else {
            extent = Extent::Complete;
            break;
        };
        if boundary != Some(next) || before.is_some_and(|prior| next >= prior) {
            return Err(Problem::InvalidReply);
        }
        before = Some(next);
        extent = Extent::More;
    }
    let counts = counts.ok_or(Problem::InvalidReply)?;
    if counts.waiting() > counts.total() || counts.unread() > counts.total() {
        return Err(Problem::InvalidReply);
    }
    Ok(Snapshot {
        authority,
        counts,
        entries,
        extent,
    })
}

pub(super) async fn get(client: &NodeClient, key: Key) -> Result<Option<Entry>, Problem> {
    client
        .inbox_request(wire_key(key))
        .await
        .map_err(problem)
        .and_then(|entry| {
            entry
                .map(|entry| {
                    let entry = project(client.node().id, entry);
                    if entry.key == key {
                        Ok(entry)
                    } else {
                        Err(Problem::InvalidReply)
                    }
                })
                .transpose()
        })
}

pub(super) fn project(node: NodeId, entry: InboxEntry) -> Entry {
    let content = match entry.item {
        InboxItem::Permission {
            request,
            tool,
            detail,
        } => Content::Permission {
            request,
            detail,
            // Never pass a truncated command/diff off as the complete tool.
            tool: match detail {
                InboxDetail::Complete => tool,
                InboxDetail::InSession => None,
            },
        },
        InboxItem::Question { question } => Content::Question(question),
        InboxItem::InSession { .. } => Content::InSession,
        InboxItem::Finished => Content::Finished,
        InboxItem::Failed => Content::Failed,
        InboxItem::Limit => Content::Limit,
        InboxItem::Unknown => Content::Unknown,
    };
    Entry {
        position: Position::new(entry.cursor.get()),
        key: Key {
            node,
            session: entry.key.session,
            seq: entry.key.seq,
        },
        title: entry.title,
        at: entry.at,
        read: match entry.read {
            InboxRead::Read => Read::Read,
            InboxRead::Unread => Read::Unread,
        },
        content,
    }
}

pub(super) const fn wire_key(key: Key) -> vessel_proto::inbox::InboxKey {
    vessel_proto::inbox::InboxKey {
        session: key.session,
        seq: key.seq,
    }
}

pub(super) fn problem(error: vessel_client::CallError) -> Problem {
    match error {
        vessel_client::CallError::Disconnected => Problem::Offline,
        vessel_client::CallError::Remote(error) => Problem::Refused(error.code),
        vessel_client::CallError::NotInProtocol { .. }
        | vessel_client::CallError::UnassignedProtocol { .. } => {
            Problem::Refused(vessel_proto::ErrorCode::Unsupported)
        }
        vessel_client::CallError::UnexpectedReply(_) => Problem::InvalidReply,
        vessel_client::CallError::RequestIdsExhausted => Problem::Exhausted,
    }
}
