//! An in-memory boundary has no credential, process, or vendor access.
use super::*;
use vessel_app_view::usage::{Account, Phase, Reader, configured_accounts};

#[test]
fn queue_refusals_preserve_the_exact_inventory_lifetime() {
    let mut state: Reader<ConnectionContext, Vec<Account>> = Reader::default();
    let machine = vessel_app_view::connection::ConnectionEpochs::default()
        .admit(
            MachineKey::from_book("test-machine"),
            vessel_proto::NodeId::random(),
        )
        .unwrap();
    let request = state.request(machine.clone()).unwrap();
    let expected = request.ticket;
    let Answer::Accounts {
        surface,
        request,
        result,
    } = Ask::Accounts {
        surface: InventorySurface::Accounts,
        request,
        refresh: InventoryRefresh::Cached,
    }
    .refused(RequestFailure::Busy)
    else {
        panic!("inventory reply")
    };
    assert_eq!(surface, InventorySurface::Accounts);
    assert_eq!(request.ticket, expected);
    assert_eq!(request.selection, machine);
    assert!(result.is_err());
    assert!(
        state
            .answer(request.ticket, Err(RequestFailure::Busy))
            .is_none()
    );
    assert!(matches!(
        state.phase(),
        Phase::Failed {
            reason: RequestFailure::Busy,
            ..
        }
    ));
    assert!(configured_accounts(&[]).unwrap().is_empty());
}
