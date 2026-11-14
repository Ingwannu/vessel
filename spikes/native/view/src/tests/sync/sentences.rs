//! Complete visible sentences, badge meanings and command guidance.
use super::{member, now};
use crate::{Lang, badge::Variant, devices::time::Age, sync::*};
use chrono::{Duration, FixedOffset};
use vessel_proto::{FleetMember, FleetNode, Generation, ProtocolVersion};

#[test]
fn member_history_is_one_hint_and_never_repeats_the_conflict_notice() {
    let offset = FixedOffset::east_opt(9 * 3600).unwrap();
    for (state, en, ko, badge) in [
        (
            Some(MemberState::Behind(2)),
            "Version 3 · 2 behind home · reported 1 minute ago",
            "버전 3 · 홈보다 2개 이전 · 1분 전 보고",
            MemberBadge::Behind,
        ),
        (
            Some(MemberState::UpToDate),
            "Version 3 · reported 1 minute ago",
            "버전 3 · 1분 전 보고",
            MemberBadge::Current,
        ),
        (
            None,
            "Version 3 · reported 1 minute ago",
            "버전 3 · 1분 전 보고",
            MemberBadge::Unchecked,
        ),
    ] {
        for conflicts in [vec![], vec!["AGENTS.md".parse().unwrap()]] {
            let member = FleetMember {
                conflicts,
                ..member()
            };
            for (lang, expected) in [(Lang::En, en), (Lang::Ko, ko)] {
                assert_eq!(
                    MemberRow::of(lang, &member, state, now(), offset),
                    MemberRow {
                        hint: Some(expected.into()),
                        badge
                    }
                );
            }
        }
    }
}

#[test]
fn compact_report_ages_and_exact_times_keep_local_dates() {
    for (age, en, ko) in [
        (
            Age::Now,
            "Version 3 · reported just now",
            "버전 3 · 방금 보고",
        ),
        (
            Age::Minutes(1),
            "Version 3 · reported 1 minute ago",
            "버전 3 · 1분 전 보고",
        ),
        (
            Age::Minutes(7),
            "Version 3 · reported 7 minutes ago",
            "버전 3 · 7분 전 보고",
        ),
        (
            Age::Hours(1),
            "Version 3 · reported 1 hour ago",
            "버전 3 · 1시간 전 보고",
        ),
        (
            Age::Hours(2),
            "Version 3 · reported 2 hours ago",
            "버전 3 · 2시간 전 보고",
        ),
        (
            Age::Days(1),
            "Version 3 · reported 1 day ago",
            "버전 3 · 1일 전 보고",
        ),
        (
            Age::Days(4),
            "Version 3 · reported 4 days ago",
            "버전 3 · 4일 전 보고",
        ),
    ] {
        assert_eq!(
            Lang::En
                .sync()
                .member_hint(Some(Generation::new(3)), None, Some(age)),
            Some(en.into())
        );
        assert_eq!(
            Lang::Ko
                .sync()
                .member_hint(Some(Generation::new(3)), None, Some(age)),
            Some(ko.into())
        );
    }
    let offset = FixedOffset::east_opt(9 * 3600).unwrap();
    let at = "2026-11-01T20:30:40.999999999Z".parse().unwrap();
    let age = Some(Age::of(at, at + Duration::days(8), offset));
    assert_eq!(
        Lang::En
            .sync()
            .member_hint(Some(Generation::new(3)), None, age),
        Some("Version 3 · reported Nov 2".into())
    );
    assert_eq!(
        Lang::Ko
            .sync()
            .member_hint(Some(Generation::new(3)), None, age),
        Some("버전 3 · 11월 2일 보고".into())
    );
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
fn badges_are_short_and_partial_reports_do_not_claim_no_report() {
    for (badge, en, ko, variant) in [
        (MemberBadge::Current, "Up to date", "최신", Variant::Success),
        (MemberBadge::Behind, "Behind", "이전 버전", Variant::Warning),
        (
            MemberBadge::Unreported,
            "Not reported",
            "보고 전",
            Variant::Muted,
        ),
        (
            MemberBadge::VersionUnreported,
            "Version unknown",
            "버전 미보고",
            Variant::Muted,
        ),
        (
            MemberBadge::Unchecked,
            "Not checked",
            "확인 안 됨",
            Variant::Muted,
        ),
    ] {
        assert_eq!(Lang::En.sync().member_badge(badge), en);
        assert_eq!(Lang::Ko.sync().member_badge(badge), ko);
        assert_eq!(badge.variant(), variant);
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
        assert_eq!(Lang::En.sync().kept(KeptEdits::Known(n)), en);
        assert_eq!(Lang::Ko.sync().kept(KeptEdits::Known(n)), ko);
    }
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
    let hint = |lang: Lang, file: &SharedFile| {
        lang.sync()
            .file_hint(FileKind::of(&file.path), file.size, file.mode)
    };
    assert_eq!(hint(Lang::En, &instructions), "Instructions · 2.0 KB");
    assert_eq!(hint(Lang::Ko, &instructions), "지침 · 2.0 KB");
    assert_eq!(hint(Lang::En, &skill), "Skill · 3.0 MB · executable");
    assert_eq!(hint(Lang::Ko, &skill), "스킬 · 3.0 MB · 실행 가능");
    for (kind, en, ko) in [
        (FileKind::Instructions, "Instructions", "지침"),
        (FileKind::Skill, "Skill", "스킬"),
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
        Lang::En.sync().connection_version(node.protocol),
        "Connection version 6"
    );
    assert_eq!(
        Lang::Ko.sync().connection_version(node.protocol),
        "연결 버전 6"
    );
    node.reachable = false;
    assert_eq!(
        Lang::En.sync().connection_version(node.protocol),
        "Connection version 6"
    );
    assert_eq!(
        Lang::Ko.sync().connection_version(node.protocol),
        "연결 버전 6"
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
            "Make this machine a home to share its files.",
            "이 머신을 홈으로 만들어 파일을 공유하세요.",
        ),
        (
            LocalCommand::Invite,
            "On the home, create an invitation and keep its link private.",
            "홈에서 초대 링크를 만들고 비밀번호처럼 보관하세요.",
        ),
        (
            LocalCommand::Join,
            "Join on the machine receiving the shared files.",
            "공유받을 머신에서 가입하세요.",
        ),
        (
            LocalCommand::Leave,
            "Leave on the member machine; shared files stay in place.",
            "구성원 머신에서 탈퇴하세요. 공유 파일은 남습니다.",
        ),
        (
            LocalCommand::Status,
            "Check the last applied version on this machine.",
            "이 머신에서 마지막으로 적용한 버전을 확인하세요.",
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

#[test]
fn the_local_changes_notice_introduces_paths_then_explains_the_shared_copy_suffix() {
    assert_eq!(
        Lang::En.sync().changes_hint,
        "This member reported keeping local changes at these paths:"
    );
    assert_eq!(
        Lang::Ko.sync().changes_hint,
        "이 머신은 다음 경로의 로컬 변경을 유지했다고 보고했습니다:"
    );
    assert_eq!(
        Lang::En.sync().received_copies(),
        "Received shared copies were kept beside the originals with .vessel-incoming appended to their names."
    );
    assert_eq!(
        Lang::Ko.sync().received_copies(),
        "받은 공유본은 같은 자리에 이름 끝에 .vessel-incoming을 붙여 두었습니다."
    );
    assert_eq!(
        Lang::En.sync().home_unknown_hint,
        "The home provides shared files and tool-server names."
    );
    assert_eq!(
        Lang::Ko.sync().home_unknown_hint,
        "홈 머신이 공유 파일과 도구 서버 이름을 제공합니다."
    );
}

#[test]
fn a_version_report_without_a_time_keeps_its_comparison_on_one_line() {
    for (lang, expected) in [
        (Lang::En, "Version 3 · 2 behind home"),
        (Lang::Ko, "버전 3 · 홈보다 2개 이전"),
    ] {
        assert_eq!(
            lang.sync()
                .member_hint(Some(Generation::new(3)), Some(MemberState::Behind(2)), None),
            Some(expected.into())
        );
    }
}
