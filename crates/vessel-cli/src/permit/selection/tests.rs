//! Resolve exact and ambiguous prefixes before any form lookup or mutation.
use super::*;
use crate::permit::pending::{PendingId, Row};
use vessel_proto::{PermissionId, SessionId};

fn row(session: SessionId, id: &str) -> Row {
    Row {
        id: PendingId::Permission(id.parse::<PermissionId>().unwrap()),
        agent_id: session,
        agent_short_id: session.to_string(),
        name: "Permission".into(),
        description: String::new(),
    }
}
#[test]
fn all_and_prefixes_stay_with_the_selected_agent() {
    let own = SessionId::random();
    let other = SessionId::random();
    let rows = vec![
        row(own, "10000000-0000-4000-8000-000000000001"),
        row(own, "10000000-0000-4000-8000-000000000002"),
        row(other, "20000000-0000-4000-8000-000000000001"),
    ];
    assert_eq!(select(&rows, own, &Scope::All).unwrap(), rows[..2]);
    let prefix = "1".parse::<IdPrefix>().unwrap();
    assert!(matches!(
        select(&rows, own, &Scope::One(&prefix)),
        Err(crate::error::CliError::AmbiguousMatch { matches: 2, .. })
    ));
    let prefix = rows[1].id.to_string().parse::<IdPrefix>().unwrap();
    assert_eq!(
        select(&rows, own, &Scope::One(&prefix)).unwrap(),
        vec![rows[1].clone()]
    );
    let prefix = "2".parse::<IdPrefix>().unwrap();
    assert!(matches!(
        select(&rows, own, &Scope::One(&prefix)),
        Err(crate::error::CliError::NoMatch { .. })
    ));
    assert!(select(&rows, SessionId::random(), &Scope::All).is_err());
}
