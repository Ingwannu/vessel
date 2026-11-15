//! The draft's answer to the machine book changing (track E2's Settings ›
//! Machines; split from `mod.rs` for the file budget, docs/QUALITY.md M3).

use vessel_app_view::Picks;

use crate::shell::Shell;

impl Shell {
    /// The machine `from` is called `to` now (Settings › Machines, track
    /// E2, called after the book was written): the draft's machine, what
    /// its agents offer and the machine its choices are for follow the new
    /// name, so nothing is asked again.
    #[expect(
        dead_code,
        reason = "track E2's Settings › Machines calls it after a book edit (remove on its merge)"
    )]
    pub(crate) fn draft_machine_renamed(&mut self, from: &str, to: &str) {
        let draft = &mut self.areas.draft;
        if draft.machine.as_deref() == Some(from) {
            draft.machine = Some(to.to_owned());
        }
        self.areas.composer.offers.renamed(from, to);
        if let Some((machine, _)) = &mut draft.settled
            && machine == from
        {
            to.clone_into(machine);
        }
    }

    /// The machine `name` is forgotten (track E2, after the book was
    /// written): the draft drops it, its folder and its choices; the next
    /// frame settles on another machine as at start.
    #[expect(
        dead_code,
        reason = "track E2's Settings › Machines calls it after a book edit (remove on its merge)"
    )]
    pub(crate) fn draft_machine_forgotten(&mut self, name: &str) {
        let draft = &mut self.areas.draft;
        if draft.machine.as_deref() == Some(name) {
            draft.machine = None;
            draft.folder.clear();
            draft.picks = Picks::default();
            draft.chosen = false;
            draft.settled = None;
        }
        self.areas.composer.offers.forgotten(name);
    }
}
