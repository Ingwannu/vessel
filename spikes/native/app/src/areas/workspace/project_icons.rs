//! Projects' own icons (ADR 0112; Paseo's `projects/icon-cache.ts` and
//! `ProjectIconView`): each project's `project.icon` asked once when its
//! machine's list first names it, drawn in the sidebar's project rows and
//! Settings > Projects; a project without one (or on an older machine)
//! gets Paseo's lettered identity square (TRACKER 809).
//!
//! [Decision Log]
//! - Purpose: projects marked as Paseo marks them.
//! - Constraint: ADR 0112: an icon is drawn only as an image, never as SVG
//!   markup; the node decided its type from its bytes.
//! - Choice: GPUI's image from the bytes with the node's type (PNG, ICO,
//!   SVG rasterised by the image decoder), 16 square, contained, never
//!   clipped or rounded (Paseo leaves a person's mark as drawn). Asked once
//!   per machine and project for the app's run; a failed ask is no icon.
//! - Asks go one at a time per machine (the node's file work has four
//!   slots and refuses beyond them at once); a refused ask is asked again
//!   when the machine's list is read again.
//! - Cost: an icon changed on disk shows on the next start (Paseo refreshes
//!   on file changes); a machine renamed asks again under its new name.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use gpui::{Image, ImageFormat, ObjectFit, StyledImage as _, img};
use vessel_app_kit::theme::identity::{self, Hue};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{AnyElement, IntoElement as _, ParentElement as _, Styled as _, div, px};
use vessel_app_link::workspace::{Ask, ProjectAsk};
use vessel_app_view::workspace::host_folders::offered;
use vessel_app_view::workspace::problem::Problem;
use vessel_proto::ProjectId;
use vessel_proto::workspace::host_folders::{IconImage, IconType};

use crate::shell::Shell;

/// Each machine's projects' icons: asked (absent until answered, `None`
/// for none), and each machine's queue of asks. One ask is out per machine
/// at a time: the node serves file work on four slots and refuses a fifth
/// at once (`busy`), so a list of many projects asked together lost icons.
#[derive(Default)]
pub(crate) struct ProjectIcons {
    by: HashMap<(String, ProjectId), Option<Arc<Image>>>,
    waiting: HashMap<String, VecDeque<ProjectId>>,
    out: HashSet<String>,
}

impl ProjectIcons {
    pub(super) fn icon(&self, machine: &str, project: ProjectId) -> Option<&Arc<Image>> {
        self.by
            .get(&(machine.to_owned(), project))
            .and_then(Option::as_ref)
    }

    /// The next ask for `machine`, when none is out.
    fn next(&mut self, machine: &str) -> Option<ProjectId> {
        if self.out.contains(machine) {
            return None;
        }
        let project = self.waiting.get_mut(machine)?.pop_front()?;
        let _out = self.out.insert(machine.to_owned());
        Some(project)
    }
}

impl Shell {
    /// Queue the icon of each of `machine`'s listed projects not asked yet,
    /// and ask the first when none is out.
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
        let icons = &mut self.areas.workspace.icons;
        for project in unasked {
            let _asked = icons.by.insert((machine.to_owned(), project), None);
            icons
                .waiting
                .entry(machine.to_owned())
                .or_default()
                .push_back(project);
        }
        self.ask_next_icon(machine);
    }

    fn ask_next_icon(&mut self, machine: &str) {
        if let Some(project) = self.areas.workspace.icons.next(machine) {
            self.ask(machine, Ask::Project(ProjectAsk::Icon(project)));
        }
    }

    /// `project`'s icon arrived (or why not: no icon; a refusal is asked
    /// again with the machine's next list), and the next ask goes out.
    pub(super) fn project_icon_read(
        &mut self,
        machine: &str,
        project: ProjectId,
        result: Result<Option<IconImage>, Problem>,
    ) {
        let icons = &mut self.areas.workspace.icons;
        let _done = icons.out.remove(machine);
        match result {
            Ok(icon) => {
                let image = icon.map(|icon| {
                    let format = match icon.kind {
                        IconType::Png => ImageFormat::Png,
                        IconType::Ico => ImageFormat::Ico,
                        IconType::Svg => ImageFormat::Svg,
                    };
                    Arc::new(Image::from_bytes(format, icon.bytes.bytes().to_vec()))
                });
                let _was = icons.by.insert((machine.to_owned(), project), image);
            }
            Err(_) => {
                let _forgotten = icons.by.remove(&(machine.to_owned(), project));
            }
        }
        self.ask_next_icon(machine);
    }
}

/// Paseo's mark at a compact width (`densityIconProps("md")`: 20; 16 is
/// the kit's `S4`), and its square's corners, a quarter of its side.
const COMPACT_MARK: f32 = 20.0;
const CORNER: f32 = 4.0;
const COMPACT_CORNER: f32 = 5.0;
/// Paseo's letter on the square (`projectIconFallbackText`).
const LETTER: f32 = 9.0;

/// Paseo's placeholder initial (`projectIconPlaceholderLabelFromDisplayName`
/// then its first character, upper-cased): the last `/` segment's.
fn initial(name: &str) -> String {
    let trimmed = name.trim();
    let last = trimmed
        .split('/')
        .rfind(|segment| !segment.is_empty())
        .unwrap_or(trimmed);
    last.chars()
        .next()
        .map(char::to_uppercase)
        .map_or_else(String::new, Iterator::collect)
}

/// `project`'s mark (TRACKER 809, Paseo's `ProjectIconView`): its icon,
/// else its lettered identity square (the fill keyed by the project's id,
/// a white 9 letter); 16 square, 20 at a compact width.
pub(super) fn mark(
    shell: &Shell,
    machine: &str,
    project: ProjectId,
    name: &str,
    compact: bool,
) -> AnyElement {
    let side = if compact { COMPACT_MARK } else { spacing::S4 };
    match shell.areas.workspace.icons.icon(machine, project) {
        Some(image) => img(Arc::clone(image))
            .size(px(side))
            .flex_none()
            .object_fit(ObjectFit::Contain)
            .into_any_element(),
        None => div()
            .size(px(side))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(if compact { COMPACT_CORNER } else { CORNER }))
            .bg(identity::fill(Hue::of_key(&project.to_string())))
            .text_size(px(LETTER))
            .line_height(px(side))
            .text_color(gpui::white())
            .child(initial(name))
            .into_any_element(),
    }
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests;
