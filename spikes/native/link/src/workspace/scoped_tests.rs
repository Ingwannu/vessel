//! Exercise the actual link actor and framed peer, without a live node.
use super::FileAsk;
use super::scoped::*;
use crate::connections::Connected;
use crate::testing::{Home, peer, phase, state, within};
use vessel_app_view::connection::{ConnectionEpochs, MachineKey};
use vessel_app_view::workspace::file_ticket::FileTickets;
use vessel_app_view::workspace::{file_scope::FileScope, problem::Problem};
use vessel_proto::{
    NodeId, ProjectId,
    workspace::{ListDir, RelPath},
};
use vessel_proto::{SessionId, workspace::WorkspaceRoot};

fn scope() -> FileScope {
    let context = ConnectionEpochs::default()
        .admit(MachineKey::from_book("fixture"), NodeId::random())
        .unwrap();
    FileScope::new(
        context,
        WorkspaceRoot::Project {
            project: ProjectId::random(),
        },
    )
}
#[test]
fn root_mismatch_is_refused_and_unsent_preserves_original_scope_and_ticket() {
    let scope = scope();
    let other = WorkspaceRoot::Project {
        project: ProjectId::random(),
    };
    assert!(
        Request::file(
            scope.clone(),
            FileAsk::List {
                ask: ListDir {
                    root: other,
                    dir: RelPath::root(),
                    ignored: true,
                },
                ticket: FileTickets::default().issue().unwrap()
            }
        )
        .is_err()
    );
    let ticket = FileTickets::default().issue().unwrap();
    let request = Request::root(scope.connection().clone(), SessionId::random(), ticket);
    let (context, outcome) = request.refused().into_parts();
    assert_eq!(&context, scope.connection());
    assert!(
        matches!(outcome, Outcome::Root { ticket: original, result: Err(Problem::Unsent), .. } if original == ticket)
    );
}
#[tokio::test]
async fn retired_request_is_settled_without_sending_a_frame_to_replacement() {
    within(async {
        let home = Home::new();
        let (mut state, mut events) = state(&home);
        let mut original = peer::connect("fixture").await;
        let client = original.client.take().unwrap();
        let context = state.admissions.admit(MachineKey::from_book("fixture"), client.node().id).unwrap();
        let entry = vessel_client::book::BookEntry { address: "ws://127.0.0.1:0/rpc".parse().unwrap(), token: None, credential: None, id: None };
        let _old = state.clients.insert("fixture".into(), Connected::new(client, Vec::new(), entry.clone(), context.clone()));
        let ticket = FileTickets::default().issue().unwrap();
        let request = Request::root(context.clone(), SessionId::random(), ticket);
        state.clients["fixture"].retire();
        let mut replacement = peer::connect("fixture").await;
        let client = replacement.client.take().unwrap();
        let next = state.admissions.admit(MachineKey::from_book("fixture"), client.node().id).unwrap();
        let _retired = state.clients.insert("fixture".into(), Connected::new(client, Vec::new(), entry, next));
        state.scoped_file(request);
        let event = phase("settle original file request", events.recv()).await.unwrap();
        let crate::Event::FileOperation(reply) = event else { panic!("wrong owner receipt") };
        let (original_context, outcome) = reply.into_parts();
        assert_eq!(original_context, context);
        assert!(matches!(outcome, Outcome::Root { ticket: original, result: Err(Problem::Gone(_)), .. } if original == ticket));
        assert!(original.calls.lock().unwrap().is_empty());
        assert!(replacement.calls.lock().unwrap().is_empty());
    }).await;
}

#[tokio::test]
async fn retirement_after_write_dispatch_returns_original_uncertainty_without_replay() {
    use vessel_app_view::workspace::save::Saved;
    use vessel_proto::{Outcome as WireOutcome, Reply as WireReply, workspace::FileVersion};
    within(async {
        let home = Home::new();
        let (mut state, mut events) = state(&home);
        let mut original = peer::controlled("fixture").await;
        let client = original.client.take().unwrap();
        let context = state.admissions.admit(MachineKey::from_book("fixture"), client.node().id).unwrap();
        let entry = vessel_client::book::BookEntry { address: "ws://127.0.0.1:0/rpc".parse().unwrap(), token: None, credential: None, id: None };
        let _old = state.clients.insert("fixture".into(), Connected::new(client, Vec::new(), entry, context.clone()));
        let scope = FileScope::new(context.clone(), WorkspaceRoot::Project { project: ProjectId::random() });
        let ticket = FileTickets::default().issue().unwrap();
        // A normal guarded save isolates retirement after write dispatch;
        // explicit overwrite now has a separate existing-file pre-read.
        let guard = "a".repeat(64).parse::<vessel_proto::workspace::ContentHash>().unwrap();
        let ask = FileAsk::Save(super::Save { ticket, root: scope.root(), path: "a.txt".parse().unwrap(), text: "submitted".into(),
            base: Some(FileVersion { size: 8, modified: 0, hash: Some(guard.clone()) }) });
        state.scoped_file(Request::file(scope.clone(), ask).unwrap());
        let dispatched = phase("observe original write frame", original.requests.as_mut().unwrap().recv()).await.unwrap();
        assert!(matches!(&dispatched.call, vessel_proto::Call::WriteFile(write)
            if write.expect == vessel_proto::workspace::WriteExpect::Content { hash: guard }));
        state.clients["fixture"].retire();
        let _late = dispatched.answer.send(WireOutcome::Ok(WireReply::WriteFile(FileVersion { size: 9, modified: 0, hash: None })));
        let event = phase("settle retired original write", events.recv()).await.unwrap();
        let crate::Event::FileOperation(reply) = event else { panic!("file owner receipt") };
        let (original_context, outcome) = reply.into_parts();
        assert_eq!(original_context, context);
        let Outcome::File(reply) = outcome else { panic!("file outcome") };
        let (original_scope, answer, may_have_run) = (*reply).into_parts();
        assert_eq!(original_scope, scope);
        assert!(may_have_run);
        assert!(matches!(answer, super::FileAnswer::Saved { ticket: original, saved: Saved::Uncertain(_), .. } if original == ticket));
        assert_eq!(original.calls.lock().unwrap().len(), 1, "never replay a write");
    }).await;
}
