//! Legacy selected-resource adapter until the sole resident producer is adopted.
//! Captured draft keys are checked on every action; no global swap or second registry.

use vessel_app_kit::field::composer_layout::{ComposerFont, ComposerLayout, ComposerViewport};
use vessel_app_kit::ui::{App, Context, Window};
use vessel_app_kit::{SendOn, TextField};
use vessel_app_view::composer::drafts::DraftKey;
use vessel_app_view::composer_shell::surface::{
    Access, Extent, Layout, Mode, RenderedLines, Surface,
};
use vessel_app_view::prefs::SendKey;

use crate::{routes::Route, shell::Shell};

impl Shell {
    /// A window overlay must belong to the actual selected panel, including its
    /// top compact sheet. Legacy selected globals alone cannot prove visibility.
    pub(super) fn composer_fullscreen_target(&self) -> Option<DraftKey> {
        let target = self.composer_current_target()?;
        if let DraftKey::Session { session } = target {
            let context = self.layout_action_context()?;
            if context.target() != Some(&vessel_app_view::layout::Target::Agent(session)) {
                return None;
            }
        }
        Some(target)
    }

    pub(crate) fn composer_current_target(&self) -> Option<DraftKey> {
        match self.route {
            Route::Draft => Some(DraftKey::New),
            Route::Session => self
                .areas
                .session
                .open
                .as_ref()
                .filter(|open| open.structured)
                .map(|open| DraftKey::Session {
                    session: open.session,
                }),
            _ => None,
        }
    }

    pub(crate) fn composer_shell_field(&self, target: DraftKey) -> Option<&TextField> {
        if self.composer_current_target() != Some(target) {
            return None;
        }
        Some(match target {
            DraftKey::New => &self.areas.draft.composer,
            DraftKey::Session { .. } => &self.areas.session.composer,
        })
    }

    pub(crate) fn composer_surface(&self, target: DraftKey, cx: &App) -> Option<Surface> {
        let field = self.composer_shell_field(target)?;
        let mut surface = self.areas.composer_shell.surface(target);
        let editable = field.composer_is_editable(cx)
            && (target != DraftKey::New || self.areas.draft.creation.flow.may_submit());
        surface.set_access(if editable {
            Access::Editable
        } else {
            Access::ReadOnly
        });
        Some(surface)
    }

    pub(crate) fn composer_inline_layout(
        &self,
        target: DraftKey,
        touch: bool,
        window: &Window,
    ) -> ComposerLayout {
        let viewport = if touch {
            ComposerViewport::Compact
        } else {
            ComposerViewport::Desktop(window.viewport_size().height)
        };
        let send = match self.prefs.settings.composer.send {
            SendKey::Enter => SendOn::Enter,
            SendKey::ModEnter => SendOn::ModEnter,
        };
        ComposerLayout::inline(viewport, self.composer_font(target), send)
    }

    pub(crate) fn composer_font(&self, target: DraftKey) -> ComposerFont {
        match self.areas.composer_shell.surface(target).mode() {
            Mode::Chat => ComposerFont::Chat,
            Mode::Terminal => ComposerFont::Terminal,
        }
    }

    pub(crate) fn composer_expand(
        &mut self,
        target: DraftKey,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if self.composer_fullscreen_target() != Some(target) {
            return;
        }
        let Some(field) = self.composer_shell_field(target).cloned() else {
            return;
        };
        let Some(mut surface) = self.composer_surface(target, cx) else {
            return;
        };
        let Some(rows) = field.rendered_composer_rows(cx) else {
            return;
        };
        let layout = if super::super::draft::compact(window) {
            Layout::Compact
        } else {
            Layout::Desktop
        };
        surface.expand(layout, RenderedLines::new(rows.get()));
        *self.areas.composer_shell.surface_mut(target) = surface;
        if surface.extent() == Extent::Fullscreen {
            field.focus(window, cx);
            cx.notify();
        }
    }

    pub(crate) fn composer_collapse(
        &mut self,
        target: DraftKey,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if self.composer_fullscreen_target() != Some(target) {
            return;
        }
        let Some(field) = self.composer_shell_field(target).cloned() else {
            return;
        };
        self.areas.composer_shell.collapse(target);
        field.focus(window, cx);
        cx.notify();
    }
}
