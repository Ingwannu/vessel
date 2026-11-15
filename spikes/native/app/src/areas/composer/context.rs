//! All producers share this validated intake and the kit's attachment pills.
//! Source snapshots stay with their composer; replacing or suppressing one
//! does not mutate another panel's registry or fetch its URL during send.
use crate::shell::Shell;
use gpui::FocusHandle;
use vessel_app_kit::attachment::{Body, PillHandlers, Standing, attachment_pill, attachment_tray};
use vessel_app_kit::dialog::{DialogContent, DialogSize, DialogState, dialog};
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, SharedString, Styled as _, Window,
    div,
};
use vessel_app_kit::{Tokens, assets::Glyph};
use vessel_app_view::composer::{
    context::{ContextAttachment, ContextKind},
    drafts::DraftKey,
};

#[cfg(test)]
use vessel_app_view::composer::context::{ContextProblem, ContextScope};

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
    /// File/browser/review/forge/plugin owners adapt their canonical values
    /// here. A producer cannot redirect context into another workspace.
    #[cfg(test)]
    pub(crate) fn composer_attach_context(
        &mut self,
        target: DraftKey,
        scope: &ContextScope,
        item: ContextAttachment,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Result<(), ContextProblem> {
        if self.composer_context_scope(target).as_ref() != Some(scope) {
            return Err(ContextProblem::Scope);
        }
        if let ContextAttachment::WorkspaceFile(file) = &item {
            if &file.scope != scope {
                return Err(ContextProblem::Scope);
            }
        }
        self.areas.composer.contexts.tray(target).reattach(item)?;
        if let Some(field) = self.composer_field(target) {
            field.focus(window, cx);
        }
        cx.notify();
        Ok(())
    }
    /// The creation area seeds this only after claiming the active draft's canonical target.
    /// History has no destination root until that draft's placement is chosen.
    #[cfg(test)]
    pub(crate) fn composer_seed_history(
        &mut self,
        target: DraftKey,
        history: vessel_app_view::composer::context::ChatHistory,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Result<(), ContextProblem> {
        if target != DraftKey::New {
            return Err(ContextProblem::Scope);
        }
        self.areas
            .composer
            .contexts
            .tray(target)
            .reattach(history.into_attachment())?;
        if let Some(field) = self.composer_field(target) {
            field.focus(window, cx);
        }
        cx.notify();
        Ok(())
    }
    pub(super) fn composer_has_context(&self, target: DraftKey) -> bool {
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
        {
            if let Some(url) = &payload.url {
                cx.open_url(url.as_str());
                return;
            }
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
    fn draft_context_removed(
        &mut self,
        index: usize,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.context_removed(DraftKey::New, index, cx);
    }
    fn session_context_removed(
        &mut self,
        index: usize,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(target) = self.session_tray_key() {
            self.context_removed(target, index, cx);
        }
    }
    fn draft_context_opened(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.context_opened(DraftKey::New, index, window, cx);
    }
    fn session_context_opened(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(target) = self.session_tray_key() {
            self.context_opened(target, index, window, cx);
        }
    }
}

pub(super) fn tray(
    shell: &Shell,
    target: DraftKey,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let tray = shell.areas.composer.contexts.get(target)?;
    if tray.is_empty() {
        return None;
    }
    let (id, open, remove): (
        &str,
        vessel_app_kit::attachment::PillClick<Shell>,
        vessel_app_kit::attachment::PillClick<Shell>,
    ) = match target {
        DraftKey::New => (
            "draft-context",
            Shell::draft_context_opened,
            Shell::draft_context_removed,
        ),
        DraftKey::Session { .. } => (
            "session-context",
            Shell::session_context_opened,
            Shell::session_context_removed,
        ),
    };
    let pills = tray.visible().enumerate().map(|(index, item)| {
        let handlers = PillHandlers {
            on_open: (!matches!(item, ContextAttachment::WorkspaceFile(_))).then_some(open),
            on_remove: remove,
        };
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
        attachment_pill(
            id,
            index,
            &Body::Label {
                icon,
                title: SharedString::from(item.title().to_owned()),
                subtitle: subtitle.into(),
            },
            Standing::Quiet,
            &handlers,
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
