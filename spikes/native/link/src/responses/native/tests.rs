//! The real framed client talks only to an in-memory peer, never a daemon.
use super::*;
use std::num::NonZeroUsize;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _, DuplexStream};
use vessel_app_view::{
    devices::MachineKey,
    responses::{Registry, Target},
};
use vessel_client::{ClientConfig, NodeClient};
use vessel_proto::{
    Ack, Call, ClientFrame, ClientInfo, ErrorCode, NodeId, NodeInfo, NodeState, Outcome,
    PermissionId, PermissionOptionId, ProtocolRange, Reply, ServerFrame, SessionId, Welcome,
    WireError,
};

const FRAME_CAP: usize = 65_536;
async fn read(stream: &mut DuplexStream) -> ClientFrame {
    let length = usize::try_from(stream.read_u32().await.unwrap()).unwrap();
    assert!(length <= FRAME_CAP);
    let mut data = vec![0; length];
    stream.read_exact(&mut data).await.unwrap();
    serde_json::from_slice(&data).unwrap()
}
async fn write(stream: &mut DuplexStream, frame: ServerFrame) {
    let data = serde_json::to_vec(&frame).unwrap();
    stream
        .write_u32(u32::try_from(data.len()).unwrap())
        .await
        .unwrap();
    stream.write_all(&data).await.unwrap();
}
async fn peer() -> (NodeClient, DuplexStream) {
    let (local, mut remote) = tokio::io::duplex(FRAME_CAP);
    let connecting = tokio::spawn(async move {
        let (reader, writer) = tokio::io::split(local);
        NodeClient::over(
            vessel_rpc::Framed::new(reader, writer, NonZeroUsize::new(FRAME_CAP).unwrap()),
            ClientInfo {
                name: "response-test".into(),
                version: "0".into(),
            },
            &ClientConfig::default(),
        )
        .await
        .unwrap()
    });
    assert!(matches!(read(&mut remote).await, ClientFrame::Hello(_)));
    write(
        &mut remote,
        ServerFrame::Welcome(Welcome {
            protocol: vessel_proto::PROTOCOL_VERSION,
            node: NodeInfo {
                id: NodeId::random(),
                name: "fake".into(),
                hostname: "fake".into(),
                os: "linux".into(),
                arch: "x86_64".into(),
                version: "0".into(),
                protocol: ProtocolRange::CURRENT,
                started_at: chrono::DateTime::UNIX_EPOCH,
                state: NodeState::Serving,
                public_key: None,
                timezone: None,
            },
        }),
    )
    .await;
    (connecting.await.unwrap(), remote)
}
fn ask() -> Ask {
    let response = Response::Permission {
        id: PermissionId::random(),
        option: PermissionOptionId::new("implement_resume"),
    };
    let pending = Registry::default()
        .begin(
            Target {
                machine: MachineKey::from_book("scratch"),
                session: SessionId::random(),
            },
            response.request(),
        )
        .unwrap();
    Ask::new(pending, response).unwrap()
}

#[tokio::test]
async fn only_an_acknowledged_exact_option_succeeds_and_diagnostics_stay_private() {
    for success in [true, false] {
        let (client, mut remote) = peer().await;
        let ask = ask();
        let pending = ask.pending.clone();
        let task = tokio::spawn(async move { run(&client, ask).await });
        let ClientFrame::Request {
            id,
            call: Call::SendInput(input),
        } = read(&mut remote).await
        else {
            panic!("one typed input")
        };
        assert_eq!(input.session, pending.target.session);
        assert!(
            matches!(input.input, SessionInput::AnswerPermission { id, outcome: PermissionOutcome::Selected { option } } if pending.request == vessel_app_view::responses::Request::Permission(id) && option.as_str() == "implement_resume")
        );
        assert!(!task.is_finished());
        let outcome = if success {
            Outcome::Ok(Reply::SendInput(Ack {}))
        } else {
            Outcome::Err(WireError::new(
                ErrorCode::Conflict,
                "synthetic-private-diagnostic",
            ))
        };
        write(&mut remote, ServerFrame::Response { id, outcome }).await;
        let answer = task.await.unwrap();
        assert_eq!(answer.pending, pending);
        assert_eq!(
            answer.result,
            if success {
                Ok(())
            } else {
                Err(Problem::Refused(ErrorCode::Conflict))
            }
        );
        assert!(!format!("{answer:?}").contains("synthetic-private-diagnostic"));
    }
}

#[tokio::test(start_paused = true)]
async fn deadline_is_uncertain_and_never_sends_a_second_input_or_denial() {
    let (client, mut remote) = peer().await;
    let ask = ask();
    let pending = ask.pending.clone();
    let task = tokio::spawn(async move { run(&client, ask).await });
    assert!(matches!(
        read(&mut remote).await,
        ClientFrame::Request {
            call: Call::SendInput(_),
            ..
        }
    ));
    tokio::time::advance(RESPONSE_DEADLINE).await;
    let answer = task.await.unwrap();
    assert_eq!(answer.pending, pending);
    assert_eq!(answer.result, Err(Problem::Unconfirmed));
    // The only client handle was dropped; any extra frame before EOF would
    // prove a retry/denial was sent after the unconfirmed result.
    assert!(
        matches!(remote.read_u32().await, Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof)
    );
}

#[tokio::test]
async fn missing_connection_and_capacity_refuse_the_same_ticket() {
    let hub = Hub::default();
    let (events, mut answers) = mpsc::channel(1);
    let ask = ask();
    let pending = ask.pending.clone();
    hub.ask(None, ask, events.clone()).await;
    let crate::Event::Responded(answer) = answers.recv().await.unwrap() else {
        panic!("response")
    };
    assert_eq!(answer.pending, pending);
    assert_eq!(answer.result, Err(Problem::Offline));
    let permit = Arc::clone(&hub.0)
        .acquire_many_owned(u32::try_from(MAX_PENDING).unwrap())
        .await
        .unwrap();
    let ask = self::ask();
    let pending = ask.pending.clone();
    hub.ask(None, ask, events).await;
    let crate::Event::Responded(answer) = answers.recv().await.unwrap() else {
        panic!("response")
    };
    assert_eq!(answer.pending, pending);
    assert_eq!(answer.result, Err(Problem::Busy));
    drop(permit);
}
