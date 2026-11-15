//! Walk only already validated finite control graphs, carrying availability
//! through popover/menu ancestors so hidden nested fields cannot dispatch.
use super::SurfaceProblem;
use vessel_proto::plugins::{
    Action, Availability, Button, ButtonBehavior, Caption, ContributionId, Control, ControlContent,
    MenuEntry, Surface,
};

pub(super) fn visit<'a>(
    surface: &'a Surface,
    enabled: bool,
    visitor: &mut impl FnMut(&'a Control, bool),
) {
    for control in &surface.controls {
        visitor(control, enabled);
        match &control.content {
            ControlContent::Stack { surface }
            | ControlContent::Row { surface }
            | ControlContent::Section { surface, .. } => visit(surface, enabled, visitor),
            ControlContent::Button { button } => visit_behavior(
                &button.behavior,
                enabled && button.availability == Availability::Enabled,
                visitor,
            ),
            ControlContent::Text { .. }
            | ControlContent::Markdown { .. }
            | ControlContent::TextField { .. }
            | ControlContent::Choice { .. }
            | ControlContent::Toggle { .. }
            | ControlContent::Status { .. } => {}
        }
    }
}

fn visit_behavior<'a>(
    behavior: &'a ButtonBehavior,
    enabled: bool,
    visitor: &mut impl FnMut(&'a Control, bool),
) {
    match behavior {
        ButtonBehavior::Action { .. } => {}
        ButtonBehavior::Popover { surface } => visit(surface, enabled, visitor),
        ButtonBehavior::Menu { items } => {
            for item in items {
                match item {
                    MenuEntry::Separator { .. } => {}
                    MenuEntry::Item {
                        availability,
                        behavior,
                        ..
                    } => visit_behavior(
                        behavior,
                        enabled && *availability == Availability::Enabled,
                        visitor,
                    ),
                }
            }
        }
    }
}

pub(super) fn action<'a>(
    button: &'a Button,
    path: &[ContributionId],
) -> Result<&'a Action, SurfaceProblem> {
    match behavior(button, path)?.1 {
        ButtonBehavior::Action { action } => Ok(action),
        ButtonBehavior::Menu { .. } | ButtonBehavior::Popover { .. } => Err(SurfaceProblem::Value),
    }
}

pub(super) fn behavior<'a>(
    button: &'a Button,
    path: &[ContributionId],
) -> Result<(&'a Caption, &'a ButtonBehavior), SurfaceProblem> {
    if button.availability != Availability::Enabled {
        return Err(SurfaceProblem::Disabled);
    }
    let mut behavior = &button.behavior;
    let mut title = &button.title;
    for identity in path {
        let ButtonBehavior::Menu { items } = behavior else {
            return Err(SurfaceProblem::Value);
        };
        let item = items
            .iter()
            .find(|item| match item {
                MenuEntry::Item { id, .. } | MenuEntry::Separator { id } => id == identity,
            })
            .ok_or(SurfaceProblem::Missing)?;
        match item {
            MenuEntry::Separator { .. } => return Err(SurfaceProblem::Value),
            MenuEntry::Item {
                availability,
                behavior: selected,
                title: selected_title,
            } => {
                if *availability != Availability::Enabled {
                    return Err(SurfaceProblem::Disabled);
                }
                behavior = selected;
                title = selected_title;
            }
        }
    }
    Ok((title, behavior))
}
