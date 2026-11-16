//! Pane commands capture the owner's presentation fence. The command center
//! never mutates a layout or resolves a workspace from paths/session guesses.
use super::source::Command;
use crate::shell::Shell;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::Window;
use vessel_app_view::layout::{LayoutActionContext, LayoutMode};
use vessel_app_view::palette::registration::{
    Contribution, Owner, Presentation, Rank, RegistrationError, Visibility,
};
use vessel_app_view::shortcuts::{Action, action_label};

const SOURCE: &str = "workspace-layout";
#[derive(Clone, Copy)]
enum Capability {
    Any,
    Split,
    Single,
}
impl Capability {
    const fn allows(self, split: bool) -> bool {
        match self {
            Self::Any => true,
            Self::Split => split,
            Self::Single => !split,
        }
    }
}
// This is command-center presentation, never a second keyboard/default table.
struct Spec {
    id: &'static str,
    action: Action,
    rank: i16,
    capability: Capability,
    glyph: Glyph,
    keywords: &'static [&'static str],
}

const COMMANDS: &[Spec] = &[
    Spec {
        id: "tab:chooser",
        action: Action::NewTab,
        rank: 0,
        capability: Capability::Any,
        glyph: Glyph::Plus,
        keywords: &["tab", "new", "chooser"],
    },
    Spec {
        id: "tab:previous",
        action: Action::PreviousTab,
        rank: 30,
        capability: Capability::Any,
        glyph: Glyph::ArrowLeft,
        keywords: &["tab", "previous", "switch", "navigate"],
    },
    Spec {
        id: "tab:next",
        action: Action::NextTab,
        rank: 31,
        capability: Capability::Any,
        glyph: Glyph::ChevronRight,
        keywords: &["tab", "next", "switch", "navigate"],
    },
    Spec {
        id: "tab:close-current",
        action: Action::CloseTab,
        rank: 32,
        capability: Capability::Any,
        glyph: Glyph::X,
        keywords: &["tab", "close", "current"],
    },
    Spec {
        id: "pane:split-right",
        action: Action::SplitRight,
        rank: 50,
        capability: Capability::Split,
        glyph: Glyph::Columns2,
        keywords: &["split", "pane", "vertical"],
    },
    Spec {
        id: "pane:split-down",
        action: Action::SplitDown,
        rank: 51,
        capability: Capability::Split,
        glyph: Glyph::Rows2,
        keywords: &["split", "pane", "horizontal"],
    },
    Spec {
        id: "pane:focus-left",
        action: Action::FocusLeft,
        rank: 52,
        capability: Capability::Split,
        glyph: Glyph::ChevronLeft,
        keywords: &["pane", "focus", "left"],
    },
    Spec {
        id: "pane:focus-right",
        action: Action::FocusRight,
        rank: 53,
        capability: Capability::Split,
        glyph: Glyph::ChevronRight,
        keywords: &["pane", "focus", "right"],
    },
    Spec {
        id: "pane:focus-up",
        action: Action::FocusUp,
        rank: 54,
        capability: Capability::Split,
        glyph: Glyph::ChevronUp,
        keywords: &["pane", "focus", "up"],
    },
    Spec {
        id: "pane:focus-down",
        action: Action::FocusDown,
        rank: 55,
        capability: Capability::Split,
        glyph: Glyph::ChevronDown,
        keywords: &["pane", "focus", "down"],
    },
    Spec {
        id: "pane:move-tab-left",
        action: Action::MoveTabLeft,
        rank: 56,
        capability: Capability::Split,
        glyph: Glyph::ChevronLeft,
        keywords: &["pane", "tab", "move", "left"],
    },
    Spec {
        id: "pane:move-tab-right",
        action: Action::MoveTabRight,
        rank: 57,
        capability: Capability::Split,
        glyph: Glyph::ChevronRight,
        keywords: &["pane", "tab", "move", "right"],
    },
    Spec {
        id: "pane:move-tab-up",
        action: Action::MoveTabUp,
        rank: 58,
        capability: Capability::Split,
        glyph: Glyph::ChevronUp,
        keywords: &["pane", "tab", "move", "up"],
    },
    Spec {
        id: "pane:move-tab-down",
        action: Action::MoveTabDown,
        rank: 59,
        capability: Capability::Split,
        glyph: Glyph::ChevronDown,
        keywords: &["pane", "tab", "move", "down"],
    },
    Spec {
        id: "pane:close",
        action: Action::ClosePane,
        rank: 60,
        capability: Capability::Split,
        glyph: Glyph::X,
        keywords: &["pane", "close"],
    },
    Spec {
        id: "workspace:toggle-focus-mode",
        action: Action::ToggleFocus,
        rank: 26,
        capability: Capability::Single,
        glyph: Glyph::Maximize,
        keywords: &["toggle", "focus", "zen", "distraction", "fullscreen"],
    },
    Spec {
        id: "pane:focus-mode-toggle",
        action: Action::ToggleFocus,
        rank: 62,
        capability: Capability::Split,
        glyph: Glyph::Maximize,
        keywords: &["pane", "focus", "mode", "toggle"],
    },
];
#[derive(PartialEq, Eq)]
struct Signature {
    context: LayoutActionContext,
    split: bool,
    labels: Vec<String>,
}
#[derive(Default)]
pub(super) struct State {
    signature: Option<Signature>,
    owner: Option<Owner>,
}

/// No I/O or registration churn for an unchanged owner projection.
pub(super) fn refresh(shell: &mut Shell, window: &Window) {
    let next = if shell.areas.palette.is_open() {
        shell.layout_action_context().map(|context| Signature {
            context,
            split: crate::areas::layout::mode(window) == LayoutMode::SplitPanes,
            labels: COMMANDS
                .iter()
                .map(|spec| action_label(spec.action, shell.words).to_owned())
                .collect(),
        })
    } else {
        None
    };
    if shell.areas.palette.layout.signature == next {
        return;
    }
    let owner = match next
        .as_ref()
        .map(contributions)
        .transpose()
        .and_then(|entries| {
            entries
                .map(|entries| {
                    SOURCE
                        .parse()
                        .and_then(|id| shell.areas.palette.sources.replace(id, entries))
                })
                .transpose()
        }) {
        Ok(owner) => owner,
        Err(error) => {
            tracing::warn!(%error, "layout command source refused");
            return;
        }
    };
    if let Some(old) = shell.areas.palette.layout.owner.take() {
        shell.areas.palette.sources.remove(&old);
    }
    shell.areas.palette.layout.owner = owner;
    shell.areas.palette.layout.signature = next;
}

pub(super) fn close(shell: &mut Shell) {
    if let Some(owner) = shell.areas.palette.layout.owner.take() {
        shell.areas.palette.sources.remove(&owner);
    }
    shell.areas.palette.layout.signature = None;
}
fn contributions(signature: &Signature) -> Result<Vec<Contribution<Command>>, RegistrationError> {
    COMMANDS
        .iter()
        .zip(&signature.labels)
        .filter(|(spec, _)| spec.capability.allows(signature.split))
        .filter_map(|(spec, label)| {
            spec.action
                .layout_action(None)
                .map(|operation| (spec, label, operation))
        })
        .map(|(spec, label, operation)| {
            let context = signature.context.clone();
            Ok(Contribution {
                id: spec.id.parse()?,
                group: "workspace".parse()?,
                group_rank: Rank::WORKSPACE,
                section_title: None,
                rank: Rank::new(spec.rank),
                keywords: spec
                    .keywords
                    .iter()
                    .map(|word| (*word).to_owned())
                    .collect(),
                visibility: Visibility::Query,
                presentation: Presentation::Action {
                    title: label.clone(),
                    subtitle: None,
                },
                target: Command::callback(
                    Some(spec.glyph),
                    Some(spec.action),
                    move |shell, window, cx| {
                        shell.run_layout_action_at(&context, operation, window, cx)
                    },
                ),
            })
        })
        .collect()
}
