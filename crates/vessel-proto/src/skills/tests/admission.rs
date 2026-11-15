use crate::skills::*;
use crate::{Call, Forward, Method, MethodShape, NodeId, Request as _, Scope};

#[test]
fn skill_management_is_pending_admin_local_and_unary() {
    let calls = [
        GetSkillsStatus {}.into_call(),
        ReconcileSkills {}.into_call(),
        UninstallSkills {}.into_call(),
        SaveSkillsSelection {
            selection: SkillSelection::All,
            confirmed_removals: SkillNames::default(),
        }
        .into_call(),
        ImportSkillsSelection {
            selection: SkillSelection::All,
        }
        .into_call(),
    ];
    for call in calls {
        let method = call.method();
        // This runtime tree supplies real handlers; the portable carrier stays staged.
        assert!(!method.is_staged());
        assert_eq!(method.scope(), Some(Scope::Admin));
        assert_eq!(method.since(), crate::mcp::MCP_PARITY);
        assert!(!method.forwardable());
        assert_eq!(method.shape(), MethodShape::Unary);
        assert_eq!(call.since(), crate::mcp::MCP_PARITY);
        let forwarded = Forward {
            node: NodeId::random(),
            call: Box::new(call.clone()),
        }
        .into_call();
        assert_eq!(forwarded.since(), crate::mcp::MCP_PARITY);
        let encoded = serde_json::to_value(&call).unwrap();
        assert_eq!(serde_json::from_value::<Call>(encoded).unwrap(), call);
    }
    assert!(Method::GetSkillsStatus.as_str().starts_with("skills."));
}
