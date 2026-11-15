//! One contribution's retained form values and captured, revalidated intentions.
//! This model never invokes a helper or supplies node/workspace authorization.
mod controls;
#[cfg(test)]
mod tests;

use super::{Problem, Registry};
use std::{collections::BTreeMap, fmt};
use vessel_proto::plugins::{
    Action, ContentText, ContributionAddress, ContributionContent, ContributionId, ControlContent,
    RpcName, Surface, ToggleState,
};

/// Retained edits share the declaration-size budget rather than multiplying
/// the per-text cap by every control in every mounted contribution.
pub const MAX_SURFACE_EDIT_BYTES: usize = vessel_proto::plugins::MAX_DOCUMENT_BYTES;

/// Text is bounded canonical plugin content, including an intentionally empty edit.
#[derive(Clone, PartialEq, Eq)]
pub enum FieldValue {
    /// Literal input, never interpreted as core commands or JSON.
    Text(ContentText),
    /// One of the declared stable option identities.
    Choice(Option<ContributionId>),
    /// Explicit switch state.
    Toggle(ToggleState),
}
impl fmt::Debug for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Text(_) => "Text(<private edit>)",
            Self::Choice(_) => "Choice(<selected identity>)",
            Self::Toggle(_) => "Toggle(<state>)",
        })
    }
}

impl FieldValue {
    fn bytes(&self) -> usize {
        match self {
            Self::Text(text) => text.as_str().len(),
            Self::Choice(selected) => selected.as_ref().map_or(0, |id| id.as_str().len()),
            Self::Toggle(_) => 1,
        }
    }
}

/// Local interaction refusal. Nodes still recheck caller, target and generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SurfaceProblem {
    /// Offline, replaced or missing exact contribution.
    #[error("plugin surface is unavailable")]
    Catalog(#[from] Problem),
    /// This contribution has no native surface.
    #[error("plugin contribution has no surface")]
    Kind,
    /// Field/button identity is absent from the checked graph.
    #[error("plugin control is unavailable")]
    Missing,
    /// An explicit hidden/disabled ancestor prevents activation.
    #[error("plugin control is disabled")]
    Disabled,
    /// Form type/option or requested menu path disagrees with declarations.
    #[error("plugin control value or action is invalid")]
    Value,
    /// Refuse atomically and retain the previous complete edit.
    #[error("plugin form edits exceed their aggregate byte budget")]
    Capacity,
}

/// Private construction binds display/form state to one immutable graph and run.
#[derive(Debug)]
pub struct SurfaceState {
    address: ContributionAddress,
    surface: Surface,
    fields: BTreeMap<ContributionId, FieldValue>,
    field_bytes: usize,
}

/// A client intention, not an authenticated grant or successful node operation.
/// The app revalidates it immediately before sending; the node rechecks again.
#[derive(Clone, PartialEq, Eq)]
pub struct SurfaceRequest {
    address: ContributionAddress,
    submission: Submission,
}
impl fmt::Debug for SurfaceRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SurfaceRequest")
            .field("address", &self.address)
            .finish_non_exhaustive()
    }
}

/// Typed rendering events; the future node adapter owns their wire encoding.
#[derive(Clone, PartialEq, Eq)]
pub enum Submission {
    /// An exact declared action, including bounded static plugin-owned input.
    Action(Action),
    /// A field's exact declared method and validated current value.
    Field {
        /// Stable control identity, independent from its translated label.
        control: ContributionId,
        /// Plugin callback name, never a core RPC name.
        method: RpcName,
        /// Current bounded literal value.
        value: FieldValue,
    },
}
impl fmt::Debug for Submission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Action(_) => "Action(<captured plugin data>)",
            Self::Field { .. } => "Field(<captured private value>)",
        })
    }
}
impl SurfaceRequest {
    /// Exact node/plugin/run and contribution namespace.
    #[must_use]
    pub const fn address(&self) -> &ContributionAddress {
        &self.address
    }
    /// Requested action/form data; never execute without fresh owner checks.
    #[must_use]
    pub const fn submission(&self) -> &Submission {
        &self.submission
    }
}

impl SurfaceState {
    /// Capture only a current, validated declaration. File/workspace placement
    /// and ownership remain their feature owners' separate checks.
    /// # Errors
    /// Refuses unavailable nodes/generations and non-surface contributions.
    pub fn new(registry: &Registry, address: ContributionAddress) -> Result<Self, SurfaceProblem> {
        let contribution = registry.resolve_contribution(&address)?;
        let surface = surface(&contribution.content).ok_or(SurfaceProblem::Kind)?;
        let empty: ContentText = "".parse().map_err(|_invalid| SurfaceProblem::Value)?;
        let mut fields = BTreeMap::new();
        controls::visit(surface, true, &mut |control, _enabled| {
            let value = match &control.content {
                ControlContent::TextField { .. } => Some(FieldValue::Text(empty.clone())),
                ControlContent::Choice { selected, .. } => {
                    Some(FieldValue::Choice(selected.clone()))
                }
                ControlContent::Toggle { state, .. } => Some(FieldValue::Toggle(*state)),
                ControlContent::Text { .. }
                | ControlContent::Markdown { .. }
                | ControlContent::Stack { .. }
                | ControlContent::Row { .. }
                | ControlContent::Section { .. }
                | ControlContent::Button { .. }
                | ControlContent::Status { .. } => None,
            };
            if let Some(value) = value {
                let _old = fields.insert(control.id.clone(), value);
            }
        });
        let field_bytes: usize = fields.values().map(FieldValue::bytes).sum();
        if field_bytes > MAX_SURFACE_EDIT_BYTES {
            return Err(SurfaceProblem::Capacity);
        }
        Ok(Self {
            address,
            surface: surface.clone(),
            fields,
            field_bytes,
        })
    }

    /// Immutable presentation; core owns every control renderer.
    #[must_use]
    pub const fn surface(&self) -> &Surface {
        &self.surface
    }
    /// Identity used to scope retained control state and element IDs.
    #[must_use]
    pub const fn address(&self) -> &ContributionAddress {
        &self.address
    }
    /// Retained user values may be displayed offline; they confer no authority.
    #[must_use]
    pub fn value(&self, control: &ContributionId) -> Option<&FieldValue> {
        self.fields.get(control)
    }

    /// Update a retained field without sending or running anything.
    /// # Errors
    /// Refuses unknown controls, mismatched value kinds and undeclared choices.
    pub fn edit(&mut self, id: &ContributionId, value: FieldValue) -> Result<(), SurfaceProblem> {
        let (control, enabled) = self.control(id)?;
        if !enabled {
            return Err(SurfaceProblem::Disabled);
        }
        let valid = match &control.content {
            ControlContent::TextField { .. } => matches!(&value, FieldValue::Text(_)),
            ControlContent::Toggle { .. } => matches!(&value, FieldValue::Toggle(_)),
            ControlContent::Choice { options, .. } => match &value {
                FieldValue::Choice(selected) => selected
                    .as_ref()
                    .is_none_or(|id| options.iter().any(|option| option.id == *id)),
                FieldValue::Text(_) | FieldValue::Toggle(_) => false,
            },
            ControlContent::Text { .. }
            | ControlContent::Markdown { .. }
            | ControlContent::Stack { .. }
            | ControlContent::Row { .. }
            | ControlContent::Section { .. }
            | ControlContent::Button { .. }
            | ControlContent::Status { .. } => false,
        };
        if !valid {
            return Err(SurfaceProblem::Value);
        }
        let target = self.fields.get_mut(id).ok_or(SurfaceProblem::Missing)?;
        let bytes = self
            .field_bytes
            .saturating_sub(target.bytes())
            .saturating_add(value.bytes());
        if bytes > MAX_SURFACE_EDIT_BYTES {
            return Err(SurfaceProblem::Capacity);
        }
        *target = value;
        self.field_bytes = bytes;
        Ok(())
    }

    /// Capture one form submit only from current facts.
    /// # Errors
    /// Refuses stale/offline graphs, disabled controls and non-fields.
    pub fn submit(
        &self,
        registry: &Registry,
        id: &ContributionId,
    ) -> Result<SurfaceRequest, SurfaceProblem> {
        self.current(registry)?;
        let (control, enabled) = self.control(id)?;
        if !enabled {
            return Err(SurfaceProblem::Disabled);
        }
        let method = match &control.content {
            ControlContent::TextField { submit, .. }
            | ControlContent::Choice { submit, .. }
            | ControlContent::Toggle { submit, .. } => submit.clone(),
            ControlContent::Text { .. }
            | ControlContent::Markdown { .. }
            | ControlContent::Stack { .. }
            | ControlContent::Row { .. }
            | ControlContent::Section { .. }
            | ControlContent::Button { .. }
            | ControlContent::Status { .. } => return Err(SurfaceProblem::Value),
        };
        let value = self.fields.get(id).ok_or(SurfaceProblem::Missing)?.clone();
        Ok(SurfaceRequest {
            address: self.address.clone(),
            submission: Submission::Field {
                control: id.clone(),
                method,
                value,
            },
        })
    }

    /// Capture a button action, walking exact menu identities and availability.
    /// # Errors
    /// A missing/disabled button/menu or a non-action leaf refuses.
    pub fn activate(
        &self,
        registry: &Registry,
        id: &ContributionId,
        menu_path: &[ContributionId],
    ) -> Result<SurfaceRequest, SurfaceProblem> {
        self.current(registry)?;
        let (control, enabled) = self.control(id)?;
        if !enabled {
            return Err(SurfaceProblem::Disabled);
        }
        let ControlContent::Button { button } = &control.content else {
            return Err(SurfaceProblem::Value);
        };
        let action = controls::action(button, menu_path)?.clone();
        Ok(SurfaceRequest {
            address: self.address.clone(),
            submission: Submission::Action(action),
        })
    }

    /// Recheck a retained intention after any queued UI work, without reusing a
    /// prior permission result or silently selecting another installation.
    /// # Errors
    /// Refuses changed graphs/runs or an intention from another surface.
    pub fn revalidate(
        &self,
        registry: &Registry,
        request: &SurfaceRequest,
    ) -> Result<(), SurfaceProblem> {
        self.current(registry)?;
        if request.address != self.address {
            return Err(SurfaceProblem::Catalog(Problem::Stale));
        }
        Ok(())
    }

    fn current(&self, registry: &Registry) -> Result<(), SurfaceProblem> {
        if surface(&registry.resolve_contribution(&self.address)?.content) != Some(&self.surface) {
            return Err(SurfaceProblem::Catalog(Problem::Stale));
        }
        Ok(())
    }

    fn control(
        &self,
        id: &ContributionId,
    ) -> Result<(&vessel_proto::plugins::Control, bool), SurfaceProblem> {
        let surface = self.surface();
        let mut found = None;
        controls::visit(surface, true, &mut |control, enabled| {
            if control.id == *id {
                found = Some((control, enabled));
            }
        });
        found.ok_or(SurfaceProblem::Missing)
    }
}

fn surface(content: &ContributionContent) -> Option<&Surface> {
    match content {
        ContributionContent::Screen { surface }
        | ContributionContent::SettingsScreen { surface }
        | ContributionContent::Panel { surface, .. }
        | ContributionContent::FilePanel { surface, .. }
        | ContributionContent::TimelineRenderer { surface, .. } => Some(surface),
        ContributionContent::Sidebar { .. }
        | ContributionContent::Command { .. }
        | ContributionContent::SlashCommand { .. }
        | ContributionContent::HeaderButton { .. }
        | ContributionContent::ComposerPill { .. }
        | ContributionContent::AttachmentSource { .. } => None,
    }
}
