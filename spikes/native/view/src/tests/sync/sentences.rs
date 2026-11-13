//! Complete visible sentences, badge meanings and command guidance.
use super::{member, now};
use crate::{Lang, badge::Variant, devices::time::Age, sync::*};
use chrono::{Duration, FixedOffset};
use vessel_proto::{FleetMember, FleetNode, Generation, ProtocolVersion};

#[test]
fn member_history_says_last_report_not_live_connection_or_last_sync_in_both_languages() {
    let offset = FixedOffset::east_opt(9 * 3600).unwrap();
    assert_eq!(
        member_details(
            Lang::En,
            &member(),
            Some(MemberState::Behind(2)),
            now(),
            offset
        ),
        "2 versions behind the home.\nApplied version 3.\nNo local edits kept.\nLast reported 1 minute ago."
    );
    assert_eq!(
        member_details(
            Lang::Ko,
            &member(),
            Some(MemberState::Behind(2)),
            now(),
            offset
        ),
        "홈 머신보다 2개 이전 버전입니다.\n적용한 버전: 3.\n유지된 로컬 수정이 없습니다.\n마지막 보고: 1분 전."
    );
    let edited = FleetMember {
        conflicts: vec!["AGENTS.md".parse().unwrap()],
        ..member()
    };
    assert_eq!(
        member_details(
            Lang::En,
            &edited,
            Some(MemberState::UpToDate),
            now(),
            offset
        ),
        "Applied version 3.\nKept a local edit in 1 file.\nLast reported 1 minute ago."
    );
    assert_eq!(
        member_details(
            Lang::Ko,
            &edited,
            Some(MemberState::UpToDate),
            now(),
            offset
        ),
        "적용한 버전: 3.\n파일 1개의 로컬 수정을 유지했습니다.\n마지막 보고: 1분 전."
    );
    let never = FleetMember {
        applied: None,
        reported_at: None,
        ..member()
    };
    assert_eq!(
        member_details(Lang::En, &never, Some(MemberState::NotYet), now(), offset),
        "No applied version reported.\nNo local edits kept.\nNo report time received."
    );
    assert_eq!(
        member_details(Lang::Ko, &never, Some(MemberState::NotYet), now(), offset),
        "적용한 버전의 보고가 없습니다.\n유지된 로컬 수정이 없습니다.\n보고받은 시각이 없습니다."
    );
    assert_eq!(
        member_details(Lang::En, &member(), None, now(), offset),
        "The current shared version could not be checked.\nApplied version 3.\nNo local edits kept.\nLast reported 1 minute ago."
    );
    assert_eq!(
        member_details(Lang::Ko, &member(), None, now(), offset),
        "현재 공유 버전을 확인하지 못했습니다.\n적용한 버전: 3.\n유지된 로컬 수정이 없습니다.\n마지막 보고: 1분 전."
    );
}

#[test]
fn report_ages_and_exact_times_are_whole_sentences_with_local_dates() {
    for (age, en, ko) in [
        (
            None,
            "No report time received.",
            "보고받은 시각이 없습니다.",
        ),
        (
            Some(Age::Now),
            "Last reported just now.",
            "마지막 보고: 방금.",
        ),
        (
            Some(Age::Minutes(1)),
            "Last reported 1 minute ago.",
            "마지막 보고: 1분 전.",
        ),
        (
            Some(Age::Minutes(7)),
            "Last reported 7 minutes ago.",
            "마지막 보고: 7분 전.",
        ),
        (
            Some(Age::Hours(1)),
            "Last reported 1 hour ago.",
            "마지막 보고: 1시간 전.",
        ),
        (
            Some(Age::Hours(2)),
            "Last reported 2 hours ago.",
            "마지막 보고: 2시간 전.",
        ),
        (
            Some(Age::Days(1)),
            "Last reported 1 day ago.",
            "마지막 보고: 1일 전.",
        ),
        (
            Some(Age::Days(4)),
            "Last reported 4 days ago.",
            "마지막 보고: 4일 전.",
        ),
    ] {
        assert_eq!(Lang::En.sync().reported(age), en);
        assert_eq!(Lang::Ko.sync().reported(age), ko);
    }
    let offset = FixedOffset::east_opt(9 * 3600).unwrap();
    let at = "2026-11-01T20:30:40.999999999Z".parse().unwrap();
    let age = Some(Age::of(at, at + Duration::days(8), offset));
    assert_eq!(Lang::En.sync().reported(age), "Last reported on Nov 2.");
    assert_eq!(Lang::Ko.sync().reported(age), "마지막 보고: 11월 2일.");
    assert_eq!(
        Lang::En.sync().exact_report(Some(at), offset),
        "Report: 2026-11-02 05:30:40 (UTC+9)"
    );
    assert_eq!(
        Lang::Ko.sync().exact_report(Some(at), offset),
        "보고: 2026-11-02 05:30:40 (UTC+9)"
    );
    assert_eq!(
        Lang::En.sync().exact_report(None, offset),
        "No report time received."
    );
    assert_eq!(
        Lang::Ko.sync().exact_report(None, offset),
        "보고받은 시각이 없습니다."
    );
}

#[test]
fn badges_are_short_and_counts_are_explained_as_numeric_sentences_beneath() {
    for (state, en, ko, variant) in [
        (
            Some(MemberState::UpToDate),
            "Up to date",
            "최신",
            Variant::Success,
        ),
        (
            Some(MemberState::Behind(2)),
            "Behind",
            "이전 버전",
            Variant::Warning,
        ),
        (
            Some(MemberState::NotYet),
            "Not reported",
            "보고 전",
            Variant::Muted,
        ),
        (None, "Not checked", "확인 안 됨", Variant::Muted),
    ] {
        assert_eq!(Lang::En.sync().member_state(state), en);
        assert_eq!(Lang::Ko.sync().member_state(state), ko);
        assert_eq!(member_variant(state), variant);
    }
    for (n, en, ko) in [
        (0, "No local edits kept.", "유지된 로컬 수정이 없습니다."),
        (
            1,
            "Kept a local edit in 1 file.",
            "파일 1개의 로컬 수정을 유지했습니다.",
        ),
        (
            2,
            "Kept local edits in 2 files.",
            "파일 2개의 로컬 수정을 유지했습니다.",
        ),
    ] {
        assert_eq!(Lang::En.sync().kept(n), en);
        assert_eq!(Lang::Ko.sync().kept(n), ko);
    }
    assert_eq!(
        Lang::En.sync().comparison(Some(MemberState::Behind(1))),
        Some("1 version behind the home.".into())
    );
    assert_eq!(
        Lang::Ko.sync().comparison(Some(MemberState::Behind(1))),
        Some("홈 머신보다 1개 이전 버전입니다.".into())
    );
    assert_eq!(
        Lang::En.sync().comparison(Some(MemberState::UpToDate)),
        None
    );
    assert_eq!(Lang::Ko.sync().comparison(Some(MemberState::NotYet)), None);
}

#[test]
fn secret_presence_never_speaks_of_a_value_and_local_only_differs_from_unknown_usage() {
    for (presence, en, ko, variant) in [
        (Presence::Present, "Present", "저장됨", Variant::Success),
        (Presence::Missing, "Missing", "없음", Variant::Warning),
        (
            Presence::Unknown,
            "Not checked",
            "확인 안 됨",
            Variant::Muted,
        ),
    ] {
        assert_eq!(Lang::En.sync().presence(presence), en);
        assert_eq!(Lang::Ko.sync().presence(presence), ko);
        assert_eq!(presence.variant(), variant);
    }
    for (usage, en, ko) in [
        (
            Usage::Shared,
            "Named by shared servers",
            "공유 서버가 사용하는 이름",
        ),
        (
            Usage::LocalOnly,
            "Stored on this machine",
            "이 머신에 저장된 이름",
        ),
        (
            Usage::Unreported,
            "Usage not checked",
            "사용 여부 확인 안 됨",
        ),
    ] {
        assert_eq!(Lang::En.sync().usage(usage), en);
        assert_eq!(Lang::Ko.sync().usage(usage), ko);
    }
    assert_eq!(
        Lang::En.sync().secret_hint,
        "Only names and presence are shown. Values stay on each machine."
    );
    assert_eq!(
        Lang::Ko.sync().secret_hint,
        "이름과 저장 여부만 표시합니다. 값은 각 머신에 남습니다."
    );
}

#[test]
fn file_sizes_modes_and_connection_versions_are_readable_values() {
    let published = Published::project(super::projection::manifest());
    let mut files = published.files.into_iter();
    let instructions = files.next().unwrap();
    let skill = files.next().unwrap();
    assert_eq!(
        Lang::En
            .sync()
            .file_metadata(instructions.size, instructions.mode),
        "Size: 2.0 KB."
    );
    assert_eq!(
        Lang::Ko
            .sync()
            .file_metadata(instructions.size, instructions.mode),
        "크기: 2.0 KB."
    );
    assert_eq!(
        Lang::En.sync().file_metadata(skill.size, skill.mode),
        "Size: 3.0 MB. This file can run as a program."
    );
    assert_eq!(
        Lang::Ko.sync().file_metadata(skill.size, skill.mode),
        "크기: 3.0 MB. 실행할 수 있는 파일입니다."
    );
    for (kind, en, ko) in [
        (FileKind::Instructions, "Instructions", "지침"),
        (FileKind::Skill, "Skill file", "스킬 파일"),
        (FileKind::Other, "Shared file", "공유 파일"),
    ] {
        assert_eq!(Lang::En.sync().file_kind(kind), en);
        assert_eq!(Lang::Ko.sync().file_kind(kind), ko);
    }
    let mut node = FleetNode {
        node: "00000000-0000-4000-8000-000000000099".parse().unwrap(),
        name: "home".into(),
        protocol: ProtocolVersion::new(6),
        reachable: true,
    };
    assert_eq!(
        Lang::En.sync().connection_details(&node),
        "This machine can be reached. Connection version 6."
    );
    assert_eq!(
        Lang::Ko.sync().connection_details(&node),
        "연결할 수 있는 머신입니다. 연결 버전: 6."
    );
    node.reachable = false;
    assert_eq!(
        Lang::En.sync().connection_details(&node),
        "This machine cannot be reached. Connection version 6."
    );
    assert_eq!(
        Lang::Ko.sync().connection_details(&node),
        "연결할 수 없는 머신입니다. 연결 버전: 6."
    );
    for (reachable, en, ko) in [
        (true, "Reachable", "연결 가능"),
        (false, "Unreachable", "연결 불가"),
    ] {
        assert_eq!(Lang::En.sync().connection_state(reachable), en);
        assert_eq!(Lang::Ko.sync().connection_state(reachable), ko);
    }
    assert_eq!(
        Lang::En.sync().version(Generation::new(5)),
        "Shared version 5"
    );
    assert_eq!(Lang::Ko.sync().version(Generation::new(5)), "공유 버전 5");
}

#[test]
fn local_guidance_names_where_to_run_a_real_command_without_drawing_a_remote_action() {
    assert_eq!(Lang::En.sync().on_machine("laptop"), "Sync on laptop");
    assert_eq!(Lang::Ko.sync().on_machine("연구실"), "연구실의 동기화");
    for (command, en, ko) in [
        (
            LocalCommand::Init,
            "Make a home on the machine that should share its files.",
            "파일을 공유할 머신에서 홈을 만드세요.",
        ),
        (
            LocalCommand::Invite,
            "Create a fleet invitation on the home. Treat its link like a password.",
            "홈 머신에서 공유 초대를 만드세요. 초대 링크는 비밀번호처럼 다루세요.",
        ),
        (
            LocalCommand::Join,
            "Join on the receiving machine. Paste the home's fleet invitation at its prompt.",
            "파일을 받을 머신에서 가입하세요. 입력창에 홈 머신의 공유 초대 링크를 붙여 넣으세요.",
        ),
        (
            LocalCommand::Leave,
            "Leave on the member machine. Its shared files stay in place.",
            "구성원 머신에서 탈퇴하세요. 공유받은 파일은 그대로 남습니다.",
        ),
        (
            LocalCommand::Status,
            "Check the last applied version locally. Changes are pulled automatically while the node runs.",
            "머신에서 마지막으로 적용한 버전을 확인하세요. 노드가 실행되는 동안 변경 사항을 자동으로 가져옵니다.",
        ),
    ] {
        assert_eq!(Lang::En.sync().command(command), en);
        assert_eq!(Lang::Ko.sync().command(command), ko);
    }
}

#[test]
fn progress_failures_and_kept_edits_have_distinct_semantic_notices() {
    assert_eq!(LOADING, Variant::Muted);
    assert_eq!(KEPT_EDITS_NOTICE, crate::alert::Variant::Warning);
    assert_eq!(Problem::Offline.notice(), crate::alert::Variant::Error);
    assert_eq!(INCOMING_SUFFIX, ".vessel-incoming");
}
