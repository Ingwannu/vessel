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

#[tokio::test]
async fn supplied_device_fence_wakes_control_even_with_full_data_queues() {
    let device_gate = Arc::new(Fence::default());
    let (capture, handle, _actor) = with_fence(Arc::clone(&device_gate));
    for _ in 0..CAPTURE_QUEUE_FRAMES {
        capture.submit(frame()).unwrap();
    }
    handle.stop();
    assert!(device_gate.is_closed());
    device_gate.closed().await;
    assert_eq!(capture.submit(frame()), Err(CaptureError::Stopped));
    // Device-side failure closes this exact gate too, waking an uploader
    // blocked on a remote upload without requiring a data-queue command.
    let gate = Arc::new(Fence::default());
    let (capture, handle, _actor) = with_fence(Arc::clone(&gate));
    gate.close();
    assert_eq!(capture.submit(frame()), Err(CaptureError::Stopped));
    assert!(handle.fence.is_closed());
}

#[test]
fn maximum_size_chunks_cannot_turn_the_two_second_queue_into_ten_seconds() {
    let (capture, handle, _actor) = channel();
    let pcm = Bytes::from(vec![0; MAX_FRAME_BYTES]);
    for _ in 0..20 {
        capture.submit(Frame::new(pcm.clone()).unwrap()).unwrap();
    }
    assert_eq!(capture.budget.available_permits(), 0);
    assert_eq!(capture.submit(frame()), Err(CaptureError::Overrun));
    assert_eq!(capture.submit(frame()), Err(CaptureError::Stopped));
    handle.stop();
}

#[tokio::test]
async fn an_in_flight_upload_keeps_its_byte_permit_until_cancelled() {
    let probe = Arc::new(Probe::default());
    let (capture, handle, actor) = channel();
    let pcm = Bytes::from(vec![0; MAX_FRAME_BYTES]);
    for _ in 0..20 {
        capture.submit(Frame::new(pcm.clone()).unwrap()).unwrap();
    }
    let (upload_tx, upload_rx) = oneshot::channel();
    let (stop_tx, stop_rx) = oneshot::channel();
    let (reply_tx, reply_rx) = oneshot::channel();
    let task = tokio::spawn(actor.run(
        Node {
            probe: Arc::clone(&probe),
            entered_upload: Some(upload_tx),
            entered_stop: Some(stop_tx),
            stop_reply: reply_rx,
        },
        Device {
            probe: Arc::clone(&probe),
            release: Ok(()),
        },
    ));
    upload_rx.await.unwrap();
    // The channel has space, but its held request still owns its audio budget.
    assert_eq!(capture.budget.available_permits(), 0);
    assert_eq!(capture.submit(frame()), Err(CaptureError::Overrun));
    stop_rx.await.unwrap();
    assert_eq!(capture.budget.available_permits(), CAPTURE_QUEUE_BYTES);
    assert_eq!(probe.sent.load(Ordering::Acquire), 1);
    assert_eq!(capture.submit(frame()), Err(CaptureError::Stopped));
    reply_tx.send(Ok(())).unwrap();
    task.await.unwrap();
    assert_eq!(handle.status().borrow().remote, Remote::Stopped);
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
#[tokio::test]
async fn the_same_capture_budget_survives_device_normalization_queue_and_upload() {
    use crate::audio::testing::CaptureFixture;
    use vessel_app_view::audio::CapturePurpose;
    let mut device = CaptureFixture::new(CapturePurpose::Conversation);
    let lease = device.lease();
    let (capture, handle, actor) = with_fence(device.fence());
    for _ in 0..CAPTURE_QUEUE_FRAMES {
        capture
            .submit(Frame::from_capture(lease, device.frame()).unwrap())
            .unwrap();
    }
    let probe = Arc::new(Probe::default());
    let (upload_tx, upload_rx) = oneshot::channel();
    let (stop_tx, stop_rx) = oneshot::channel();
    let (reply_tx, reply_rx) = oneshot::channel();
    let task = tokio::spawn(actor.run(
        Node {
            probe: Arc::clone(&probe),
            entered_upload: Some(upload_tx),
            entered_stop: Some(stop_tx),
            stop_reply: reply_rx,
        },
        Device {
            probe,
            release: Ok(()),
        },
    ));
    upload_rx.await.unwrap();
    // One in flight plus99queued; moving out of either queue retained all permits.
    assert_eq!(device.available(), 32_000);
    capture
        .submit(Frame::from_capture(lease, device.frame()).unwrap())
        .unwrap();
    assert_eq!(device.available(), 31_680);
    handle.stop();
    stop_rx.await.unwrap();
    assert_eq!(device.available(), CAPTURE_QUEUE_BYTES);
    reply_tx.send(Ok(())).unwrap();
    task.await.unwrap();
}
