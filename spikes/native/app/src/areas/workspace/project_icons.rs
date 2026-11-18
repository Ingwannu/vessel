//! Projects' own icons (ADR 0112; Paseo's `projects/icon-cache.ts` and
//! `ProjectIconView`): each project's `project.icon` asked once when its
//! machine's list first names it, drawn in the sidebar's project rows and
//! Settings > Projects where the initial tile was; the tile stays for a
//! project without one and on an older machine.
//!
//! [Decision Log]
//! - Purpose: projects marked as Paseo marks them.
//! - Constraint: ADR 0112: an icon is drawn only as an image, never as SVG
//!   markup; the node decided its type from its bytes.
//! - Choice: GPUI's image from the bytes with the node's type (PNG, ICO,
//!   SVG rasterised by the image decoder), 16 square, contained, never
//!   clipped or rounded (Paseo leaves a person's mark as drawn). Asked once
//!   per machine and project for the app's run; a failed ask is no icon.
//! - Cost: an icon changed on disk shows on the next start (Paseo refreshes
//!   on file changes); a machine renamed asks again under its new name.

use std::collections::HashMap;
use std::sync::Arc;

use gpui::{Image, ImageFormat, ObjectFit, StyledImage as _, img};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{AnyElement, IntoElement as _, Styled as _, px};
use vessel_app_kit::{Tokens, initial_of, initial_tile};
use vessel_app_link::workspace::{Ask, ProjectAsk};
use vessel_app_view::workspace::host_folders::offered;
use vessel_app_view::workspace::problem::Problem;
use vessel_proto::ProjectId;
use vessel_proto::workspace::host_folders::{IconImage, IconType};

use crate::shell::Shell;

/// Each machine's projects' icons: asked (absent until answered, `None`
/// for none).
#[derive(Default)]
pub(crate) struct ProjectIcons {
    by: HashMap<(String, ProjectId), Option<Arc<Image>>>,
}

impl ProjectIcons {
    pub(super) fn icon(&self, machine: &str, project: ProjectId) -> Option<&Arc<Image>> {
        self.by
            .get(&(machine.to_owned(), project))
            .and_then(Option::as_ref)
    }
}

impl Shell {
    /// Ask the icon of each of `machine`'s listed projects not asked yet.
    pub(super) fn ask_project_icons(&mut self, machine: &str) {
        let speaks = self
            .fleet
            .machines
            .iter()
            .find(|known| known.name == machine)
            .and_then(|known| known.protocol);
        if !offered(speaks) {
            return;
        }
        let Some(list) = self
            .areas
            .workspace
            .projects
            .get(machine)
            .and_then(|view| view.list())
        else {
            return;
        };
        let asked = &self.areas.workspace.icons.by;
        let unasked: Vec<ProjectId> = list
            .projects
            .iter()
            .map(|project| project.id)
            .filter(|project| !asked.contains_key(&(machine.to_owned(), *project)))
            .collect();
        for project in unasked {
            let _asked = self
                .areas
                .workspace
                .icons
                .by
                .insert((machine.to_owned(), project), None);
            self.ask(machine, Ask::Project(ProjectAsk::Icon(project)));
        }
    }

    /// `project`'s icon arrived (or why not: no icon).
    pub(super) fn project_icon_read(
        &mut self,
        machine: &str,
        project: ProjectId,
        result: Result<Option<IconImage>, Problem>,
    ) {
        let image = result.ok().flatten().map(|icon| {
            let format = match icon.kind {
                IconType::Png => ImageFormat::Png,
                IconType::Ico => ImageFormat::Ico,
                IconType::Svg => ImageFormat::Svg,
            };
            Arc::new(Image::from_bytes(format, icon.bytes.bytes().to_vec()))
        });
        let _was = self
            .areas
            .workspace
            .icons
            .by
            .insert((machine.to_owned(), project), image);
    }
}

/// `project`'s mark: its icon, else its initial on a tile (16).
pub(super) fn mark(
    shell: &Shell,
    machine: &str,
    project: ProjectId,
    name: &str,
    tokens: &Tokens,
) -> AnyElement {
    match shell.areas.workspace.icons.icon(machine, project) {
        Some(image) => img(Arc::clone(image))
            .size(px(spacing::S4))
            .flex_none()
            .object_fit(ObjectFit::Contain)
            .into_any_element(),
        None => initial_tile(initial_of(name), tokens).into_any_element(),
    }
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests;
