//! Inbox calls route through the store; answers keep the session actor's boundary.
use super::{Connection, NodeService, wire};
use vessel_proto::inbox::{InboxPage, ListInbox, MarkInbox};
use vessel_proto::{Ack, ErrorCode, WireError};
impl NodeService {
    pub(crate) async fn list_inbox(
        &self,
        _: &Connection,
        request: ListInbox,
    ) -> Result<InboxPage, WireError> {
        let projects = self.workspaces.list_projects().await.projects;
        let mut page = self
            .sessions
            .store()
            .inbox(request)
            .await
            .map_err(|error| wire(&error.into()))?;
        for entry in &mut page.entries {
            if let Some(info) = self
                .sessions
                .store()
                .session(entry.key.session)
                .await
                .map_err(|error| wire(&error.into()))?
            {
                let info = self.with_workspace(info);
                entry.title = info.title;
                entry.project = info.workspace.map(|place| place.project);
                entry.project_name = projects
                    .iter()
                    .find(|project| Some(project.id) == entry.project)
                    .map(|project| project.name.clone());
            }
        }
        Ok(page)
    }
    pub(crate) async fn mark_inbox(
        &self,
        connection: &Connection,
        request: MarkInbox,
    ) -> Result<Ack, WireError> {
        if connection.principal().agent().is_some() {
            return Err(WireError::new(
                ErrorCode::Forbidden,
                "an agent cannot mark a person's inbox read; open the inbox on a paired device",
            ));
        }
        self.sessions
            .mark_inbox(request)
            .await
            .map(|()| Ack)
            .map_err(|error| wire(&error))
    }
}

impl NodeService {
    pub(crate) async fn get_inbox_entry(
        &self,
        _: &Connection,
        request: vessel_proto::inbox::GetInboxEntry,
    ) -> Result<Option<vessel_proto::inbox::InboxEntry>, WireError> {
        let Some(mut entry) = self
            .sessions
            .store()
            .inbox_entry(request.key)
            .await
            .map_err(|error| wire(&error.into()))?
        else {
            return Ok(None);
        };
        if let Some(info) = self
            .sessions
            .store()
            .session(entry.key.session)
            .await
            .map_err(|error| wire(&error.into()))?
        {
            let info = self.with_workspace(info);
            entry.title = info.title;
            entry.project = info.workspace.map(|place| place.project);
            let projects = self.workspaces.list_projects().await.projects;
            entry.project_name = projects
                .iter()
                .find(|project| Some(project.id) == entry.project)
                .map(|project| project.name.clone());
        }
        if request.fetch == vessel_proto::inbox::InboxFetch::LiveRequest
            && let vessel_proto::inbox::InboxItem::InSession { request } = entry.item
        {
            let Some(item) = self
                .sessions
                .inbox_request(entry.key.session, request)
                .await
                .map_err(|error| wire(&error))?
            else {
                return Ok(None);
            };
            let compact = std::mem::replace(&mut entry.item, item);
            // Leave an explicit reference when one form exceeds the reply
            // budget; never clip a form or overflow the 16 MiB frame.
            let size = serde_json::to_vec(&entry)
                .map_err(|source| wire(&vessel_store::StoreError::Encode(source).into()))?
                .len();
            if size > 15 * 1024 * 1024 {
                entry.item = compact;
            }
        }
        Ok(Some(entry))
    }
}
