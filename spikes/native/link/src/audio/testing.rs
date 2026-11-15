//! Test-only frame producer: no device, second product engine or public constructor.
use super::capture::Held;
use super::dsp::Processor;
use super::format::Format;
use super::owner::{Engine, Handle, Owner};
use super::{CaptureState, Frame};
use crate::cancel::Fence;
use std::sync::Arc;
use vessel_app_view::audio::{CaptureLease, CapturePurpose};

#[derive(Debug)]
pub(crate) struct CaptureFixture {
    owner: Owner,
    _control: Handle,
    processor: Processor,
}
impl CaptureFixture {
    pub(crate) fn new(purpose: CapturePurpose) -> Self {
        let (owner, control) = Engine::default().reserve(purpose).unwrap();
        owner.started().unwrap();
        assert_eq!(*control.status().borrow(), CaptureState::Running);
        Self {
            owner,
            _control: control,
            processor: Processor::capture_only(Format::capture()).unwrap(),
        }
    }
    pub(crate) fn lease(&self) -> CaptureLease {
        self.owner.lease()
    }
    pub(crate) fn fence(&self) -> Arc<Fence> {
        self.owner.fence()
    }
    pub(crate) fn available(&self) -> usize {
        self.owner.budget().available()
    }
    pub(crate) async fn wait_empty(&self) {
        self.owner.budget().wait_empty().await;
    }
    pub(crate) fn mute(&self, mute: vessel_app_view::voice::Mute) {
        self.owner.upload_gate().set_mute(mute).unwrap();
    }
    pub(crate) fn frame(&mut self) -> Frame {
        let permit = self.owner.budget().reserve().unwrap();
        let audio = self.processor.capture(&[0.0; 160]).unwrap();
        Frame::new(
            self.owner.lease(),
            Held::new(audio, permit, self.owner.upload_gate().snapshot()),
        )
    }
}
