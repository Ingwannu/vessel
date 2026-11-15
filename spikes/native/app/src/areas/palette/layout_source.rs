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
const COMMANDS: &[(&str, Action, i16, Capability, Glyph)] = &[
    (
        "tab:chooser",
        Action::NewTab,
        0,
        Capability::Any,
        Glyph::Plus,
    ),
    (
        "tab:previous",
        Action::PreviousTab,
        30,
        Capability::Any,
        Glyph::ChevronLeft,
    ),
    (
        "tab:next",
        Action::NextTab,
        31,
        Capability::Any,
        Glyph::ChevronRight,
    ),
    (
        "tab:close-current",
        Action::CloseTab,
        32,
        Capability::Any,
        Glyph::Close,
    ),
    (
        "pane:split-right",
        Action::SplitRight,
        50,
        Capability::Split,
        Glyph::Split,
    ),
    (
        "pane:split-down",
        Action::SplitDown,
        51,
        Capability::Split,
        Glyph::Split,
    ),
    (
        "pane:focus-left",
        Action::FocusLeft,
        52,
        Capability::Split,
        Glyph::ChevronLeft,
    ),
    (
        "pane:focus-right",
        Action::FocusRight,
        53,
        Capability::Split,
        Glyph::ChevronRight,
    ),
    (
        "pane:focus-up",
        Action::FocusUp,
        54,
        Capability::Split,
        Glyph::ChevronUp,
    ),
    (
        "pane:focus-down",
        Action::FocusDown,
        55,
        Capability::Split,
        Glyph::ChevronDown,
    ),
    (
        "pane:move-tab-left",
        Action::MoveTabLeft,
        56,
        Capability::Split,
        Glyph::ChevronLeft,
    ),
    (
        "pane:move-tab-right",
        Action::MoveTabRight,
        57,
        Capability::Split,
        Glyph::ChevronRight,
    ),
    (
        "pane:move-tab-up",
        Action::MoveTabUp,
        58,
        Capability::Split,
        Glyph::ChevronUp,
    ),
    (
        "pane:move-tab-down",
        Action::MoveTabDown,
        59,
        Capability::Split,
        Glyph::ChevronDown,
    ),
    (
        "pane:close",
        Action::ClosePane,
        60,
        Capability::Split,
        Glyph::Close,
    ),
    (
        "workspace:toggle-focus-mode",
        Action::ToggleFocus,
        26,
        Capability::Single,
        Glyph::Maximize,
    ),
    (
        "pane:focus-mode-toggle",
        Action::ToggleFocus,
        62,
        Capability::Split,
        Glyph::Maximize,
    ),
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
                .map(|(_, action, _, _, _)| action_label(*action, shell.words).to_owned())
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
        .filter(|((_, _, _, capability, _), _)| capability.allows(signature.split))
        .filter_map(|(spec, label)| {
            spec.1
                .layout_action(None)
                .map(|operation| (spec, label, operation))
        })
        .map(|((id, action, rank, _, glyph), label, operation)| {
            let context = signature.context.clone();
            Ok(Contribution {
                id: id.parse()?,
                group: "workspace".parse()?,
                group_rank: Rank::WORKSPACE,
                section_title: None,
                rank: Rank::new(*rank),
                keywords: vec![(*id).to_owned()],
                visibility: Visibility::Query,
                presentation: Presentation::Action {
                    title: label.clone(),
                    subtitle: None,
                },
                target: Command::callback(Some(*glyph), Some(*action), move |shell, window, cx| {
                    shell.run_layout_action_at(&context, operation, window, cx)
                }),
            })
        })
        .collect()
}
