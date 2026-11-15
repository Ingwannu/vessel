//! Detail actions are single-flight and refer to the source event confirmed by
//! the node, never whichever request happens to be first in a session now.
use crate::shell::Shell;
use vessel_app_kit::ui::Context;
use vessel_app_link::{
    Command,
    inbox::{Ask, Input},
};
use vessel_app_view::inbox::{Key, Operation, Read, Source};

impl Shell {
    pub(crate) fn inbox_refresh(&mut self) {
        self.command(Command::Inbox(Ask::Discover));
    }

    pub(crate) fn inbox_more(&mut self, id: vessel_proto::NodeId) {
        let Some(source) = self.areas.inbox.store.expand(id) else {
            return;
        };
        let pages = self
            .areas
            .inbox
            .store
            .nodes()
            .iter()
            .find(|node| node.source.node() == id)
            .map(|node| node.pages)
            .unwrap_or_default();
        match self.areas.inbox.store.begin(id) {
            Ok(Some(request)) => self.command(Command::Inbox(Ask::List {
                source,
                pages,
                request,
            })),
            Ok(None) => {}
            Err(problem) => self.areas.inbox.problem = Some(problem),
        }
    }

    pub(crate) fn inbox_source(&self, key: Key) -> Option<Source> {
        self.areas
            .inbox
            .store
            .nodes()
            .iter()
            .find(|node| node.source.node() == key.node)
            .map(|node| node.source.clone())
    }

    pub(crate) fn inbox_select(&mut self, key: Key, cx: &mut Context<'_, Self>) {
        if self.areas.inbox.detail.key() == Some(key) && self.areas.inbox.detail.pending().is_some()
        {
            return;
        }
        let Some(source) = self.inbox_source(key) else {
            return;
        };
        let request = match self.areas.inbox.store.ticket() {
            Ok(request) => request,
            Err(problem) => {
                self.areas.inbox.problem = Some(problem);
                return;
            }
        };
        let same_key = self.areas.inbox.detail.key() == Some(key);
        self.areas.inbox.detail_source = Some(source.clone());
        self.areas.inbox.store.select(key);
        self.areas.inbox.detail.select(key, request);
        self.areas.inbox.tool = None;
        if !same_key {
            self.areas.inbox.conversation = super::super::conversation::ConversationArea::new(cx);
        }
        self.command(Command::Inbox(Ask::Get {
            source,
            key,
            request,
        }));
    }

    fn inbox_action(
        &mut self,
        operation: Operation,
    ) -> Option<(Source, Key, vessel_app_view::inbox::Request)> {
        let key = self.areas.inbox.detail.key()?;
        let permitted = match operation {
            Operation::Answer => self.areas.inbox.store.may_answer(key),
            Operation::Mark => self.areas.inbox.store.may_mark(key),
            Operation::Load => false,
        };
        if !permitted {
            return None;
        }
        let source = self.inbox_source(key)?;
        if self.areas.inbox.detail_source.as_ref() != Some(&source) {
            return None;
        }
        let request = match self.areas.inbox.store.ticket() {
            Ok(request) => request,
            Err(problem) => {
                self.areas.inbox.problem = Some(problem);
                return None;
            }
        };
        let _same_key = self.areas.inbox.detail.begin(request, operation)?;
        Some((source, key, request))
    }

    pub(crate) fn inbox_mark(&mut self, read: Read) {
        let Some((source, key, request)) = self.inbox_action(Operation::Mark) else {
            return;
        };
        self.command(Command::Inbox(Ask::Mark {
            source,
            key,
            read,
            request,
        }));
    }

    pub(crate) fn inbox_send_answer(&mut self, input: Input) {
        let Some(entry) = self.areas.inbox.detail.entry() else {
            return;
        };
        if let Err(problem) = input.validate(entry) {
            let kind = match input {
                Input::Permission(..) => vessel_app_link::ActKind::Permit,
                Input::Question(..) => vessel_app_link::ActKind::Answer,
            };
            self.areas.inbox.conversation.refused = Some((
                kind,
                vessel_app_view::inbox::words::problem(problem, self.words).to_owned(),
            ));
            return;
        }
        let Some((source, key, request)) = self.inbox_action(Operation::Answer) else {
            return;
        };
        self.command(Command::Inbox(Ask::Answer {
            source,
            key,
            input,
            request,
        }));
    }
}
