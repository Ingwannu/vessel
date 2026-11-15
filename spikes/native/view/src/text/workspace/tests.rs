//! The page's `words.test.ts`, the word cases of `place.test.ts`,
//! `command-changes.test.ts` and `panel-model.test.ts`, against the English
//! table; and the Korean table read for the same cases.

use chrono::{FixedOffset, NaiveDate, TimeZone, Utc};
use vessel_proto::ProjectId;
use vessel_proto::scripts::{ScriptEntry, ScriptKind};
use vessel_proto::workspace::{
    ApprovedCommands, CloneFailure, CloneStage, CommandSet, DiffScope, GitOperation, GitRefusal,
    GitSupport, GitVersion, KeptReason, ProjectCommand, ProjectCommands, ProjectInfo, ProjectState,
    RelPath, RepositoryConfigReason, SetupFailure, WorktreeRefusal,
};

use super::{En, Ko, Local, WorkspaceWords};
use crate::workspace::branch_name::branch_name_problem;
use crate::workspace::command_changes::{
    Change, Compared, Listed, ScriptLine, compare_approved, compare_scripts, needs_folder,
};
use crate::workspace::place::{Place, PlaceMode, ProjectPlace, place_problem};

fn local() -> Local {
    Local {
        offset: FixedOffset::east_opt(0).unwrap(),
        today: NaiveDate::from_ymd_opt(2026, 11, 13).unwrap(),
    }
}

#[test]
fn names_what_happened_and_what_to_do_never_the_reason_s_name() {
    let push = GitRefusal::NonFastForward;
    let words = En.refusal(&push);
    assert_eq!(words.title, "The remote has commits this branch does not.");
    assert!(words.next.contains("Pull first"));
    assert!(!words.title.contains("non_fast_forward"));
}

#[test]
fn carries_the_paths_and_the_output_a_refusal_names() {
    let overwrite = GitRefusal::WouldOverwrite {
        paths: vec!["src/a.ts".parse().unwrap(), "README.md".parse().unwrap()],
    };
    let words = En.refusal(&overwrite);
    let paths: Vec<&str> = words.paths.iter().map(RelPath::as_str).collect();
    assert_eq!(paths, ["src/a.ts", "README.md"]);
    let hook = GitRefusal::HookFailed {
        output: "lint failed".to_owned(),
    };
    assert_eq!(En.refusal(&hook).output, Some("lint failed"));
    assert_eq!(En.refusal(&GitRefusal::NothingToCommit).output, None);
}

#[test]
fn says_which_operation_holds_the_repository() {
    let refusal = GitRefusal::Operation {
        operation: GitOperation::CherryPick,
    };
    assert_eq!(
        En.refusal(&refusal).title,
        "The repository is in the middle of a cherry-pick."
    );
}

#[test]
fn reads_a_reason_from_a_newer_machine_as_unknown_not_as_a_failure() {
    assert!(
        En.refusal(&GitRefusal::Unknown)
            .title
            .contains("does not know")
    );
}

#[test]
fn names_the_repository_s_own_setting_that_stopped_it_never_a_value() {
    let ssh = GitRefusal::RepositoryConfig {
        key: Some("core.sshcommand".parse().unwrap()),
        reason: RepositoryConfigReason::Program,
    };
    let words = En.refusal(&ssh);
    assert_eq!(
        words.title,
        "Its setting core.sshcommand names a program this would run."
    );
    assert!(words.next.contains("global git configuration"));
    let whole = GitRefusal::RepositoryConfig {
        key: None,
        reason: RepositoryConfigReason::TooLarge,
    };
    assert!(
        En.refusal(&whole)
            .title
            .contains("larger than Vessel reads")
    );
}

#[test]
fn counts_with_the_noun() {
    assert_eq!(En.files(1), "1 file");
    assert_eq!(En.files(3), "3 files");
}

#[test]
fn says_how_many_a_search_found_and_when_it_stopped_short() {
    assert_eq!(En.found_count(1, 40, true, false), "1 file");
    assert_eq!(
        En.found_count(12, 50_000, false, false),
        "12 files, from the first 50000 searched"
    );
    assert_eq!(
        En.found_count(100, 400, true, true),
        "The best 100 files; type more of the name to narrow them."
    );
    assert_eq!(En.grepped_count(1, 1, false), "1 line in 1 file");
    assert_eq!(
        En.grepped_count(200, 41, true),
        "The first 200 lines, in 41 files; there are more. Narrow the search to see them."
    );
    assert_eq!(Ko.found_count(12, 50_000, true, false), "파일 12개");
    assert_eq!(Ko.grepped_count(200, 41, false), "파일 41개에서 200줄");
    assert!(Ko.grepped_count(200, 41, true).contains("검색어를 좁혀"));
}

#[test]
fn says_each_reason_a_worktree_was_kept() {
    assert_eq!(
        En.kept_reason(KeptReason::Uncommitted { files: 2 }),
        "2 files with uncommitted changes"
    );
    assert_eq!(
        En.kept_reason(KeptReason::UniqueCommits { commits: 1 }),
        "1 commit on no other branch"
    );
    assert_eq!(
        En.kept_reason(KeptReason::Operation {
            operation: GitOperation::Rebase
        }),
        "A rebase in progress"
    );
    assert!(En.kept_reason(KeptReason::InUse).contains("a terminal"));
}

#[test]
fn names_the_setup_command_that_failed() {
    assert_eq!(
        En.setup_failure(
            &SetupFailure::Exit {
                command: 1,
                code: Some(2)
            },
            &["npm ci", "npm run build"]
        ),
        "`npm run build` exited with 2."
    );
    assert_eq!(
        En.setup_failure(
            &SetupFailure::Exit {
                command: 4,
                code: None
            },
            &[]
        ),
        "Setup command 5 was stopped by a signal."
    );
}

#[test]
fn shows_a_clone_s_stage_and_percent_and_nothing_for_a_ready_project() {
    assert_eq!(
        En.project_state(&ProjectState::Cloning {
            stage: CloneStage::Receiving,
            percent: Some(42)
        }),
        Some("Receiving objects \u{b7} 42%".to_owned())
    );
    assert_eq!(En.project_state(&ProjectState::Ready), None);
    assert!(
        En.clone_failure(CloneFailure::NeedsCredential)
            .contains("credential")
    );
}

#[test]
fn says_when_the_machine_s_git_cannot_be_used() {
    let old = GitSupport::TooOld {
        version: GitVersion {
            major: 2,
            minor: 17,
            patch: 1,
        },
        minimum: GitVersion {
            major: 2,
            minor: 25,
            patch: 0,
        },
    };
    assert!(En.git_support(&old).unwrap().contains("2.17 is too old"));
    let found = GitSupport::Found {
        version: GitVersion {
            major: 2,
            minor: 43,
            patch: 0,
        },
    };
    assert_eq!(En.git_support(&found), None);
}

#[test]
fn says_why_a_worktree_was_not_made() {
    let refusal = WorktreeRefusal::CheckedOutElsewhere {
        path: "/w/x".into(),
    };
    assert!(En.worktree_refusal(&refusal).contains("/w/x"));
}

#[test]
fn a_branch_name_as_git_checks_it() {
    for (name, problem) in [
        ("feature/login", None),
        ("-x", Some("dash")),
        ("a..b", Some("..")),
        ("a/.hidden", Some("start with a dot")),
        ("topic.lock", Some(".lock")),
        ("end.", Some("end with a dot")),
        ("a//b", Some("two in a row")),
        ("what?", Some("\u{201c}?\u{201d}")),
        ("HEAD", Some("not a branch name")),
    ] {
        let said = branch_name_problem(name).map(|problem| En.branch_name(problem));
        match problem {
            None => assert_eq!(said, None, "{name}"),
            Some(part) => assert!(said.as_deref().unwrap_or_default().contains(part), "{name}"),
        }
    }
}

fn p1() -> ProjectId {
    "00000000-0000-4000-8000-000000000001".parse().unwrap()
}

fn project() -> ProjectInfo {
    ProjectInfo {
        id: p1(),
        name: "vessel".parse().unwrap(),
        root: "/work/vessel".into(),
        git: true,
        default_branch: Some("origin/main".parse().unwrap()),
        state: ProjectState::Ready,
        created_at: Utc.with_ymd_and_hms(2026, 11, 12, 0, 0, 0).unwrap(),
        revision: 1,
    }
}

fn in_worktree(mode: PlaceMode, branch: &str) -> Place {
    Place::Project(ProjectPlace {
        mode,
        branch: branch.to_owned(),
        ..ProjectPlace::new(p1())
    })
}

#[test]
fn says_why_it_cannot_start_yet() {
    let said = |place: &Place, project: Option<&ProjectInfo>| {
        place_problem(place, project).map(|problem| En.place(problem))
    };
    let folder = Place::Project(ProjectPlace::new(p1()));
    assert!(said(&folder, None).unwrap().contains("not on"));
    let cloning = ProjectInfo {
        state: ProjectState::Cloning {
            stage: CloneStage::Receiving,
            percent: None,
        },
        ..project()
    };
    assert!(
        said(&folder, Some(&cloning))
            .unwrap()
            .contains("still being cloned")
    );
    let plain = ProjectInfo {
        git: false,
        ..project()
    };
    assert!(
        said(&in_worktree(PlaceMode::Worktree, ""), Some(&plain))
            .unwrap()
            .contains("not a git repository")
    );
    assert_eq!(
        said(&in_worktree(PlaceMode::Branch, ""), Some(&project())).as_deref(),
        Some("Choose the branch to run on.")
    );
    assert!(
        said(
            &in_worktree(PlaceMode::Worktree, "bad name"),
            Some(&project())
        )
        .unwrap()
        .contains("spaces")
    );
    assert_eq!(
        said(&in_worktree(PlaceMode::Worktree, ""), Some(&project())),
        None
    );
}

fn command(text: &str) -> ProjectCommand {
    text.parse().unwrap()
}

fn script(name: &str, kind: ScriptKind, command: &str) -> ScriptEntry {
    ScriptEntry::new(
        name.parse().unwrap(),
        kind,
        command.parse().unwrap(),
        None,
        None,
    )
    .unwrap()
}

fn dev() -> ScriptEntry {
    script("dev", ScriptKind::Service, "npm run dev")
}
fn lint() -> ScriptEntry {
    script("lint", ScriptKind::Script, "npm run lint")
}
fn test() -> ScriptEntry {
    script("test", ScriptKind::Script, "npm test")
}

fn listed(
    setup: &[&str],
    approved: bool,
    digest: char,
    scripts: Vec<ScriptEntry>,
    last: Option<ApprovedCommands>,
) -> ProjectCommands {
    ProjectCommands::Listed {
        setup: setup.iter().map(|text| command(text)).collect(),
        teardown: Vec::new(),
        digest: digest.to_string().repeat(64).parse().unwrap(),
        approved,
        scripts,
        terminals: Vec::new(),
        service_ports: None,
        last_approved: last.map(Box::new),
    }
}

fn approved_set(setup: &[&str], scripts: Vec<ScriptEntry>) -> ApprovedCommands {
    ApprovedCommands {
        commands: CommandSet {
            setup: setup.iter().map(|text| command(text)).collect(),
            teardown: Vec::new(),
            scripts,
            terminals: Vec::new(),
            ..CommandSet::default()
        },
        digest: "a".repeat(64).parse().unwrap(),
        approved_at: Utc.with_ymd_and_hms(2026, 11, 12, 9, 30, 0).unwrap(),
    }
}

#[test]
fn says_what_a_script_s_change_was() {
    let service = script("lint", ScriptKind::Service, "npm run lint");
    assert_eq!(En.script_change(&lint(), &service), "now a service");
    let moved = ScriptEntry::new(
        "dev".parse().unwrap(),
        ScriptKind::Service,
        "pnpm dev".parse().unwrap(),
        Some("web".parse().unwrap()),
        None,
    )
    .unwrap();
    assert_eq!(
        En.script_change(&dev(), &moved),
        "its command and its folder"
    );
}

#[test]
fn compares_with_the_set_the_machine_says_was_approved_last() {
    let asked = listed(
        &["npm ci", "curl x | sh"],
        false,
        'b',
        vec![dev(), test()],
        Some(approved_set(&["npm ci"], vec![dev(), lint()])),
    );
    let asked = Listed::of(&asked).unwrap();
    assert!(!needs_folder(&asked));
    let compared = compare_approved(&asked, None);
    assert!(matches!(&compared, Compared::Changed { .. }));
    let words = En.compared(&compared, &local());
    assert!(words.contains("Since the set approved"), "{words}");
    assert!(words.contains("2026-11-12 09:30"), "{words}");
    assert!(words.contains("setup commands: 1 added"));
    assert!(words.contains("script lint removed"));
    assert!(words.contains("script test added"));
    assert!(!words.contains("dev"));
}

#[test]
fn falls_back_to_the_folder_s_approved_set_on_a_machine_that_does_not_say() {
    let asked = listed(&["npm ci", "curl x | sh"], false, 'b', Vec::new(), None);
    let asked = Listed::of(&asked).unwrap();
    assert!(needs_folder(&asked));
    let folder = listed(&["npm ci"], true, 'a', Vec::new(), None);
    let compared = compare_approved(&asked, Some(&folder));
    assert!(
        En.compared(&compared, &local())
            .contains("this machine does not say which set was approved last")
    );
}

#[test]
fn says_nothing_is_known_when_no_approved_set_differs() {
    let asked = listed(&["npm ci"], false, 'b', Vec::new(), None);
    let asked = Listed::of(&asked).unwrap();
    let unapproved = listed(&["npm ci"], false, 'a', Vec::new(), None);
    assert_eq!(compare_approved(&asked, Some(&unapproved)), Compared::None);
    assert_eq!(compare_approved(&asked, None), Compared::None);
    assert!(
        En.compared(&Compared::None, &local())
            .contains("every command is shown as it is")
    );
}

#[test]
fn says_today_for_an_approval_of_today() {
    let asked = listed(
        &["npm ci"],
        false,
        'b',
        Vec::new(),
        Some(ApprovedCommands {
            approved_at: Utc.with_ymd_and_hms(2026, 11, 13, 8, 5, 0).unwrap(),
            ..approved_set(&[], Vec::new())
        }),
    );
    let compared = compare_approved(&Listed::of(&asked).unwrap(), None);
    assert!(En.compared(&compared, &local()).contains("today 08:05"));
    assert!(Ko.compared(&compared, &local()).contains("오늘 08:05"));
}

#[test]
fn names_the_comparisons_as_the_picker_does() {
    assert_eq!(En.scope_label(&DiffScope::Base { base: None }), "Changes");
    let commit = DiffScope::Commit {
        id: "0123456789abcdef".parse().unwrap(),
    };
    assert_eq!(En.scope_label(&commit), "Commit 0123456");
}

#[test]
fn says_every_case_in_korean_too() {
    // The same cases read in Korean: something is said, and it is not the
    // English sentence left untranslated.
    let refusals = [
        GitRefusal::NonFastForward,
        GitRefusal::Operation {
            operation: GitOperation::CherryPick,
        },
        GitRefusal::Unknown,
    ];
    for refusal in &refusals {
        let (en, ko) = (En.refusal(refusal), Ko.refusal(refusal));
        assert!(!ko.title.is_empty() && ko.title != en.title);
        assert!(!ko.next.is_empty() && ko.next != en.next);
    }
    assert_eq!(Ko.files(3), "파일 3개");
    assert_eq!(
        Ko.kept_reason(KeptReason::Operation {
            operation: GitOperation::Rebase
        }),
        "리베이스 진행 중"
    );
    assert_eq!(
        Ko.setup_failure(
            &SetupFailure::Exit {
                command: 1,
                code: Some(2)
            },
            &["npm ci", "npm run build"]
        ),
        "`npm run build`이(가) 코드 2로 끝났습니다."
    );
    let problem = branch_name_problem("what?").unwrap();
    assert!(Ko.branch_name(problem).contains("\u{201c}?\u{201d}"));
}

#[test]
fn refuses_approval_of_unshown_terminal_commands() {
    use crate::workspace::command_changes::ApprovalRefusal;
    use vessel_proto::workspace::FileTerminal;
    let mut asked = listed(&[], false, 'c', vec![], None);
    assert_eq!(Listed::of(&asked).unwrap().approval(), Ok(()));
    if let ProjectCommands::Listed { terminals, .. } = &mut asked {
        terminals.push(FileTerminal {
            name: None,
            command: command("npm run watch"),
        });
    }
    assert_eq!(
        Listed::of(&asked).unwrap().approval(),
        Err(ApprovalRefusal::Terminals)
    );
    assert!(!En.labels().approval_unseen.is_empty());
    assert!(!Ko.labels().approval_unseen.is_empty());
}

#[test]
fn refuses_comparison_that_would_hide_removed_terminal_commands() {
    use crate::workspace::command_changes::ApprovalRefusal;
    use vessel_proto::workspace::FileTerminal;
    let mut previous = approved_set(&[], vec![]);
    previous.commands.terminals.push(FileTerminal {
        name: None,
        command: command("npm run watch"),
    });
    let asked = listed(&[], false, 'c', vec![], Some(previous));
    assert_eq!(
        Listed::of(&asked).unwrap().approval(),
        Err(ApprovalRefusal::Terminals)
    );
}

#[test]
fn compares_and_names_declared_port_additions_changes_and_removal() {
    use vessel_proto::workspace::ServicePort;
    let without = dev();
    let declared = |port: u16| {
        ScriptEntry::new(
            without.name.clone(),
            without.kind(),
            without.command.clone(),
            without.dir.clone(),
            Some(ServicePort::try_from(port).unwrap()),
        )
        .unwrap()
    };
    let first = declared(3000);
    let second = declared(3001);
    for (before, after, english, korean) in [
        (
            &without,
            &first,
            "no declared port → port 3000",
            "지정 포트 없음 → 포트 3000",
        ),
        (
            &first,
            &second,
            "port 3000 → port 3001",
            "포트 3000 → 포트 3001",
        ),
        (
            &first,
            &without,
            "port 3000 → no declared port",
            "포트 3000 → 지정 포트 없음",
        ),
    ] {
        assert_eq!(
            compare_scripts(std::slice::from_ref(before), std::slice::from_ref(after)),
            vec![ScriptLine::Changed {
                script: after.clone(),
                was: before.clone()
            }]
        );
        assert_eq!(En.script_change(before, after), english);
        assert_eq!(Ko.script_change(before, after), korean);
    }
    assert_eq!(
        compare_scripts(std::slice::from_ref(&first), std::slice::from_ref(&first)),
        vec![ScriptLine::Line {
            change: Change::Same,
            script: first
        }]
    );
}

#[test]
fn an_approved_declared_port_removal_is_still_a_change() {
    let previous = ScriptEntry::new(
        "dev".parse().unwrap(),
        ScriptKind::Service,
        "npm run dev".parse().unwrap(),
        None,
        Some("3000".parse().unwrap()),
    )
    .unwrap();
    let asked = listed(
        &[],
        false,
        'b',
        vec![dev()],
        Some(approved_set(&[], vec![previous.clone()])),
    );
    let compared = compare_approved(&Listed::of(&asked).unwrap(), None);
    let Compared::Changed { scripts, .. } = compared else {
        panic!("expected an approved comparison")
    };
    assert_eq!(
        scripts,
        vec![ScriptLine::Changed {
            script: dev(),
            was: previous
        }]
    );
}

#[test]
fn allocation_choices_are_fully_projected_and_refused_without_any_service() {
    use crate::workspace::command_changes::ApprovalRefusal;
    use vessel_proto::workspace::{PortRange, PortScript, ServicePorts, ServicePortsView};
    let range: PortRange = "7000-7010".parse().unwrap();
    let helper: PortScript = "tools/port".parse().unwrap();
    for ports in [
        ServicePorts::new(Some(range), None).unwrap(),
        ServicePorts::new(None, Some(helper.clone())).unwrap(),
        ServicePorts::new(Some(range), Some(helper)).unwrap(),
    ] {
        let mut asked = listed(&[], false, 'c', vec![], None);
        if let ProjectCommands::Listed { service_ports, .. } = &mut asked {
            *service_ports = Some(ports.clone());
        }
        let listed = Listed::of(&asked).unwrap();
        assert!(listed.scripts.is_empty());
        let ServicePortsView { range, port_script } = listed.service_ports.unwrap();
        assert_eq!(range, ports.range());
        assert_eq!(port_script, ports.port_script());
        assert_eq!(listed.approval(), Err(ApprovalRefusal::ServicePorts));
    }
}

#[test]
fn removing_a_previously_approved_allocation_block_is_also_refused() {
    use crate::workspace::command_changes::ApprovalRefusal;
    use vessel_proto::workspace::ServicePorts;
    let mut prior = approved_set(&[], vec![]);
    prior.commands.service_ports =
        Some(ServicePorts::new(Some("7000-7010".parse().unwrap()), None).unwrap());
    let asked = listed(&[], false, 'c', vec![], Some(prior));
    let listed = Listed::of(&asked).unwrap();
    assert!(listed.service_ports.is_none());
    assert_eq!(listed.approval(), Err(ApprovalRefusal::ServicePorts));
}

#[test]
fn allocation_refusal_has_complete_english_and_korean_copy() {
    use crate::localization::{ENGLISH, Locale, Translator};
    let english = Translator::get(ENGLISH)
        .unwrap()
        .approval()
        .unseen_service_ports();
    let korean = Translator::get(Locale::from_tag("ko").unwrap())
        .unwrap()
        .approval()
        .unseen_service_ports();
    assert!(english.contains("service port"));
    assert!(korean.contains("서비스 포트"));
}

#[test]
fn an_approved_folder_fallback_cannot_hide_removed_allocation_choices() {
    use crate::workspace::command_changes::ApprovalRefusal;
    use vessel_proto::workspace::ServicePorts;
    let asked = listed(&[], false, 'c', vec![], None);
    let mut folder = listed(&[], true, 'b', vec![], None);
    if let ProjectCommands::Listed { service_ports, .. } = &mut folder {
        *service_ports =
            Some(ServicePorts::new(None, Some("tools/port".parse().unwrap())).unwrap());
    }
    let asked = Listed::of(&asked).unwrap();
    assert_eq!(asked.approval(), Ok(()));
    assert_eq!(
        compare_approved(&asked, Some(&folder)).approval(&asked),
        Err(ApprovalRefusal::ServicePorts)
    );
}
