//! Workspace draft content is keyed before layout opens; payload ownership stays W6's.
//! Presentation/focus belongs to W12's (LayoutKey, TabId), never this content map.
#[cfg(test)]
mod tests;

use super::create::CreationState;
use crate::{
    areas::layout::registry::{PanelContext, PanelFocus},
    shell::Shell,
};
use gpui::AppContext as _;
use std::collections::BTreeMap;
use vessel_app_kit::ui::{
    AnyElement, Context, Entity, IntoElement as _, ParentElement as _, Styled as _, WeakEntity,
    Window, div,
};
use vessel_app_kit::{TextField, Tokens, Type, TypeExt as _};
use vessel_app_view::composer::{
    context::ContextScope,
    drafts::{DraftKey, Drafts},
};
use vessel_app_view::composer_shell::surface::Mode;
use vessel_app_view::creation::{DraftDestination, DraftSeed, Launch, SeedPlace, SeedProblem};
use vessel_app_view::layout::{DraftId, LayoutKey, Target};

#[derive(Default)]
pub(super) struct WorkspaceDrafts {
    entries: BTreeMap<(LayoutKey, DraftId), WorkspaceDraft>,
}
struct WorkspaceDraft {
    destination: DraftDestination,
    launch: Launch,
    composer: TextField,
    // Keeps the one field callback owner alive; the field below shares its engine.
    _editor: Entity<Editor>,
    creation: CreationState,
    // Only an authoritative WorkspaceLookup may supply this; no cwd fallback.
    scope: Option<ContextScope>,
    error: Option<String>,
}
/// Callback ownership carries this exact draft address through the kit's engine.
struct Editor {
    workspace: LayoutKey,
    draft: DraftId,
    shell: WeakEntity<Shell>,
    field: TextField,
}
impl Editor {
    fn edited(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        let LayoutKey::Workspace { node, workspace } = &self.workspace else {
            return;
        };
        let target = DraftKey::Workspace {
            node: *node,
            workspace: *workspace,
            draft: self.draft,
        };
        let text = self.field.text(cx);
        let _closed = self.shell.update(cx, |shell, cx| {
            shell.composer_keep_workspace_text(target, &text, cx);
        });
    }
    fn submitted(&mut self, text: String, _window: &mut Window, cx: &mut Context<'_, Self>) {
        let _closed = self.shell.update(cx, |shell, cx| {
            shell.submit_workspace_draft(&self.workspace, self.draft, text, cx);
        });
    }
}
impl Shell {
    /// Display policy follows this retained launch selection, never a global agent.
    pub(crate) fn workspace_draft_mode(
        &self,
        workspace: &LayoutKey,
        draft: DraftId,
    ) -> Option<Mode> {
        self.areas
            .draft
            .workspace_drafts
            .entries
            .get(&(workspace.clone(), draft))
            .map(|entry| match entry.launch {
                Launch::Chat => Mode::Chat,
                Launch::BlankTerminal | Launch::Profile(_) => Mode::Terminal,
            })
    }
    /// W6 resolves exactly this field; an absent draft is never the global New field.
    pub(crate) fn workspace_draft_field(
        &self,
        workspace: &LayoutKey,
        draft: DraftId,
    ) -> Option<&TextField> {
        self.areas
            .draft
            .workspace_drafts
            .entries
            .get(&(workspace.clone(), draft))
            .map(|entry| &entry.composer)
    }
    /// None remains explicit until the registry can resolve an authorized file root.
    pub(crate) fn workspace_draft_context_scope(
        &self,
        workspace: &LayoutKey,
        draft: DraftId,
    ) -> Option<ContextScope> {
        self.areas
            .draft
            .workspace_drafts
            .entries
            .get(&(workspace.clone(), draft))
            .and_then(|entry| entry.scope.clone())
    }
    /// Stage selections/editor before layout_open_target; a failed open retains it.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "W2 workspace seed action waits for the keyed W6 payload bridge"
        )
    )]
    pub(crate) fn seed_draft(
        &mut self,
        draft: DraftId,
        seed: DraftSeed,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Result<(), SeedProblem> {
        let SeedPlace::ExistingWorkspace(workspace) = seed.place else {
            return Err(SeedProblem::TargetMismatch);
        };
        let key = LayoutKey::Workspace {
            node: seed.destination.node,
            workspace,
        };
        let source = DraftKey::Workspace {
            node: seed.destination.node,
            workspace,
            draft,
        };
        let entries = &self.areas.draft.workspace_drafts.entries;
        if entries.contains_key(&(key.clone(), draft)) {
            return Err(SeedProblem::Busy);
        }
        if entries.len() >= Drafts::MAX_DRAFTS {
            return Err(SeedProblem::Context(
                vessel_app_view::composer::context::ContextProblem::Capacity,
            ));
        }
        if !self.fleet.machines.iter().any(|machine| {
            machine.name == seed.destination.machine.as_str()
                && machine.node == Some(seed.destination.node)
        }) {
            return Err(SeedProblem::DestinationChanged);
        }
        let owner = cx.entity().downgrade();
        let target = key.clone();
        // The kit deliberately takes a function pointer. Put the captured
        // address in its model owner, rather than inferring a global field.
        let editor = cx.new(|cx| {
            let field = TextField::composer("", Editor::submitted, window, cx);
            field.on_edit(Editor::edited, window, cx);
            Editor {
                workspace: target,
                draft,
                shell: owner,
                field,
            }
        });
        let composer = editor.read(cx).field.clone();
        let _previous = self.areas.draft.workspace_drafts.entries.insert(
            (key.clone(), draft),
            WorkspaceDraft {
                destination: seed.destination,
                launch: seed.launch,
                composer,
                _editor: editor,
                creation: CreationState::default(),
                scope: None,
                error: None,
            },
        );
        // History intake requires the admitted exact editor to be empty. Only
        // this new entry is rolled back on refusal; other drafts stay intact.
        if let Some(history) = seed.history {
            if let Err(problem) = self.composer_seed_history(source, history, window, cx) {
                drop(
                    self.areas
                        .draft
                        .workspace_drafts
                        .entries
                        .remove(&(key, draft)),
                );
                return Err(problem.into());
            }
        }
        if let Some(field) = self.workspace_draft_field(&key, draft) {
            field.set_text(seed.text.as_str(), window, cx);
        }
        // Programmatic writes do not emit the field's edit event. Keep this
        // accepted seed through the same bounded policy as subsequent typing.
        self.composer_keep_workspace_text(source, seed.text.as_str(), cx);
        cx.notify();
        Ok(())
    }
    fn submit_workspace_draft(
        &mut self,
        workspace: &LayoutKey,
        draft: DraftId,
        _text: String,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(entry) = self
            .areas
            .draft
            .workspace_drafts
            .entries
            .get_mut(&(workspace.clone(), draft))
        {
            // No legacy new-session fallback can discard the explicit workspace.
            entry.error = Some(self.words.creation().unsupported().to_owned());
            cx.notify();
        }
    }
}

pub(crate) fn render_panel(
    shell: &Shell,
    panel: &PanelContext<'_>,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let Target::Draft(draft) = panel.target else {
        return unavailable(shell);
    };
    let Some(entry) = shell
        .areas
        .draft
        .workspace_drafts
        .entries
        .get(&(panel.workspace.clone(), *draft))
    else {
        return unavailable(shell);
    };
    let Some(field) = shell.workspace_draft_field(panel.workspace, *draft) else {
        return unavailable(shell);
    };
    let LayoutKey::Workspace { node, workspace } = panel.workspace else {
        return unavailable(shell);
    };
    let source = DraftKey::Workspace {
        node: *node,
        workspace: *workspace,
        draft: *draft,
    };
    let editable = panel.focus == PanelFocus::Focused
        && entry.creation.flow.may_submit()
        && !shell.composer_attachments_blocked(source);
    field.set_editability(
        if editable {
            vessel_app_kit::Editability::Editable
        } else {
            vessel_app_kit::Editability::ReadOnly
        },
        cx,
    );
    let Some(mode) = shell.workspace_draft_mode(panel.workspace, *draft) else {
        return unavailable(shell);
    };
    let editor = match mode {
        Mode::Chat => {
            let Some(card) =
                crate::areas::composer::workspace_card(shell, source, editable, tokens, window, cx)
            else {
                return unavailable(shell);
            };
            card.into_any_element()
        }
        // Terminal input uses this retained field without agent affordances.
        Mode::Terminal => field
            .element(tokens, vessel_app_kit::breakpoint::phone(window), cx)
            .into_any_element(),
    };
    div()
        .size_full()
        .flex()
        .flex_col()
        .type_(Type::Body)
        .child(entry.destination.machine.as_str().to_owned())
        .child(
            match &entry.launch {
                Launch::Chat => shell.words.creation().launch_chat(),
                Launch::BlankTerminal => shell.words.creation().launch_shell(),
                Launch::Profile(_) => shell.words.creation().unsupported(),
            }
            .to_owned(),
        )
        .child(editor)
        .children(entry.error.iter().cloned().map(|error| div().child(error)))
        .into_any_element()
}
fn unavailable(shell: &Shell) -> AnyElement {
    div()
        .child(shell.words.layout().panel_unavailable().to_owned())
        .into_any_element()
}
