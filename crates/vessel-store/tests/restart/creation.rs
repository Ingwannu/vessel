//! Creation and its retained eligibility share one writer savepoint/commit.
use super::*;

#[tokio::test]
async fn failed_eligibility_insert_rolls_back_the_session_and_the_same_id_can_be_created() {
    case(async {
    let (_temp, dir) = data_dir();
    let store = fixtures::open(&dir, StoreConfig::default()).await;
    let session = info();
    let connection = rusqlite::Connection::open(dir.database_path()).unwrap();
    connection.execute_batch("CREATE TRIGGER reject_restart BEFORE INSERT ON restart_state BEGIN SELECT RAISE(ABORT,'synthetic projection failure'); END;").unwrap();
    assert!(
        store
            .create_session_with_restart(session.clone(), RestartEligibility::Person)
            .await
            .is_err()
    );
    assert!(store.session(session.id).await.unwrap().is_none());
    connection
        .execute_batch("DROP TRIGGER reject_restart;")
        .unwrap();
    store
        .create_session_with_restart(session.clone(), RestartEligibility::Person)
        .await
        .unwrap();
    assert_eq!(
        store.session(session.id).await.unwrap().unwrap().id,
        session.id
    );
    store.close().await;
    }).await;
}
