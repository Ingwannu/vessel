//! Native contribution shapes: data only, with no program, URL or paint escape.

use serde::{Deserialize, Serialize};

use super::identity::{ContentText, FileExtension, FileNamePattern};
use super::{Caption, ContributionId, ContributionKind, PanelLocation, RpcName};
use crate::{RawJson, SessionId};

/// A contribution keeps its identity when translated labels change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct Contribution {
    /// Local identity within this contribution kind.
    pub id: ContributionId,
    /// Accessible label, resolved by the plugin's translation data.
    pub title: Caption,
    /// The core-owned place and rendering contract.
    pub content: ContributionContent,
}

/// Every client extension is explicit; unknown executable client kinds fail decode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ContributionContent {
    /// A host-scoped navigable plugin surface.
    Screen {
        /// Content rendered by the kit.
        surface: Surface,
    },
    /// A screen inserted under this host's settings.
    SettingsScreen {
        /// Content rendered by the kit.
        surface: Surface,
    },
    /// A panel bound to a workspace or one of its agents.
    Panel {
        /// Required runtime context.
        context: PanelContext,
        /// Supported docking locations.
        locations: Vec<PanelLocation>,
        /// Content rendered by the kit.
        surface: Surface,
    },
    /// A core file tab selects this contribution using literal extension/name matches.
    FilePanel {
        /// Literal case-insensitive extensions, without dots.
        extensions: Vec<FileExtension>,
        /// Literal complete names.
        filenames: Vec<FileNamePattern>,
        /// Content rendered by the kit.
        surface: Surface,
    },
    /// A header/footer entry opens only a declared screen in its own installation.
    Sidebar {
        /// Position in the shared sidebar.
        placement: SidebarPlacement,
        /// Same-installation screen to open.
        screen: ContributionId,
    },
    /// A command-center action; core owns filtering, keyboard and context.
    Command {
        /// Search keywords are display text, not executable matching rules.
        keywords: Vec<Caption>,
        /// Where this command may appear.
        context: CommandContext,
        /// The bounded declared action.
        action: Action,
    },
    /// A slash command routes submitted arguments to a declared method.
    SlashCommand {
        /// Literal command name, separate from contribution identity.
        name: crate::conversation::CommandName,
        /// Help text displayed by the composer.
        description: Caption,
        /// Optional argument help, displayed as text.
        argument_hint: Option<Caption>,
        /// Where the command may be submitted.
        context: PanelContext,
        /// Handler registered in this helper generation.
        method: RpcName,
    },
    /// A button belongs to one canonical workspace, never an inferred cwd.
    HeaderButton {
        /// Shared W11 identity; the core rechecks current registry ownership.
        workspace: crate::WorkspaceId,
        /// Data-driven button behavior.
        button: Button,
    },
    /// A pill owned by the specified agent composer.
    ComposerPill {
        /// Shared W11 identity, checked against the target session's owner.
        workspace: crate::WorkspaceId,
        /// Target agent identity.
        agent: SessionId,
        /// Data-driven button behavior.
        button: Button,
    },
    /// An attachment picker uses the helper's registered search contract.
    AttachmentSource {
        /// Handler produces core attachment items.
        search: RpcName,
    },
    /// Timeline extensions are presentation data for an explicitly named kind.
    TimelineRenderer {
        /// The plugin's own item kind, separate from core kinds.
        item_kind: ContributionId,
        /// Declarative presentation.
        surface: Surface,
    },
}

impl ContributionContent {
    /// The namespace used to reject duplicates without conflating different slots.
    #[must_use]
    pub const fn kind(&self) -> ContributionKind {
        match self {
            Self::Screen { .. } => ContributionKind::Screen,
            Self::SettingsScreen { .. } => ContributionKind::SettingsScreen,
            Self::Panel { .. } => ContributionKind::Panel,
            Self::FilePanel { .. } => ContributionKind::FilePanel,
            Self::Sidebar { placement, .. } => match placement {
                SidebarPlacement::Header => ContributionKind::SidebarHeader,
                SidebarPlacement::Footer => ContributionKind::SidebarFooter,
            },
            Self::Command { .. } => ContributionKind::Command,
            Self::SlashCommand { .. } => ContributionKind::SlashCommand,
            Self::HeaderButton { .. } => ContributionKind::HeaderButton,
            Self::ComposerPill { .. } => ContributionKind::ComposerPill,
            Self::AttachmentSource { .. } => ContributionKind::AttachmentSource,
            Self::TimelineRenderer { .. } => ContributionKind::TimelineRenderer,
        }
    }
}

/// Context availability is checked again when dispatching the captured action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum CommandContext {
    /// Does not require a selected workspace or agent.
    Global,
    /// Requires a selected workspace.
    Workspace,
    /// Requires a selected agent.
    Agent,
}

/// Panels and slash commands always belong to a workspace or an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum PanelContext {
    /// A workspace-owned contribution.
    Workspace,
    /// An agent-owned contribution.
    Agent,
}

/// Sidebar content remains in its existing core layout band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SidebarPlacement {
    /// Above the workspace tree.
    Header,
    /// Below the workspace tree.
    Footer,
}

/// A recursive core-control tree. `ClientDocument` validates depth, total nodes,
/// local identities and action references before any consumer receives it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct Surface {
    /// Ordered content; core chooses compact wrapping and token spacing.
    pub controls: Vec<Control>,
}

/// Each control has an identity for retained focus and captured edit state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct Control {
    /// Unique within the contribution surface, including nested containers.
    pub id: ContributionId,
    /// An existing kit control's declarative inputs.
    pub content: ControlContent,
}

/// The native kit controls available in contract version one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ControlContent {
    /// A paragraph; the kit owns fonts and colors.
    Text {
        /// Text never evaluated as code or markup.
        text: ContentText,
    },
    /// Existing kit Markdown; external resources and links retain core policy.
    Markdown {
        /// Content is display data, never executable plugin code.
        text: ContentText,
    },
    /// A vertical container.
    Stack {
        /// Ordered children.
        surface: Surface,
    },
    /// A wrapping row.
    Row {
        /// Ordered children.
        surface: Surface,
    },
    /// A labelled group using core section spacing.
    Section {
        /// Group label.
        title: Caption,
        /// Ordered content.
        surface: Surface,
    },
    /// A core button with bounded action/menu/popover behavior.
    Button {
        /// Label and explicit availability.
        button: Button,
    },
    /// A retained edit; submitting invokes the registered method with field values.
    TextField {
        /// Accessible field label.
        label: Caption,
        /// Edit submission contract.
        submit: RpcName,
    },
    /// A finite choice; core owns focus, selection and compact layout.
    Choice {
        /// Accessible field label.
        label: Caption,
        /// Unique option values.
        options: Vec<ChoiceOption>,
        /// Retained selected identity, if any.
        selected: Option<ContributionId>,
        /// Selection submission contract.
        submit: RpcName,
    },
    /// A core switch; its state is explicit at every call site.
    Toggle {
        /// Accessible field label.
        label: Caption,
        /// Current state.
        state: ToggleState,
        /// Change submission contract.
        submit: RpcName,
    },
    /// A core status indicator using semantic tone, never arbitrary colors.
    Status {
        /// Accessible explanation.
        label: Caption,
        /// Core semantic tone.
        tone: StatusTone,
    },
}

/// A choice option carries a stable value independent of translated labels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct ChoiceOption {
    /// Stable submitted value.
    pub id: ContributionId,
    /// Accessible display label.
    pub title: Caption,
}

/// Avoids ambiguous boolean state in contribution contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ToggleState {
    /// Switch enabled.
    On,
    /// Switch disabled.
    Off,
}

/// Status colors come from the active core theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum StatusTone {
    /// Informational.
    Neutral,
    /// Successful or healthy.
    Success,
    /// Needs attention.
    Warning,
    /// Failed or refused.
    Danger,
}

/// Hidden and disabled differ: hidden removes a control; disabled keeps its label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    /// May be activated with its captured host/generation.
    Enabled,
    /// Visible but cannot dispatch.
    Disabled,
    /// Excluded from paint and keyboard navigation.
    Hidden,
}

/// Shared header, composer and surface button data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct Button {
    /// Accessible label.
    pub title: Caption,
    /// Explicit visibility and interaction state.
    pub availability: Availability,
    /// Core-owned interaction kind.
    pub behavior: ButtonBehavior,
}

/// Menus and popovers never contain callbacks or an embedded second UI runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ButtonBehavior {
    /// One captured action.
    Action {
        /// Validated action.
        action: Action,
    },
    /// Core menu semantics, keyboard handling and compact sheet.
    Menu {
        /// Ordered menu entries.
        items: Vec<MenuEntry>,
    },
    /// Core popover semantics and compact sheet.
    Popover {
        /// Bounded core-control tree.
        surface: Surface,
    },
}

/// Menus retain item identities and nested behavior without executable callbacks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MenuEntry {
    /// A separator has identity so dynamic presentation updates do not shift focus.
    Separator {
        /// Unique within the parent menu.
        id: ContributionId,
    },
    /// Item presentation and interaction state are explicit.
    Item {
        /// Unique within the parent menu.
        id: ContributionId,
        /// Accessible label.
        title: Caption,
        /// Explicit interaction state.
        availability: Availability,
        /// Action, nested menu or core popover.
        behavior: ButtonBehavior,
    },
}

/// Every action stays inside the selected plugin's node and runtime generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    /// Open a screen from this same declaration document.
    OpenScreen {
        /// Referenced declared screen id.
        screen: ContributionId,
    },
    /// Invoke a registered method. The node applies caller and plugin policy.
    Invoke {
        /// Plugin method, not an unrestricted node method.
        method: RpcName,
        /// Opaque plugin-owned payload, never inspected by core.
        input: RawJson,
    },
}
