//! Reference decision cases; no installed provider or credential is consulted.

use super::*;
use crate::providers::builtin_manifest;
use vessel_proto::providers::CreateParent;

fn input(id: &str) -> CreateInput {
    let provider: ProviderId = id.parse().unwrap();
    let available = builtin_manifest()
        .unwrap()
        .into_iter()
        .find(|b| b.id == provider)
        .map(|b| b.modes);
    CreateInput {
        provider,
        mode: None,
        auto_accept: None,
        parent: None,
        approval: ApprovalFlow::Interactive,
        available,
    }
}

fn parent(id: &str, mode: Option<&str>, approval: ApprovalFlow) -> CreateParent {
    CreateParent {
        provider: id.parse().unwrap(),
        mode: mode.map(|m| m.parse().unwrap()),
        approval,
    }
}

#[test]
fn an_explicit_mode_wins_and_unknown_differs_from_no_modes() {
    let mut request = input("codex");
    request.parent = Some(parent(
        "claude",
        Some("bypassPermissions"),
        ApprovalFlow::Unattended,
    ));
    request.mode = Some("auto".parse().unwrap());
    let resolved = resolve_create(CreatePolicy::Default, &request).unwrap();
    assert_eq!(resolved.mode, request.mode);
    assert_eq!(resolved.approval, ApprovalFlow::Interactive);
    request.mode = Some("unknown".parse().unwrap());
    assert!(matches!(
        resolve_create(CreatePolicy::Default, &request),
        Err(CreateError::InvalidMode { .. })
    ));
    request.available = None;
    assert_eq!(
        resolve_create(CreatePolicy::Default, &request)
            .unwrap()
            .mode,
        request.mode
    );
    request.available = Some(ModeCatalogue::new(vec![], None).unwrap());
    assert!(matches!(
        resolve_create(CreatePolicy::Default, &request),
        Err(CreateError::InvalidMode { .. })
    ));
}

#[test]
fn root_defaults_are_unset_and_unattended_roots_use_the_target_marker() {
    for built in builtin_manifest().unwrap() {
        let mut request = input(built.id.as_str());
        assert_eq!(
            resolve_create(CreatePolicy::Default, &request)
                .unwrap()
                .mode,
            None
        );
        request.approval = ApprovalFlow::Unattended;
        let expected = built
            .modes
            .modes()
            .iter()
            .find(|m| m.approval == ApprovalFlow::Unattended)
            .map(|m| m.id.clone());
        assert_eq!(
            resolve_create(CreatePolicy::Default, &request)
                .unwrap()
                .mode,
            expected
        );
    }
}

#[test]
fn same_provider_inherits_even_an_absent_or_no_longer_catalogued_mode() {
    let mut request = input("claude");
    for mode in [None, Some("acceptEdits"), Some("retired-mode")] {
        request.parent = Some(parent("claude", mode, ApprovalFlow::Interactive));
        let result = resolve_create(CreatePolicy::Default, &request).unwrap();
        assert_eq!(result.mode, mode.map(|m| m.parse().unwrap()));
    }
    // Two aliases of one implementation do not share an identity.
    request.parent = Some(parent(
        "claude-alt",
        Some("default"),
        ApprovalFlow::Interactive,
    ));
    assert!(matches!(
        resolve_create(CreatePolicy::Default, &request),
        Err(CreateError::CannotInherit { .. })
    ));
}

#[test]
fn cross_provider_bridges_unattended_only_and_empty_modes_choose_default() {
    let mut request = input("codex");
    request.parent = Some(parent(
        "claude",
        Some("bypassPermissions"),
        ApprovalFlow::Unattended,
    ));
    assert_eq!(
        resolve_create(CreatePolicy::Default, &request)
            .unwrap()
            .mode,
        Some("full-access".parse().unwrap())
    );
    request.parent = Some(parent("claude", Some("plan"), ApprovalFlow::Interactive));
    assert!(matches!(
        resolve_create(CreatePolicy::Default, &request),
        Err(CreateError::CannotInherit { .. })
    ));
    request.available = None;
    assert!(matches!(
        resolve_create(CreatePolicy::Default, &request),
        Err(CreateError::CannotInherit { .. })
    ));
    request.available = Some(ModeCatalogue::new(vec![], None).unwrap());
    assert_eq!(
        resolve_create(CreatePolicy::Default, &request)
            .unwrap()
            .mode,
        None
    );
}

#[test]
fn opencode_unattended_does_not_invent_a_build_agent_and_preserves_explicit_off() {
    let mut request = input("opencode");
    request.available = Some(ModeCatalogue::new(vec![], None).unwrap());
    request.parent = Some(parent(
        "claude",
        Some("bypassPermissions"),
        ApprovalFlow::Unattended,
    ));
    let result = resolve_create(CreatePolicy::OpenCode, &request).unwrap();
    assert_eq!(result.mode, None);
    assert_eq!(result.auto_accept, Some(AutoAccept::On));
    assert_eq!(result.approval, ApprovalFlow::Unattended);
    request.auto_accept = Some(AutoAccept::Off);
    let result = resolve_create(CreatePolicy::OpenCode, &request).unwrap();
    assert_eq!(result.auto_accept, Some(AutoAccept::Off));
    assert_eq!(result.approval, ApprovalFlow::Interactive);
}

#[test]
fn opencode_legacy_is_explicit_validated_build_plus_auto_accept() {
    let mut request = input("opencode");
    request.mode = Some("full-access".parse().unwrap());
    request.auto_accept = Some(AutoAccept::Off);
    let result = resolve_create(CreatePolicy::OpenCode, &request).unwrap();
    assert_eq!(result.mode, Some("build".parse().unwrap()));
    assert_eq!(result.auto_accept, Some(AutoAccept::On));
    request.available = Some(ModeCatalogue::new(vec![], None).unwrap());
    assert!(matches!(
        resolve_create(CreatePolicy::OpenCode, &request),
        Err(CreateError::InvalidMode { .. })
    ));
}

#[test]
fn opencode_same_provider_unattended_mode_is_validated_as_an_explicit_choice() {
    let mut request = input("opencode");
    request.parent = Some(parent("opencode", Some("plan"), ApprovalFlow::Unattended));
    assert_eq!(
        resolve_create(CreatePolicy::OpenCode, &request)
            .unwrap()
            .mode,
        Some("plan".parse().unwrap())
    );
    request.parent = Some(parent(
        "opencode",
        Some("removed-agent"),
        ApprovalFlow::Unattended,
    ));
    assert!(matches!(
        resolve_create(CreatePolicy::OpenCode, &request),
        Err(CreateError::InvalidMode { .. })
    ));
}

#[test]
fn acp_uses_auto_accept_but_keeps_explicit_choices_and_same_provider_modes() {
    let mut request = input("custom-acp");
    request.parent = Some(parent(
        "claude",
        Some("bypassPermissions"),
        ApprovalFlow::Unattended,
    ));
    let result = resolve_create(CreatePolicy::Acp, &request).unwrap();
    assert_eq!(result.mode, None);
    assert_eq!(result.auto_accept, Some(AutoAccept::On));
    assert_eq!(result.approval, ApprovalFlow::Unattended);
    request.mode = Some("native-agent".parse().unwrap());
    assert_eq!(
        resolve_create(CreatePolicy::Acp, &request).unwrap().mode,
        request.mode
    );
    request.mode = None;
    request.auto_accept = Some(AutoAccept::Off);
    request.parent = Some(parent(
        "custom-acp",
        Some("native-agent"),
        ApprovalFlow::Unattended,
    ));
    let result = resolve_create(CreatePolicy::Acp, &request).unwrap();
    assert_eq!(result.mode, request.parent.unwrap().mode);
    assert_eq!(result.auto_accept, Some(AutoAccept::Off));
}

#[test]
fn default_policy_does_not_treat_an_unrelated_auto_accept_feature_as_authority() {
    let mut request = input("claude");
    request.auto_accept = Some(AutoAccept::On);
    assert_eq!(
        resolve_create(CreatePolicy::Default, &request)
            .unwrap()
            .approval,
        ApprovalFlow::Interactive
    );
}
