//! Typed in-memory peer asserts that retry and identity mismatch never create.

use super::{Scenario, TestResult};
use chrono::DateTime;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _, DuplexStream};
use vessel_proto::{
    Call, ClientFrame, Delivered, HarnessKind, NodeId, NodeInfo, NodeState, Outcome, ProtocolRange,
    Reply, RequestId, ServerFrame, SessionId, SessionInfo, SessionMode, SessionSpec, Welcome,
    WireError,
};

async fn read(stream: &mut DuplexStream) -> TestResult<Option<ClientFrame>> {
    let length = match stream.read_u32().await {
        Ok(length) => length,
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut bytes = vec![0; usize::try_from(length)?];
    let _read = stream.read_exact(&mut bytes).await?;
    Ok(Some(serde_json::from_slice(&bytes)?))
}
async fn write(stream: &mut DuplexStream, frame: ServerFrame) -> TestResult {
    let bytes = serde_json::to_vec(&frame)?;
    stream.write_u32(u32::try_from(bytes.len())?).await?;
    stream.write_all(&bytes).await?;
    Ok(())
}
async fn welcome(stream: &mut DuplexStream) -> TestResult {
    if !matches!(read(stream).await?, Some(ClientFrame::Hello(_))) {
        return Err(std::io::Error::other("hello required").into());
    }
    let node = NodeInfo {
        id: NodeId::random(),
        name: "fake".to_owned(),
        hostname: "fake".to_owned(),
        os: "test".to_owned(),
        arch: "test".to_owned(),
        version: "0".to_owned(),
        protocol: ProtocolRange::CURRENT,
        started_at: DateTime::UNIX_EPOCH,
        state: NodeState::Serving,
        public_key: None,
        timezone: None,
    };
    write(
        stream,
        ServerFrame::Welcome(Welcome {
            protocol: ProtocolRange::CURRENT.max,
            node,
        }),
    )
    .await
}
fn fixture(session: SessionId) -> SessionInfo {
    SessionInfo::new(
        session,
        SessionSpec::new(
            HarnessKind::Codex,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        ),
        DateTime::UNIX_EPOCH,
    )
}
async fn register(stream: &mut DuplexStream, scenario: Scenario) -> TestResult<Option<SessionId>> {
    let Some(ClientFrame::Request { id, call }) = read(stream).await? else {
        return Err(std::io::Error::other("registration request required").into());
    };
    let (info, reply) = match (call, scenario) {
        (Call::GetSession(request), Scenario::Retry(expected)) if request.session == expected => {
            let info = fixture(expected);
            (info.clone(), Reply::GetSession(info))
        }
        (Call::GetSession(request), Scenario::WrongRetryReply(expected))
            if request.session == expected =>
        {
            let info = fixture(SessionId::random());
            (info.clone(), Reply::GetSession(info))
        }
        (
            Call::CreateSession(request),
            Scenario::Accept
            | Scenario::RefusePrompt(_)
            | Scenario::DisconnectPrompt
            | Scenario::RefuseCreate(_),
        ) => {
            if let Scenario::RefuseCreate(code) = scenario {
                write(
                    stream,
                    ServerFrame::Response {
                        id,
                        outcome: Outcome::Err(WireError::new(code, "private diagnostics")),
                    },
                )
                .await?;
                return Ok(None);
            }
            let info = SessionInfo::new(SessionId::random(), request.spec, DateTime::UNIX_EPOCH);
            (info.clone(), Reply::CreateSession(info))
        }
        _ => return Err(std::io::Error::other("retry must read and never create").into()),
    };
    write(
        stream,
        ServerFrame::Response {
            id,
            outcome: Outcome::Ok(reply),
        },
    )
    .await?;
    Ok(Some(info.id))
}
async fn prompt(stream: &mut DuplexStream, id: RequestId, scenario: Scenario) -> TestResult {
    let outcome = match scenario {
        Scenario::Accept | Scenario::Retry(_) => {
            Outcome::Ok(Reply::PromptSession(Delivered::Started))
        }
        Scenario::RefusePrompt(code) => Outcome::Err(WireError::new(code, "private diagnostics")),
        Scenario::DisconnectPrompt => return Ok(()),
        Scenario::RefuseCreate(_) | Scenario::WrongNode | Scenario::WrongRetryReply(_) => {
            return Err(std::io::Error::other("no prompt allowed").into());
        }
    };
    write(stream, ServerFrame::Response { id, outcome }).await
}
async fn await_close(stream: &mut DuplexStream) -> TestResult {
    // A success/refusal fixture models a still-admitted connection. Dropping
    // immediately after its reply instead races the post-await liveness guard
    // and correctly makes even that reply uncertain. Keep transport loss an
    // explicit scenario, and still reject any extra create/prompt dispatch.
    if read(stream).await?.is_some() {
        return Err(std::io::Error::other("no request after the terminal reply").into());
    }
    Ok(())
}
pub(super) async fn serve(mut stream: DuplexStream, scenario: Scenario) -> TestResult {
    welcome(&mut stream).await?;
    if matches!(scenario, Scenario::WrongNode) {
        if read(&mut stream).await?.is_some() {
            return Err(std::io::Error::other("identity mismatch sent a request").into());
        }
        return Ok(());
    }
    let Some(session) = register(&mut stream, scenario).await? else {
        return await_close(&mut stream).await;
    };
    if let Some(frame) = read(&mut stream).await? {
        let ClientFrame::Request {
            id,
            call: Call::PromptSession(request),
        } = frame
        else {
            return Err(std::io::Error::other("registration must not repeat").into());
        };
        if request.session != session {
            return Err(std::io::Error::other("wrong retry destination").into());
        }
        prompt(&mut stream, id, scenario).await?;
        if !matches!(scenario, Scenario::DisconnectPrompt) {
            await_close(&mut stream).await?;
        }
    }
    Ok(())
}
