//! Real screen handlers share one bounded, I/O-free test fixture.
use crate::shell::Shell;
use vessel_app_kit::ui::Context;
use vessel_app_view::agent_status::{
    command::CommandShell,
    pages::{Administration, Inventory, MachineKey, Operation},
};
use vessel_proto::{
    HarnessRef, NodeId,
    agents::{AddedAgent, AgentDetail, AgentInfo, AgentSource},
};

pub(super) const COMMAND_CAPACITY: usize = 8;
pub(super) fn init(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        vessel_app_kit::init(
            cx,
            vessel_app_kit::SchemeChoice::Fixed(vessel_app_kit::Scheme::Dark),
        )
        .unwrap();
    });
}
pub(super) fn node() -> NodeId {
    "00000000-0000-4000-8000-000000000001".parse().unwrap()
}
pub(super) fn added(name: &str) -> AgentInfo {
    AgentInfo {
        agent: HarnessRef::Added(name.parse().unwrap()),
        detail: AgentDetail::Added {
            added: AddedAgent {
                label: None,
                program: "fake-acp".parse().unwrap(),
                args: Vec::new(),
                env: Vec::new(),
                secrets: Vec::new(),
                inherit: Vec::new(),
                mcp: Vec::new(),
                source: AgentSource::Command,
            },
        },
    }
}
// A reported node is sufficient for these legacy local admission rules. This
// fixture supplies no transport, scopes or canonical delivery lease proof.
pub(super) fn select(
    shell: &mut Shell,
    name: &str,
    agents: Vec<AgentInfo>,
    cx: &mut Context<'_, Shell>,
) {
    if !shell
        .fleet
        .machines
        .iter()
        .any(|machine| machine.name == name)
    {
        let mut machine =
            vessel_app_view::Machine::unreachable(name.into(), "test has no transport");
        machine.node = Some(node());
        shell.fleet.machines.push(machine);
    }
    shell.route = crate::routes::Route::Settings;
    shell.areas.settings.section = vessel_app_view::settings::Section::Providers;
    shell.areas.settings.pushed = true;
    shell.areas.settings.machine = Some(name.into());
    let key = MachineKey::from_book(name);
    let pages = &mut shell.areas.settings.agents.pages;
    let ticket = pages.begin(key.clone(), Operation::Discovery).unwrap();
    assert!(pages.loaded(&key, ticket, inventory(agents),));
    shell.agent_load(false, cx);
}

// Same typed snapshot for admission and late-reply fixtures; no transport.
pub(super) fn inventory(agents: Vec<AgentInfo>) -> Inventory {
    Inventory {
        node: node(),
        command_shell: CommandShell::Unix,
        harnesses: Vec::new(),
        agents,
        readiness: Vec::new(),
        administration: Administration::Supported,
    }
}
