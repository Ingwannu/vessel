//! Complete rendering projection shared by grant and start workflows.
use super::as_listed;
use vessel_app_view::workspace::allocation_changes::AllocationLine;
use vessel_app_view::workspace::command_changes::{
    Change, CommandLine, Compared, Listed, ScriptLine,
};
use vessel_app_view::workspace::terminal_changes::TerminalLine;

pub(super) struct Displayed {
    pub(super) setup: Vec<CommandLine>,
    pub(super) teardown: Vec<CommandLine>,
    pub(super) scripts: Vec<ScriptLine>,
    pub(super) terminals: Vec<TerminalLine>,
    pub(super) allocation: Box<AllocationLine>,
}
pub(super) fn displayed(asked: &Listed<'_>, compared: &Compared) -> Displayed {
    match compared {
        Compared::Changed {
            against: _,
            setup,
            teardown,
            scripts,
            terminals,
            allocation,
            unshown: _,
        } => Displayed {
            setup: setup.clone(),
            teardown: teardown.clone(),
            scripts: scripts.clone(),
            terminals: terminals.clone(),
            allocation: allocation.clone(),
        },
        Compared::None => Displayed {
            setup: as_listed(asked.setup),
            teardown: as_listed(asked.teardown),
            scripts: asked
                .scripts
                .iter()
                .map(|script| ScriptLine::Line {
                    change: Change::Same,
                    script: script.clone(),
                })
                .collect(),
            terminals: vessel_app_view::workspace::terminal_changes::compare(
                asked.terminals,
                asked.terminals,
            ),
            allocation: Box::new(vessel_app_view::workspace::allocation_changes::compare(
                asked.service_ports,
                asked.service_ports,
            )),
        },
    }
}
