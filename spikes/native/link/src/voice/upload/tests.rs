use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tokio::sync::oneshot;

#[derive(Debug, Default)]
struct Probe {
    sent: AtomicUsize,
    released: AtomicBool,
}

struct Node {
    probe: Arc<Probe>,
    entered_upload: Option<oneshot::Sender<()>>,
    entered_stop: Option<oneshot::Sender<()>>,
    stop_reply: oneshot::Receiver<Result<(), Failure>>,
}

impl Adapter for Node {
    async fn upload(&mut self, frame: Frame) -> Result<(), Failure> {
        assert!(!frame.pcm().is_empty());
        let _sent = self.probe.sent.fetch_add(1, Ordering::AcqRel);
        if let Some(tx) = self.entered_upload.take() {
            tx.send(()).unwrap();
        }
        std::future::pending().await
    }

    async fn stop(&mut self) -> Result<(), Failure> {
        // The controller must have disposed its local backend before starting this wait.
        assert!(self.probe.released.load(Ordering::Acquire));
        self.entered_stop.take().unwrap().send(()).unwrap();
        (&mut self.stop_reply)
            .await
            .unwrap_or(Err(Failure::Disconnected))
    }
}

struct Device {
    probe: Arc<Probe>,
    release: Result<(), Failure>,
}

impl Audio for Device {
    fn release(&mut self) -> impl Future<Output = Result<(), Failure>> + Send {
        std::future::ready(self.release)
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        self.probe.released.store(true, Ordering::Release);
    }
}

fn frame() -> Frame {
    Frame::new(Bytes::from_static(&[0, 0])).unwrap()
}

#[tokio::test]
async fn full_queues_and_rejected_remote_stop_cannot_keep_or_retarget_capture() {
    let probe = Arc::new(Probe::default());
    let (capture, handle, actor) = channel();
    for _ in 0..CAPTURE_QUEUE_FRAMES {
        capture.submit(frame()).unwrap();
    }
    let (upload_tx, upload_rx) = oneshot::channel();
    let (stop_tx, stop_rx) = oneshot::channel();
    let (reply_tx, reply_rx) = oneshot::channel();
    let node = Node {
        probe: Arc::clone(&probe),
        entered_upload: Some(upload_tx),
        entered_stop: Some(stop_tx),
        stop_reply: reply_rx,
    };
    let device = Device {
        probe: Arc::clone(&probe),
        release: Ok(()),
    };
    let task = tokio::spawn(actor.run(node, device));
    upload_rx.await.unwrap();
    // The in-flight send is held; refill the one slot it consumed.
    capture.submit(frame()).unwrap();
    handle.stop();
    assert_eq!(capture.submit(frame()), Err(CaptureError::Stopped));
    stop_rx.await.unwrap();
    let status = handle.status();
    assert_eq!(status.borrow().local, Local::Released);
    assert_eq!(status.borrow().remote, Remote::Stopping);
    assert_eq!(probe.sent.load(Ordering::Acquire), 1);
    // A late callback and the still-full queue cannot create a second send.
    assert_eq!(capture.clone().submit(frame()), Err(CaptureError::Stopped));
    reply_tx.send(Err(Failure::Rejected)).unwrap();
    task.await.unwrap();
    assert_eq!(status.borrow().remote, Remote::Uncertain(Failure::Rejected));
    assert_eq!(status.borrow().local, Local::Released);
    assert_eq!(probe.sent.load(Ordering::Acquire), 1);
}

#[tokio::test(start_paused = true)]
async fn lost_remote_reply_and_release_failure_remain_visible_after_local_disposal() {
    let probe = Arc::new(Probe::default());
    let (capture, handle, actor) = channel();
    let (stop_tx, stop_rx) = oneshot::channel();
    let (_reply_tx, reply_rx) = oneshot::channel();
    let node = Node {
        probe: Arc::clone(&probe),
        entered_upload: None,
        entered_stop: Some(stop_tx),
        stop_reply: reply_rx,
    };
    let device = Device {
        probe: Arc::clone(&probe),
        release: Err(Failure::Rejected),
    };
    handle.stop();
    let task = tokio::spawn(actor.run(node, device));
    stop_rx.await.unwrap();
    assert!(probe.released.load(Ordering::Acquire));
    assert_eq!(capture.submit(frame()), Err(CaptureError::Stopped));
    tokio::time::advance(STOP_DEADLINE).await;
    task.await.unwrap();
    let status = handle.status();
    assert_eq!(status.borrow().local, Local::Uncertain(Failure::Rejected));
    assert_eq!(status.borrow().remote, Remote::Uncertain(Failure::TimedOut));
    assert_eq!(probe.sent.load(Ordering::Acquire), 0);
}

#[tokio::test]
async fn queue_overrun_is_explicit_and_owner_drop_stops_without_a_data_command() {
    assert!(Frame::new(Bytes::new()).is_err());
    assert!(Frame::new(Bytes::from_static(&[1])).is_err());
    assert!(!format!("{:?}", frame()).contains("[0, 0]"));
    let (capture, handle, _actor) = channel();
    for _ in 0..CAPTURE_QUEUE_FRAMES {
        capture.submit(frame()).unwrap();
    }
    assert_eq!(capture.submit(frame()), Err(CaptureError::Overrun));
    assert_eq!(capture.submit(frame()), Err(CaptureError::Stopped));
    drop(handle);
    let (capture, handle, _actor) = channel();
    drop(handle);
    assert_eq!(capture.submit(frame()), Err(CaptureError::Stopped));
}
