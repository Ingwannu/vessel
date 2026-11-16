//! Bounded serialization at the existing fixture framing edge, never a second codec.
use crate::testing::FixtureError;
use std::io::{self, Write};
use vessel_proto::ServerFrame;
struct Buffer {
    bytes: Vec<u8>,
}
impl Write for Buffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let length = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::other("fixture frame size overflow"))?;
        if length > super::FRAME_BYTES {
            return Err(io::Error::other("fixture frame exceeds bound"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
pub(super) fn frame(frame: &ServerFrame) -> Result<Vec<u8>, FixtureError> {
    let mut buffer = Buffer {
        bytes: Vec::with_capacity(super::FRAME_BYTES),
    };
    // Raw serialize/decode diagnostics could echo a credential. Retain only
    // the boundary's closed failure, matching the production answer projector.
    serde_json::to_writer(&mut buffer, frame).map_err(|_raw_encode| FixtureError::Protocol)?;
    Ok(buffer.bytes)
}
