//! A grant is accepted only with its read ticket; transport absence never grants
//! or removes authority, and cannot clear an actual operation refusal.
use super::tests::{loaded, now, rebound, snapshot};
use super::*;
use crate::schedules::{Action, tests::info};
use vessel_proto::{ErrorCode, Scope, WireError};

#[test]
fn an_effective_self_grant_controls_current_and_new_rows() {
    for scopes in [vec![], vec![Scope::Read], vec![Scope::Read, Scope::Operate]] {
        let (mut page, row) = loaded();
        page.refresh();
        let request = page.due(now()).remove(0);
        let mut answer = snapshot(vec![info()]);
        answer.scopes = Ok(scopes.clone());
        page.listed(&request, Ok(answer));
        assert_eq!(
            page.row(&row).unwrap().authority,
            &Authority::Scopes(scopes.clone())
        );
        assert_eq!(
            page.row(&row).unwrap().actions().is_empty(),
            !scopes.contains(&Scope::Operate)
        );

        // A later failed self-query does not erase evidence for this connection,
        // including for a schedule first discovered by that later list.
        let mut other = info();
        other.id = "00000000-0000-4000-8000-000000000009".parse().unwrap();
        page.refresh();
        let request = page.due(now()).remove(0);
        page.listed(&request, Ok(snapshot(vec![info(), other.clone()])));
        let other = RowKey {
            schedule: other.id,
            ..row.clone()
        };
        assert_eq!(
            page.row(&other).unwrap().authority,
            &Authority::Scopes(scopes)
        );
    }
}

#[test]
fn a_self_grant_cannot_erase_a_resource_refusal_or_cross_a_reconnect() {
    let (mut page, row) = loaded();
    let pause = page.act(&row, Action::Pause).unwrap();
    page.written(
        &pause,
        Err(Failure::Node(WireError::new(
            ErrorCode::Forbidden,
            "diagnostic",
        ))),
    );
    page.refresh();
    let request = page.due(now()).remove(0);
    let mut answer = snapshot(vec![info()]);
    answer.scopes = Ok(vec![Scope::Read, Scope::Operate]);
    page.listed(&request, Ok(answer.clone()));
    assert_eq!(
        page.row(&row).unwrap().authority,
        &Authority::OperationForbidden
    );

    page.refresh();
    let old = page.due(now()).remove(0);
    page.connected(&row.machine);
    let _changed = page.admitted(rebound(&row.machine));
    assert_eq!(page.row(&row).unwrap().authority, &Authority::Unknown);
    answer.scopes = Ok(vec![Scope::Read]);
    page.listed(&old, Ok(answer));
    assert_eq!(page.row(&row).unwrap().authority, &Authority::Unknown);
    let fresh = page.due(now()).remove(0);
    page.listed(&fresh, Ok(snapshot(vec![info()])));
    assert_eq!(page.row(&row).unwrap().authority, &Authority::Unknown);
    assert!(!page.row(&row).unwrap().actions().is_empty());
}

#[test]
fn form_authority_is_scoped_to_the_original_admission_and_original_row() {
    let (mut page, row) = loaded();
    let connection = page.row(&row).unwrap().connection.unwrap().clone();
    assert_eq!(
        page.authority_for(&connection, None),
        Some(Authority::Unknown)
    );
    page.refresh();
    let request = page.due(now()).remove(0);
    let mut answer = snapshot(vec![info()]);
    answer.scopes = Ok(vec![Scope::Read, Scope::Operate]);
    page.listed(&request, Ok(answer));
    let pause = page.act(&row, Action::Pause).unwrap();
    page.written(
        &pause,
        Err(Failure::Node(WireError::new(
            ErrorCode::Forbidden,
            "diagnostic",
        ))),
    );
    assert_eq!(
        page.authority_for(&connection, Some(row.schedule)),
        Some(Authority::OperationForbidden)
    );
    assert_eq!(
        page.authority_for(&connection, None),
        Some(Authority::Scopes(vec![Scope::Read, Scope::Operate]))
    );
    let replacement = rebound(&row.machine);
    assert_eq!(page.authority_for(&replacement, None), None);
    page.connected(&row.machine);
    let _changed = page.admitted(replacement.clone());
    assert_eq!(page.authority_for(&connection, None), None);
    assert_eq!(
        page.authority_for(&replacement, None),
        Some(Authority::Unknown)
    );
}
