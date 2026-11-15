//! Legacy unit-only assertion entry points over the same fallible framed core.
use super::*;
pub(crate) async fn connect(name: &str) -> Peer {
    start(name, NodeId::random(), None)
        .await
        .expect("framed unit peer")
}
pub(crate) async fn controlled(name: &str) -> Peer {
    try_controlled(name, NodeId::random())
        .await
        .expect("controlled framed unit peer")
}
impl Peer {
    pub(crate) async fn push(&self, frame: ServerFrame) {
        assert!(
            crate::testing::phase("injected frame write receipt", send(&self.frames, frame))
                .await
                .expect("framed write receipt")
        );
    }
}
