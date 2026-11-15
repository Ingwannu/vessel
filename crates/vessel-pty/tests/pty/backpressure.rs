//! A real nonreading child holds the OS writer and existing bounded input queue.
//! This establishes enqueue cancellation, not a node responder/OS delivery contract.
use super::{SIZE, fake_tui, within};
use bytes::Bytes;
use std::io::{Read as _, Write as _};
use std::net::{Ipv4Addr, SocketAddrV4, TcpStream};
use std::time::Duration;
use vessel_pty::{PtyOutput, PtyProcess};

// This cancellation fixture window is not the future node policy constant.
const CANCEL_WINDOW: Duration = Duration::from_millis(250);
const BLOCK_BYTES: usize = 64 * 1024;
const MAX_BLOCKS: usize = 128;

#[tokio::test]
async fn full_real_queue_cancels_expired_admission_without_late_bytes_and_output_stays_live() {
    within(async {
        let (mut process, output) =
            PtyProcess::spawn(&fake_tui().arg("--held-input"), SIZE).unwrap();
        let mut seen = Output::new(output);
        let ready = seen.until("\n").await;
        let port = ready
            .strip_prefix("control ")
            .unwrap()
            .trim()
            .parse::<u16>()
            .unwrap();
        let input = process.input();
        let admitted = fill(&input).await;
        assert!(admitted > 0);
        let expired = input.write(Bytes::from_static(b"NODE-REPLY-EXPIRED-CANARY"));
        let observe = async {
            tick(port).await;
            seen.until("held tick 1").await
        };
        let (result, tick_output) =
            tokio::join!(tokio::time::timeout(CANCEL_WINDOW, expired), observe);
        assert!(
            result.is_err(),
            "a full queue unexpectedly admitted expired reply bytes"
        );
        assert!(tick_output.contains("held tick 1"));
        tick(port).await;
        assert!(seen.until("held tick 2").await.contains("held tick 2"));
        assert!(
            tokio::time::timeout(Duration::from_millis(1), process.wait())
                .await
                .is_err()
        );
        release(port).await;
        input.write(Bytes::from_static(b"done\n")).await.unwrap();
        let expected = admitted
            .checked_mul(BLOCK_BYTES)
            .unwrap()
            .checked_add(5)
            .unwrap();
        let drained = seen.until("expired false").await;
        assert!(
            drained.contains(&format!("drained {expected} expired false")),
            "{drained}"
        );
        assert_eq!(process.wait().await, Some(0));
        while seen.output.next().await.is_some() {}
    })
    .await;
}

async fn fill(input: &vessel_pty::PtyInput) -> usize {
    let block = Bytes::from(vec![b'a'; BLOCK_BYTES]);
    for count in 0..MAX_BLOCKS {
        match tokio::time::timeout(CANCEL_WINDOW, input.write(block.clone())).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => panic!("fixture closed instead of applying pressure: {error}"),
            Err(_elapsed) => return count,
        }
    }
    panic!("the actual writer/control queue never filled within its fixture budget")
}

async fn tick(port: u16) {
    tokio::task::spawn_blocking(move || {
        let mut control = control(port);
        control.write_all(b"t").unwrap();
        let mut ack = [0_u8; 2];
        control.read_exact(&mut ack).unwrap();
        assert_eq!(&ack, b"ok");
    })
    .await
    .unwrap();
}
async fn release(port: u16) {
    tokio::task::spawn_blocking(move || control(port).write_all(b"r").unwrap())
        .await
        .unwrap();
}
fn control(port: u16) -> TcpStream {
    let stream = TcpStream::connect_timeout(
        &SocketAddrV4::new(Ipv4Addr::LOCALHOST, port).into(),
        Duration::from_secs(5),
    )
    .unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    stream
}
struct Output {
    output: PtyOutput,
    text: String,
}
impl Output {
    const fn new(output: PtyOutput) -> Self {
        Self {
            output,
            text: String::new(),
        }
    }
    async fn until(&mut self, needle: &str) -> String {
        while !self.text.contains(needle) {
            let chunk = self
                .output
                .next()
                .await
                .expect("fixture output ended too soon");
            self.text.push_str(&String::from_utf8_lossy(&chunk));
            assert!(
                self.text.len() < 8192,
                "fixture output exceeded its observation budget"
            );
        }
        self.text.clone()
    }
}
