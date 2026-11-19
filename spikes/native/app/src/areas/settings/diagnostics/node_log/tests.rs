//! When the Log file row asks its machine again (the app reviewer's R2 on
//! row 337): after an answer that may change, or on a new connection;
//! never while asked, nor after a path or a lasting refusal.

use std::path::PathBuf;

use vessel_app_view::connection::{ConnectionEpoch, ConnectionEpochs, MachineKey};
use vessel_app_view::devices::Tickets;
use vessel_app_view::diagnostics::Problem;
use vessel_proto::ErrorCode;
use vessel_proto::diagnostics::NodeLog;

use super::{Read, asks_again};

fn epoch(epochs: &mut ConnectionEpochs) -> ConnectionEpoch {
    epochs
        .admit(
            MachineKey::from_book("fixture"),
            vessel_proto::NodeId::random(),
        )
        .unwrap()
        .epoch()
}

fn read(epoch: ConnectionEpoch, answer: Option<Result<NodeLog, Problem>>) -> Read {
    Read {
        machine: "fixture".to_owned(),
        epoch: Some(epoch),
        asking: None,
        answer,
    }
}

#[test]
fn a_transient_failure_is_asked_again_and_a_lasting_one_is_not() {
    let mut epochs = ConnectionEpochs::default();
    let now = epoch(&mut epochs);
    let at = Some(now);
    assert!(asks_again(None, "fixture", at), "nothing read yet");
    for problem in [
        Problem::TimedOut,
        Problem::Offline,
        Problem::Refused(ErrorCode::Unavailable),
    ] {
        assert!(
            asks_again(Some(&read(now, Some(Err(problem)))), "fixture", at),
            "{problem:?}"
        );
    }
    for problem in [
        Problem::Refused(ErrorCode::Forbidden),
        Problem::Refused(ErrorCode::Unsupported),
    ] {
        assert!(
            !asks_again(Some(&read(now, Some(Err(problem)))), "fixture", at),
            "{problem:?}"
        );
    }
    let path = NodeLog::File {
        path: PathBuf::from("~/.vessel/logs/vesseld.log"),
        lines: vec!["one".to_owned()],
        cut: false,
    };
    assert!(
        !asks_again(Some(&read(now, Some(Ok(path)))), "fixture", at),
        "a path is kept"
    );
    let mut asked = read(now, None);
    asked.asking = Some(Tickets::default().next_ticket().unwrap());
    assert!(
        !asks_again(Some(&asked), "fixture", at),
        "an ask in flight is waited for"
    );
    assert!(
        asks_again(Some(&read(now, None)), "another", at),
        "another machine"
    );
}

#[test]
fn a_new_connection_reads_again_even_after_a_path() {
    let mut epochs = ConnectionEpochs::default();
    let before = epoch(&mut epochs);
    let after = epoch(&mut epochs);
    let kept = read(
        before,
        Some(Ok(NodeLog::File {
            path: PathBuf::from("~/.vessel/logs/vesseld.log"),
            lines: Vec::new(),
            cut: false,
        })),
    );
    assert!(!asks_again(Some(&kept), "fixture", Some(before)));
    assert!(
        asks_again(Some(&kept), "fixture", Some(after)),
        "offline, then online again"
    );
}
