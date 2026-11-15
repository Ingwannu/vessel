//! Current-ticket replies update the one store. A background refresh does not
//! reset the selected form; only an exact detail response can change it.
use crate::shell::Shell;
use vessel_app_kit::ui::Context;
use vessel_app_link::{
    Command,
    inbox::{Answer, Ask, Target},
};
use vessel_app_view::inbox::{Content, Problem, Request, Snapshot, Source};
use vessel_app_view::timeline::{Live, ToolView};

impl Shell {
    pub(crate) fn inbox_answer(&mut self, answer: Answer, cx: &mut Context<'_, Self>) -> bool {
        let mut changed = true;
        match answer {
            Answer::Terminal(source, action, request, result) => {
                return self.inbox_terminal_answer(source, action, request, result, cx);
            }
            Answer::Claimed(source, key, request, result) => {
                changed = false;
                if let Some(pending) = self.areas.inbox.admission.take(&source, key, request) {
                    changed = true;
                    match result {
                        Ok(vessel_app_view::inbox::admission::Admission::Admitted) => {
                            if self.areas.inbox.granted.len()
                                < vessel_app_view::inbox::notices::MAX_PENDING_NOTICES
                            {
                                self.areas.inbox.granted.push(pending);
                            }
                        }
                        Ok(vessel_app_view::inbox::admission::Admission::Suppressed) => {}
                        Err(problem) => self.areas.inbox.problem = Some(problem),
                    }
                }
                if changed {
                    self.inbox_notice_update(cx);
                }
            }
            Answer::Targets(Ok(targets)) => {
                let sources = self
                    .areas
                    .inbox
                    .store
                    .read_routes(targets.iter().filter_map(|target| match target {
                        Target::Online(source) => Some(source.clone()),
                        Target::Offline(_) => None,
                    }));
                let targets: Vec<_> = targets
                    .into_iter()
                    .filter(|target| match target {
                        Target::Online(source) => sources.contains(source),
                        Target::Offline(_) => true,
                    })
                    .collect();
                changed = self.areas.inbox.targets != targets || self.areas.inbox.problem.is_some();
                self.inbox_targets(targets, cx);
            }
            Answer::Targets(Err(problem)) => self.areas.inbox.problem = Some(problem),
            Answer::ReadSession(source, session, request, result) => {
                let target = vessel_app_view::inbox::reading::Target {
                    node: source.node(),
                    session,
                };
                if self.areas.inbox.read_last == Some((request, target))
                    && self.areas.inbox.read_source.as_ref() == Some(&source)
                {
                    self.areas.inbox.read_problem = result.err();
                    self.inbox_refresh();
                }
            }
            Answer::Presence(source, result) => {
                changed = self.inbox_presence_answer(source, result)
            }
            Answer::Listed(source, request, result) => {
                changed = self.inbox_listed(source, request, result, cx)
            }
            Answer::Detail(source, key, request, result) => {
                changed = false;
                if self.areas.inbox.detail_source.as_ref() == Some(&source)
                    && self
                        .areas
                        .inbox
                        .targets
                        .iter()
                        .any(|target| matches!(target, Target::Online(known) if *known == source))
                    && self.areas.inbox.detail.receive(key, request, result)
                {
                    changed = true;
                    self.inbox_cards();
                    self.inbox_refresh();
                }
            }
        }
        changed
    }

    fn inbox_presence_answer(&mut self, source: Source, result: Result<(), Problem>) -> bool {
        let mut changed = false;
        if self
            .areas
            .inbox
            .store
            .nodes()
            .iter()
            .any(|node| node.source == source)
        {
            changed = self
                .areas
                .inbox
                .presence_problems
                .iter()
                .find(|(id, _)| *id == source.node())
                .map(|(_, problem)| *problem)
                != result.as_ref().err().copied();
            self.areas
                .inbox
                .presence_problems
                .retain(|(id, _)| *id != source.node());
            if let Err(problem) = result {
                self.areas
                    .inbox
                    .presence_problems
                    .push((source.node(), problem));
            }
        }
        changed
    }

    fn inbox_listed(
        &mut self,
        source: Source,
        request: Request,
        result: Result<Snapshot, Problem>,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        if !self
            .areas
            .inbox
            .targets
            .iter()
            .any(|target| matches!(target, Target::Online(known) if *known == source))
        {
            return false;
        }
        let (visible, notices) =
            self.areas
                .inbox
                .store
                .receive_visible(source.node(), request, result);
        let changed = visible;
        let (opportunities, coalescing) = self.areas.inbox.store.take_terminal_notices();
        let queued = &mut self.areas.inbox.terminals.queued;
        let remaining =
            vessel_app_view::inbox::notices::MAX_PENDING_NOTICES.saturating_sub(queued.len());
        let overflow = opportunities.len() > remaining;
        queued.extend(opportunities.into_iter().take(remaining));
        if overflow || coalescing == vessel_app_view::inbox::notices::Coalescing::Combined {
            self.areas.inbox.coalescing = vessel_app_view::inbox::notices::Coalescing::Combined;
        }
        let area = &mut self.areas.inbox;
        area.notices.retain(|key| area.store.entry(*key).is_some());
        area.notices.extend(notices);
        if changed {
            self.inbox_notice_update(cx);
        }
        changed
    }

    fn inbox_targets(&mut self, targets: Vec<Target>, cx: &mut Context<'_, Self>) {
        let old = self.areas.inbox.store.nodes();
        let retained: Vec<_> = targets
            .iter()
            .filter_map(|target| {
                target.node().or_else(|| {
                    old.iter()
                        .find(|node| node.source.name() == target.route())
                        .map(|node| node.source.node())
                })
            })
            .collect();
        self.areas.inbox.store.retain(&retained);
        self.areas
            .inbox
            .presence_problems
            .retain(|(id, _)| retained.contains(id));
        if self
            .areas
            .inbox
            .detail
            .key()
            .is_some_and(|key| !retained.contains(&key.node))
        {
            self.areas.inbox.detail = vessel_app_view::inbox::Detail::default();
            super::place::clear_cards(&mut self.areas.inbox.conversation);
        }
        self.areas.inbox.problem = None;
        let sources = targets
            .iter()
            .filter_map(|target| match target {
                Target::Online(source) => Some(source.clone()),
                Target::Offline(_) => None,
            })
            .collect();
        for source in self.areas.inbox.store.poll_routes(sources) {
            let id = source.node();
            let changed =
                self.areas.inbox.store.nodes().iter().any(|node| {
                    node.source.node() == id && node.source.context() != source.context()
                });
            let connected = self
                .areas
                .inbox
                .store
                .nodes()
                .iter()
                .any(|node| node.source == source);
            self.areas.inbox.store.connect(source.clone());
            if !connected {
                self.command(Command::Inbox(Ask::Presence {
                    source: source.clone(),
                    report: self.areas.inbox.presence.snapshot(),
                }));
            }
            if changed && let Some(key) = self.areas.inbox.detail.key().filter(|key| key.node == id)
            {
                self.areas.inbox.detail.retire();
                self.inbox_select(key, cx);
            }
            match self.areas.inbox.store.begin(id) {
                Ok(Some(request)) => {
                    let pages = self
                        .areas
                        .inbox
                        .store
                        .nodes()
                        .iter()
                        .find(|node| node.source.node() == id)
                        .map(|node| node.pages)
                        .unwrap_or_default();
                    self.command(Command::Inbox(Ask::List {
                        source,
                        pages,
                        request,
                    }));
                }
                Ok(None) => {}
                Err(problem) => self.areas.inbox.problem = Some(problem),
            }
        }
        for target in &targets {
            if matches!(target, Target::Offline(_))
                && let Some(node) = self
                    .areas
                    .inbox
                    .store
                    .nodes()
                    .iter()
                    .find(|node| node.source.name() == target.route())
            {
                let id = node.source.node();
                self.areas.inbox.store.disconnect(id);
            }
        }
        self.areas.inbox.targets = targets;
    }

    fn inbox_cards(&mut self) {
        let area = &mut self.areas.inbox;
        area.conversation.permit_sending = None;
        area.conversation.refused = None;
        area.tool = None;
        let Some(entry) = area.detail.entry() else {
            super::place::clear_cards(&mut area.conversation);
            return;
        };
        let Some(source) = area
            .store
            .nodes()
            .iter()
            .find(|node| node.source.node() == entry.key.node)
        else {
            return;
        };
        area.conversation
            .open(source.source.name().as_str(), entry.key.session);
        area.conversation.live = match &entry.content {
            Content::Permission { request, tool, .. } => {
                area.tool = tool.as_ref().map(ToolView::announced);
                Live {
                    permissions: vec![request.clone()],
                    questions: Vec::new(),
                }
            }
            Content::Question(question) => Live {
                permissions: Vec::new(),
                questions: vec![question.clone()],
            },
            Content::InSession
            | Content::Finished
            | Content::Failed
            | Content::Limit
            | Content::Unknown => Live::default(),
        };
        if let Some(problem) = area.detail.problem() {
            let message = vessel_app_view::inbox::words::problem(problem, self.words).to_owned();
            let kind = match &entry.content {
                Content::Question(_) => vessel_app_link::ActKind::Answer,
                Content::Permission { .. }
                | Content::InSession
                | Content::Finished
                | Content::Failed
                | Content::Limit
                | Content::Unknown => vessel_app_link::ActKind::Permit,
            };
            area.conversation.refused = Some((kind, message));
        }
    }
}
