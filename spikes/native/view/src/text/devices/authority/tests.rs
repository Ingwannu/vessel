//! Full sentences are asserted in both languages, independently of peer prose.
use super::*;
use crate::{Lang, localization::Locale};

#[test]
fn failed_reads_say_what_happened_without_inventing_a_denied_grant() {
    for (tag, lang, unsupported, refused, unavailable, changed, invalid) in [
        (
            "en",
            Lang::En,
            "This machine cannot report connection permissions. Update Vessel on that machine and reconnect.",
            "Could not check this connection’s access. Reconnect to this machine and try again.",
            "Could not check this connection’s access right now. Try again.",
            "Could not check access because this machine’s connection changed. Check access again.",
            "Could not check access because the machine returned an invalid response. Try again.",
        ),
        (
            "ko",
            Lang::Ko,
            "이 머신은 연결의 권한을 알려 줄 수 없습니다. 머신의 Vessel을 업데이트한 뒤 다시 연결하세요.",
            "이 연결의 권한을 확인하지 못했습니다. 머신에 다시 연결한 뒤 시도하세요.",
            "지금은 이 연결의 권한을 확인할 수 없습니다. 다시 시도하세요.",
            "머신의 연결이 바뀌어 권한을 확인하지 못했습니다. 권한을 다시 확인하세요.",
            "머신의 응답이 올바르지 않아 권한을 확인하지 못했습니다. 다시 시도하세요.",
        ),
    ] {
        let t = Translator::get(Locale::from_tag(tag).unwrap()).unwrap();
        for (why, expected) in [
            (Problem::Refused(ErrorCode::Unsupported), unsupported),
            (Problem::Refused(ErrorCode::ProtocolMismatch), unsupported),
            (Problem::Refused(ErrorCode::Forbidden), refused),
            (Problem::Refused(ErrorCode::Unauthorized), refused),
            (Problem::Refused(ErrorCode::Unavailable), unavailable),
            (Problem::Refused(ErrorCode::Internal), unavailable),
            (Problem::Refused(ErrorCode::InvalidRequest), unavailable),
            (Problem::Refused(ErrorCode::NotFound), unavailable),
            (Problem::Refused(ErrorCode::Conflict), unavailable),
            (
                Problem::Refused(ErrorCode::HarnessNotInstalled),
                unavailable,
            ),
            (Problem::Refused(ErrorCode::SpawnFailed), unavailable),
            (Problem::NoRoute, invalid),
            (Problem::TargetChanged, changed),
            (Problem::InvalidReply, invalid),
        ] {
            assert_eq!(lang.devices().authority_problem(why, t), expected);
        }
    }
}

#[test]
fn a_known_denial_and_an_unconfirmed_control_have_different_whole_copy() {
    for (tag, denied, unconfirmed) in [
        (
            "en",
            "This connection cannot manage paired devices. Use an administrator connection.",
            "Management access is not confirmed for this connection. Check access before trying again.",
        ),
        (
            "ko",
            "이 연결에는 페어링된 기기를 관리할 권한이 없습니다. 관리자 연결을 사용하세요.",
            "이 연결의 관리 권한이 확인되지 않았습니다. 권한을 확인한 뒤 다시 시도하세요.",
        ),
    ] {
        let t = Translator::get(Locale::from_tag(tag).unwrap()).unwrap();
        assert_eq!(t.device_authority().denied(), denied);
        assert_eq!(t.device_authority().not_confirmed(), unconfirmed);
        assert_ne!(denied, unconfirmed);
    }
}

#[test]
fn local_failures_and_loading_controls_have_complete_copy_in_both_languages() {
    for (tag, lang, offline, exhausted, unconfirmed, loading, retry, management) in [
        (
            "en",
            Lang::En,
            "Could not check access because this machine disconnected. Reconnect and check access again.",
            "Could not check access safely. Reopen the app and try again.",
            "Management access is not confirmed for this connection. Check access before trying again.",
            "Checking this connection’s access",
            "Check access",
            "Device management",
        ),
        (
            "ko",
            Lang::Ko,
            "머신과의 연결이 끊어져 권한을 확인하지 못했습니다. 다시 연결한 뒤 권한을 확인하세요.",
            "권한을 안전하게 확인할 수 없습니다. 앱을 다시 열고 시도하세요.",
            "이 연결의 관리 권한이 확인되지 않았습니다. 권한을 확인한 뒤 다시 시도하세요.",
            "이 연결의 권한 확인 중",
            "권한 확인",
            "기기 관리",
        ),
    ] {
        let t = Translator::get(Locale::from_tag(tag).unwrap()).unwrap();
        for (why, expected) in [
            (Problem::Offline, offline),
            (Problem::Exhausted, exhausted),
            (Problem::AccessNotConfirmed, unconfirmed),
        ] {
            assert_eq!(lang.devices().authority_problem(why, t), expected);
        }
        assert_eq!(t.device_authority().loading(), loading);
        assert_eq!(t.device_authority().retry(), retry);
        assert_eq!(t.device_authority().management(), management);
    }
}

#[test]
fn interrupted_writes_do_not_claim_a_definite_failure_in_either_language() {
    use crate::devices::Unconfirmed;
    for (tag, lang, title, invitation, revocation, both) in [
        (
            "en",
            Lang::En,
            "Result unconfirmed",
            "The connection changed while creating a pairing link. A link may have been created, but it cannot be shown safely. Reconnect and create a fresh link when needed.",
            "The connection changed while revoking access. Access may already have been revoked. Reconnect and check the paired devices before trying again.",
            "The connection changed while creating a pairing link and revoking access. Either change may already have happened. Reconnect and check the paired devices before trying again; create a fresh link when needed.",
        ),
        (
            "ko",
            Lang::Ko,
            "결과 확인 필요",
            "페어링 링크를 만드는 동안 연결이 바뀌었습니다. 링크가 만들어졌을 수 있지만 안전하게 표시할 수 없습니다. 다시 연결한 뒤 필요하면 새 링크를 만드세요.",
            "접근을 해제하는 동안 연결이 바뀌었습니다. 이미 해제되었을 수 있습니다. 다시 연결해 페어링된 기기를 확인한 뒤 재시도하세요.",
            "페어링 링크 생성과 접근 해제 중에 연결이 바뀌었습니다. 두 작업 모두 이미 처리되었을 수 있습니다. 다시 연결해 페어링된 기기를 확인한 뒤 재시도하고, 필요하면 새 링크를 만드세요.",
        ),
    ] {
        let t = Translator::get(Locale::from_tag(tag).unwrap()).unwrap();
        assert_eq!(t.device_authority().uncertain_title(), title);
        for (operation, expected) in [
            (Unconfirmed::Invitation, invitation),
            (Unconfirmed::Revocation, revocation),
            (Unconfirmed::Both, both),
        ] {
            assert_eq!(lang.devices().unconfirmed(operation, t), expected);
            assert_eq!(operation.notice(), crate::alert::Variant::Warning);
        }
    }
}
