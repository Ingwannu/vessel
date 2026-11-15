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
    let head = read_page(client, None).await?;
    let counts = Counts::new(
        head.counts.total.get(),
        head.counts.waiting.get(),
        head.counts.unread.get(),
    );
    if counts.waiting() > counts.total() || counts.unread() > counts.total() {
        return Err(Problem::InvalidReply);
    }
    let mut entries: Vec<_> = head
        .entries
        .into_iter()
        .map(|entry| project(client.node().id, entry))
        .collect();
    let next = if let Some(before) = pages.before() {
        let older = read_page(
            client,
            Some(vessel_proto::inbox::InboxCursor::new(before.get())),
        )
        .await?;
        let mut keys: std::collections::HashSet<_> =
            entries.iter().map(|entry| entry.key).collect();
        entries.extend(
            older
                .entries
                .into_iter()
                .map(|entry| project(client.node().id, entry))
                .filter(|entry| keys.insert(entry.key)),
        );
        entries.sort_by_key(|entry| std::cmp::Reverse(entry.position));
        older.next
    } else {
        head.next
    };
    Ok(Snapshot {
        authority,
        attention: attention(client).await?,
        counts,
        entries,
        next: next.map(|cursor| Position::new(cursor.get())),
        extent: if next.is_some() {
            Extent::More
        } else {
            Extent::Complete
        },
    })
}

async fn read_page(
    client: &NodeClient,
    before: Option<vessel_proto::inbox::InboxCursor>,
) -> Result<vessel_proto::inbox::InboxPage, Problem> {
    let page = client.inbox(ListInbox { before }).await.map_err(problem)?;
    if page.entries.len() > PAGE_ENTRIES || (page.entries.is_empty() && page.next.is_some()) {
        return Err(Problem::InvalidReply);
    }
    let mut boundary = before;
    let mut keys = std::collections::HashSet::new();
    for entry in &page.entries {
        if boundary.is_some_and(|prior| entry.cursor >= prior) || !keys.insert(entry.key) {
            return Err(Problem::InvalidReply);
        }
        boundary = Some(entry.cursor);
    }
    if page.next.is_some() && page.next != boundary {
        return Err(Problem::InvalidReply);
    }
    Ok(page)
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
        project: entry.project_name,
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

async fn attention(
    client: &NodeClient,
) -> Result<Option<Vec<vessel_proto::inbox_attention::SessionAttention>>, Problem> {
    use vessel_proto::inbox_attention::ATTENTION_BATCH;
    if !vessel_proto::Method::GetInboxAttention
        .since()
        .is_some_and(|since| since <= client.protocol())
    {
        return Ok(None);
    }
    let sessions = client.sessions().await.map_err(problem)?;
    let ids: Vec<_> = sessions.into_iter().map(|session| session.id).collect();
    let mut attention = Vec::with_capacity(ids.len());
    for chunk in ids.chunks(ATTENTION_BATCH) {
        let badges = client
            .inbox_attention(chunk.to_vec())
            .await
            .map_err(problem)?;
        let mut seen = std::collections::HashSet::new();
        for badge in &badges {
            if !chunk.contains(&badge.session) || !seen.insert(badge.session) {
                return Err(Problem::InvalidReply);
            }
        }
        attention.extend(badges);
    }
    Ok(Some(attention))
}
