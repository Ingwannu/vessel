//! The page's `data/machines.test.ts` and `nodes.test.ts`.

use crate::Lang;
use crate::machines::{
    BookNode, MachineActions, MachineChoice, MachineState, NodeStatus, Transport, machine_actions,
    machine_choices, machine_for, machine_line, machine_meant, name_problem,
};

/// The page's `bookNode`.
fn book_node(name: &str) -> BookNode {
    BookNode {
        name: name.to_owned(),
        transport: Transport::Direct,
        default: false,
    }
}

/// The page's `connectedAs`.
fn connected_as(os: &str) -> NodeStatus {
    NodeStatus::Connected {
        os: os.to_owned(),
        arch: "x86_64".to_owned(),
        draining: false,
    }
}

#[test]
fn says_what_it_runs_on_how_it_is_reached_its_state_and_its_sessions() {
    assert_eq!(
        machine_line(Lang::En, &book_node("box"), Some(&connected_as("linux")), 1),
        ["Linux x86_64", "Direct", "Connected", "1 live session"]
    );
    let mac = BookNode {
        transport: Transport::Paired,
        ..book_node("mac")
    };
    assert_eq!(
        machine_line(Lang::En, &mac, Some(&connected_as("macos")), 0),
        ["macOS x86_64", "Paired", "Connected", "0 live sessions"]
    );
}

#[test]
fn says_only_how_it_is_reached_and_its_state_while_not_connected() {
    let ssh = BookNode {
        transport: Transport::Ssh,
        ..book_node("box")
    };
    assert_eq!(
        machine_line(Lang::En, &ssh, Some(&NodeStatus::Offline), 0),
        ["Over SSH", "Offline"]
    );
    assert_eq!(
        Lang::En.machines().state(MachineState::of(None)),
        "Connecting"
    );
    let draining = NodeStatus::Connected {
        os: "linux".to_owned(),
        arch: "x86_64".to_owned(),
        draining: true,
    };
    assert_eq!(MachineState::of(Some(&draining)), MachineState::Draining);
}

/// The page's "offers forget and rename on book entries only, default
/// unless it is": the app has no "here" machine, so only the default
/// differs.
#[test]
fn offers_forget_and_rename_default_unless_it_is() {
    assert_eq!(
        machine_actions(&book_node("box")),
        MachineActions {
            rename: true,
            make_default: true,
            forget: true,
        }
    );
    let default = BookNode {
        default: true,
        ..book_node("box")
    };
    assert!(!machine_actions(&default).make_default);
}

#[test]
fn takes_the_books_names_and_no_other() {
    let book = [book_node("laptop"), book_node("gpu-box")];
    let problem = |text: &str| {
        name_problem(text, &book, None).map(|problem| Lang::En.machines().name_problem(&problem))
    };
    assert_eq!(problem("studio"), None);
    assert_eq!(problem("build.server_2-a"), None);
    assert_eq!(problem("").as_deref(), Some("Give it a name."));
    for start in ["-box", "_here", "상자"] {
        assert_eq!(
            problem(start).as_deref(),
            Some("Start with a letter or a digit."),
            "{start}"
        );
    }
    assert_eq!(
        problem("two words").as_deref(),
        Some("Use letters, digits, “.”, “_” and “-” only (no spaces).")
    );
    assert_eq!(
        problem(&"a".repeat(65)).as_deref(),
        Some("At most 64 characters.")
    );
    assert_eq!(problem(&"a".repeat(64)), None);
}

#[test]
fn refuses_a_taken_name_except_the_machines_own_when_renaming() {
    let book = [book_node("laptop"), book_node("gpu-box")];
    assert_eq!(
        name_problem("gpu-box", &book, None)
            .map(|problem| Lang::En.machines().name_problem(&problem)),
        Some("There is already a machine called gpu-box.".to_owned())
    );
    assert_eq!(name_problem("gpu-box", &book, Some("gpu-box")), None);
}

#[test]
fn is_the_addresss_else_the_last_chosen_else_the_default_in_the_book() {
    let book = [
        book_node("laptop"),
        BookNode {
            default: true,
            ..book_node("gpu-box")
        },
        book_node("build-server"),
    ];
    let name = |asked, selected| machine_for(&book, asked, selected).map(|node| node.name.as_str());
    assert_eq!(name(Some("laptop"), Some("build-server")), Some("laptop"));
    assert_eq!(name(None, Some("build-server")), Some("build-server"));
    assert_eq!(name(Some("missing"), None), Some("gpu-box"));
    assert_eq!(machine_for(&[], None, None), None);
}

/// `nodes.test.ts`.
#[test]
fn is_the_one_asked_for_else_the_last_chosen_else_the_default_else_the_first() {
    let book = [
        book_node("gpu-box"),
        BookNode {
            default: true,
            ..book_node("laptop")
        },
        book_node("nas"),
    ];
    let any = |_: &str| true;
    assert_eq!(
        machine_meant(&book, &[Some("nas"), Some("gpu-box")], any),
        Some("nas")
    );
    assert_eq!(
        machine_meant(&book, &[None, Some("gpu-box")], any),
        Some("gpu-box")
    );
    assert_eq!(machine_meant(&book, &[None, None], any), Some("laptop"));
    assert_eq!(machine_meant(&book[..1], &[], any), Some("gpu-box"));
    assert_eq!(machine_meant(&[], &[], any), None);
}

#[test]
fn is_only_one_that_is_accepted() {
    let book = [
        book_node("gpu-box"),
        BookNode {
            default: true,
            ..book_node("laptop")
        },
        book_node("nas"),
    ];
    let connected = |name: &str| name == "nas";
    assert_eq!(
        machine_meant(&book, &[Some("gpu-box"), Some("laptop")], connected),
        Some("nas")
    );
    assert_eq!(machine_meant(&book, &[], |_| false), None);
}

#[test]
fn are_offered_by_name_the_unconnected_one_saying_why() {
    let book = [book_node("laptop"), book_node("build-server")];
    let status = |name: &str| (name == "laptop").then(|| connected_as("linux"));
    assert_eq!(
        machine_choices(Lang::En, &book, status, false),
        [
            MachineChoice {
                value: "laptop".to_owned(),
                label: "laptop".to_owned(),
                detail: None,
                disabled: false,
            },
            MachineChoice {
                value: "build-server".to_owned(),
                label: "build-server".to_owned(),
                detail: Some("Connecting".to_owned()),
                disabled: false,
            },
        ]
    );
    assert!(machine_choices(Lang::En, &book, status, true)[1].disabled);
    assert!(!machine_choices(Lang::En, &book, status, true)[0].disabled);
}
