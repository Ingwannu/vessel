//! Queued write results cannot consume owner tickets after physical retirement.
use super::*;
use vessel_proto::{Call, Mark, Outcome, Reply, conversation::MarkSession};

#[tokio::test]
async fn a_real_unary_archive_reply_queued_before_retirement_cannot_settle_the_ticket() {
    within(async {
        let mut fixture = fixture(true).await;
        let scope = Scope::capture(&fixture.selected, &fixture.context, fixture.binding).unwrap();
        let operation = Operation {
            kind: OperationKind::Lifecycle,
            ticket: fixture.residents.admit_operation(fixture.binding, OperationKind::Lifecycle).unwrap(),
        };
        let session = fixture.binding.resident.address.session;
        let info = {
            let request = fixture.selected.call(MarkSession {session, mark: Mark::Archive});
            tokio::pin!(request);
            let received = phase("observe actual archive request", async {
                tokio::select! {
                    result = &mut request => panic!("archive completed before reply: {}", result.is_ok()),
                    received = fixture.peer.requests.as_mut().unwrap().recv() => received.unwrap(),
                }
            }).await;
            assert!(matches!(received.call, Call::MarkSession(ask) if ask.session == session && ask.mark == Mark::Archive));
            let mut marked = peer::info(session);
            marked.marks.archived_at = Some(chrono::DateTime::UNIX_EPOCH);
            received.answer.send(Outcome::Ok(Reply::MarkSession(marked))).unwrap();
            phase("decode original archive reply", request).await.unwrap()
        };
        let (sender, mut receiving) = tokio::sync::mpsc::channel(1);
        phase("queue actual original archive result", fixture.selected.send_guarded(
            &sender,
            scope.envelope(Payload::Completion {operation, result: Box::new(Ok(Acted::Session(Box::new(info))))}),
        )).await.unwrap();
        fixture.state.clients["fixture"].retire();
        let envelope = phase("receive actual queued archive result", receiving.recv()).await.unwrap();
        assert_eq!(envelope.operation(), Some(operation));
        assert_eq!(envelope.install(|_, _| panic!("retired archive result reached mutation")), Err(Problem::Delivery(Refusal::Changed)));
        let retired = fixture.residents.retire_current(fixture.binding.resident).unwrap().unwrap();
        assert_eq!(retired.binding(), fixture.binding);
        let [lost] = retired.operations() else {panic!("original pending operation was not retained")};
        assert_eq!(lost.kind(), operation.kind);
        assert_eq!(lost.ticket(), operation.ticket);
        assert!(fixture.residents.settle_operation(fixture.binding, operation.kind, operation.ticket).is_err());
    }).await;
}

#[tokio::test]
async fn canonical_operation_kind_and_single_use_ticket_precede_completion() {
    within(async {
        let mut fixture = fixture(false).await;
        let scope = Scope::capture(&fixture.selected, &fixture.context, fixture.binding).unwrap();
        let operation = Operation {
            kind: OperationKind::Lifecycle,
            ticket: fixture
                .residents
                .admit_operation(fixture.binding, OperationKind::Lifecycle)
                .unwrap(),
        };
        let mut count = 0;
        let completion = || {
            scope.envelope(Payload::Completion {
                operation,
                result: Box::new(Ok(Acted::Done)),
            })
        };
        assert_eq!(completion().operation(), Some(operation));
        install(&mut fixture.residents, completion(), &mut count).unwrap();
        assert!(matches!(
            install(&mut fixture.residents, completion(), &mut count),
            Err(Problem::Resident(_))
        ));
        assert_eq!(count, 1);
        let wrong = scope.envelope(Payload::Completion {
            operation: Operation {
                kind: OperationKind::Composer,
                ..operation
            },
            result: Box::new(Ok(Acted::Done)),
        });
        assert!(matches!(
            install(&mut fixture.residents, wrong, &mut count),
            Err(Problem::Resident(_))
        ));
        assert_eq!(count, 1);
    })
    .await;
}

#[tokio::test]
async fn guardless_refusals_downgrade_only_the_original_pending_admission() {
    within(async {
        let mut fixture = fixture(false).await;
        let operation = Operation {
            kind: OperationKind::Lifecycle,
            ticket: fixture
                .residents
                .admit_operation(fixture.binding, OperationKind::Lifecycle)
                .unwrap(),
        };
        CorrelatedRefusal::new(
            fixture.context.clone(),
            fixture.binding,
            Some(operation),
            FailurePhase::NotSent,
            Refusal::Changed,
        )
        .apply(|context, binding, admission, failure_phase, reason| {
            assert_eq!(context, &fixture.context);
            assert_eq!(failure_phase, FailurePhase::NotSent);
            assert_eq!(reason, Refusal::Changed);
            let admission = admission.unwrap();
            fixture
                .residents
                .settle_operation(binding, admission.kind, admission.ticket)
        })
        .unwrap();
        let old = fixture.binding;
        let fresh = fixture.residents.begin_follow(old.resident).unwrap();
        let successor = fixture
            .residents
            .admit_operation(fresh, OperationKind::Lifecycle)
            .unwrap();
        let refusal = CorrelatedRefusal::new(
            fixture.context.clone(),
            old,
            Some(operation),
            FailurePhase::Uncertain,
            Refusal::Closed,
        );
        assert!(
            refusal
                .apply(|_, binding, _, _, _| {
                    fixture.residents.validate_binding(binding)?;
                    let _retired = fixture.residents.retire_current(binding.resident)?;
                    Ok(())
                })
                .is_err()
        );
        assert!(fixture.residents.validate_binding(fresh).is_ok());
        fixture
            .residents
            .settle_operation(fresh, OperationKind::Lifecycle, successor)
            .unwrap();
    })
    .await;
}
