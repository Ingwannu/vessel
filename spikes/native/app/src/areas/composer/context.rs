//! All producers share this validated intake and the kit's attachment pills.
//! Source snapshots stay with their composer; replacing or suppressing one
//! does not mutate another panel's registry or fetch its URL during send.
use crate::shell::Shell;
use gpui::FocusHandle;
use vessel_app_kit::attachment::{Body, Standing, attachment_pill_scoped, attachment_tray};
use vessel_app_kit::dialog::{DialogContent, DialogSize, DialogState, dialog};
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, SharedString, Styled as _, Window,
    div,
};
use vessel_app_kit::{Tokens, assets::Glyph};
use vessel_app_view::composer::{
    context::{ContextAttachment, ContextKind, ContextProblem, ContextScope},
    drafts::DraftKey,
};

#[derive(Debug)]
pub(super) struct Preview {
    dialog: DialogState,
    item: Option<ContextAttachment>,
    previous: Option<FocusHandle>,
}
impl Preview {
    pub(super) fn new(cx: &mut vessel_app_kit::ui::App) -> Self {
        Self {
            dialog: DialogState::new(cx),
            item: None,
            previous: None,
        }
    }
}
impl Shell {
    /// Until resident contexts land, this is the explicit route-to-draft bridge.
    pub(crate) fn composer_destination(&self) -> Option<DraftKey> {
        match self.route {
            crate::routes::Route::Draft | crate::routes::Route::NewWorkspace => Some(DraftKey::New),
            crate::routes::Route::Session => self
                .areas
                .session
                .open
                .as_ref()
                .filter(|open| open.structured)
                .map(|open| DraftKey::Session {
                    session: open.session,
                }),
            crate::routes::Route::Board
            | crate::routes::Route::Archive
            | crate::routes::Route::Settings
            | crate::routes::Route::Pair => None,
        }
    }
    /// Producers use one localized presentation for a rejected context intake.
    pub(crate) fn composer_context_refused(
        &mut self,
        problem: ContextProblem,
        cx: &mut Context<'_, Self>,
    ) {
        let words = self.words.composer();
        let message = match problem {
            ContextProblem::Busy => words.context_busy(),
            ContextProblem::Scope => words.context_scope(),
            ContextProblem::Capacity => words.context_capacity(),
            ContextProblem::Exhausted => words.context_unavailable(),
            ContextProblem::Identity
            | ContextProblem::Title
            | ContextProblem::Text
            | ContextProblem::Url
            | ContextProblem::Lines => words.context_invalid(),
        };
        self.toast(message, vessel_app_view::toasts::ToastTone::Error, cx);
    }
    /// File/browser/review/forge/plugin owners adapt their canonical values
    /// here. A producer cannot redirect context into another workspace.
    pub(crate) fn composer_attach_context(
        &mut self,
        target: DraftKey,
        scope: &ContextScope,
        item: ContextAttachment,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Result<(), ContextProblem> {
        if self.composer_destination() != Some(target)
            || self.composer_context_scope(target).as_ref() != Some(scope)
        {
            return Err(ContextProblem::Scope);
        }
        if let ContextAttachment::WorkspaceFile(file) = &item
            && &file.scope != scope
        {
            return Err(ContextProblem::Scope);
        }
        self.context_intake_tray(target)?.reattach(item)?;
        if let Some(field) = self.composer_field(target) {
            field.focus(window, cx);
        }
        cx.notify();
        Ok(())
    }
    /// The creation area seeds this only after claiming the active draft's canonical target.
    /// History has no destination root until that draft's placement is chosen.
    pub(crate) fn composer_seed_history(
        &mut self,
        target: DraftKey,
        history: vessel_app_view::composer::context::ChatHistory,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Result<(), ContextProblem> {
        if matches!(target, DraftKey::Session { .. }) || self.composer_field(target).is_none() {
            return Err(ContextProblem::Scope);
        }
        if self.composer_draft_has_payload(target, cx) {
            return Err(ContextProblem::Busy);
        }
        self.context_intake_tray(target)?
            .reattach(history.into_attachment())?;
        if target == DraftKey::New
            && let Some(field) = self.composer_field(target)
        {
            field.focus(window, cx);
        }
        cx.notify();
        Ok(())
    }
    /// Seeding must not overwrite whitespace, suppressed context, or pending work.
    pub(crate) fn draft_has_payload(&self, cx: &vessel_app_kit::ui::App) -> bool {
        self.composer_draft_has_payload(DraftKey::New, cx)
            || !self.areas.draft.creation.flow.may_submit()
    }

    fn context_intake_tray(
        &mut self,
        target: DraftKey,
    ) -> Result<&mut vessel_app_view::composer::context::ContextTray, ContextProblem> {
        self.prune_empty_attachment_slots();
        let reserved = self.reserved_attachment_slots().1;
        let contexts = &mut self.areas.composer.contexts;
        if contexts.get(target).is_none()
            && contexts.len().saturating_add(reserved)
                >= vessel_app_view::composer::trays::Trays::MAX
        {
            return Err(ContextProblem::Capacity);
        }
        Ok(contexts.tray(target))
    }

    /// A scoped seed cannot replace text, suppressed source context or pending bytes.
    pub(crate) fn composer_draft_has_payload(
        &self,
        target: DraftKey,
        cx: &vessel_app_kit::ui::App,
    ) -> bool {
        self.composer_field(target)
            .is_none_or(|field| !field.text(cx).is_empty())
            || !self.areas.composer.drafts.text(target).is_empty()
            || self.composer_has_files(target)
            || self
                .areas
                .composer
                .contexts
                .get(target)
                .is_some_and(vessel_app_view::composer::context::ContextTray::has_retained)
            || self.composer_attachments_blocked(target)
            || match target {
                DraftKey::New => self.areas.composer.starting.borrow().is_some(),
                DraftKey::Workspace { .. } => self
                    .areas
                    .composer
                    .workspace_starting
                    .keys()
                    .any(|(held, _)| *held == target),
                DraftKey::Session { session } => self.areas.composer.sends.contains_key(&session),
            }
    }

    pub(super) fn prepare_session_context(
        &mut self,
        session: vessel_proto::SessionId,
        cx: &mut Context<'_, Self>,
    ) -> Result<Option<vessel_app_view::composer::context::Submission>, ContextProblem> {
        let result = self
            .areas
            .composer
            .contexts
            .get_mut(DraftKey::Session { session })
            .map(vessel_app_view::composer::context::ContextTray::prepare)
            .transpose()
            .map(Option::flatten);
        if let Err(problem) = result {
            self.composer_context_refused(problem, cx);
        }
        result
    }

    pub(crate) fn composer_has_context(&self, target: DraftKey) -> bool {
        self.areas
            .composer
            .contexts
            .get(target)
            .is_some_and(|tray| !tray.is_empty())
    }
    fn context_removed(&mut self, target: DraftKey, index: usize, cx: &mut Context<'_, Self>) {
        if let Some(tray) = self.areas.composer.contexts.get_mut(target) {
            let item = tray.visible().nth(index).cloned();
            if let Some(item) = item {
                tray.remove(&item);
            }
        }
        cx.notify();
    }
    fn context_opened(
        &mut self,
        target: DraftKey,
        index: usize,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let item = self
            .areas
            .composer
            .contexts
            .get(target)
            .and_then(|tray| tray.visible().nth(index))
            .cloned();
        let Some(item) = item else {
            return;
        };
        if matches!(item, ContextAttachment::WorkspaceFile(_)) {
            return;
        }
        if let ContextAttachment::Snapshot {
            kind:
                ContextKind::ForgeIssue | ContextKind::ForgeChangeRequest | ContextKind::PluginResource,
            payload,
        } = &item
            && let Some(url) = &payload.url
        {
            cx.open_url(url.as_str());
            return;
        }
        let preview = &mut self.areas.composer.context_preview;
        if preview.item.is_none() {
            preview.previous = window.focused(cx);
        }
        preview.item = Some(item);
        preview.dialog.focus(window, cx);
        cx.notify();
    }
    fn context_closed(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let preview = &mut self.areas.composer.context_preview;
        preview.item = None;
        if let Some(previous) = preview.previous.take() {
            previous.focus(window, cx);
        }
        cx.notify();
    }
}

pub(super) fn tray(
    shell: &Shell,
    target: DraftKey,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    if !shell.composer_has_context(target) {
        return None;
    }
    let tray = shell.areas.composer.contexts.get(target)?;
    let pills = tray.visible().enumerate().map(|(index, item)| {
        let open = (!matches!(item, ContextAttachment::WorkspaceFile(_))).then(|| {
            std::rc::Rc::new(
                move |shell: &mut Shell,
                      index,
                      window: &mut Window,
                      cx: &mut Context<'_, Shell>| {
                    shell.context_opened(target, index, window, cx)
                },
            ) as vessel_app_kit::attachment::ScopedPillClick<Shell>
        });
        let remove = std::rc::Rc::new(
            move |shell: &mut Shell, index, _window: &mut Window, cx: &mut Context<'_, Shell>| {
                shell.context_removed(target, index, cx)
            },
        );
        let words = shell.words.composer();
        let (icon, subtitle) = match item {
            ContextAttachment::WorkspaceFile(file) => {
                (Glyph::FileText, file.path.as_str().to_owned())
            }
            ContextAttachment::Snapshot { kind, .. } => match kind {
                ContextKind::BrowserElement => (Glyph::Monitor, words.context_browser().to_owned()),
                ContextKind::Review => (Glyph::FileText, words.context_review().to_owned()),
                ContextKind::ChatHistory => (Glyph::Message, words.context_history().to_owned()),
                ContextKind::PullComment
                | ContextKind::PullReview
                | ContextKind::PullCheck
                | ContextKind::ForgeChangeRequest => {
                    (Glyph::PullRequest, words.context_pull().to_owned())
                }
                ContextKind::ForgeIssue => (Glyph::CircleDot, words.context_issue().to_owned()),
                ContextKind::PluginResource => (Glyph::Boxes, words.context_plugin().to_owned()),
            },
        };
        attachment_pill_scoped(
            "composer-context",
            index,
            &Body::Label {
                icon,
                title: SharedString::from(item.title().to_owned()),
                subtitle: subtitle.into(),
            },
            Standing::Quiet,
            open,
            remove,
            touch,
            tokens,
            cx,
        )
    });
    Some(attachment_tray().children(pills).into_any_element())
}

pub(super) fn preview(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let preview = &shell.areas.composer.context_preview;
    let item = preview.item.as_ref()?;
    Some(dialog(
        &preview.dialog,
        DialogContent {
            title: item.title().to_owned().into(),
            body: div()
                .text_color(tokens.neutral.foreground)
                .child(item.text())
                .into_any_element(),
            footer: Vec::new(),
            size: DialogSize::Wide,
        },
        tokens,
        window,
        crate::areas::draft::compact(window),
        Shell::context_closed,
        cx,
    ))
}
