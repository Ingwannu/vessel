//! Validate the declared WAV container before any remote stream is opened.
//!
//! Existing attachment readers neither decode WAV nor validate its full extent.
//! Seeking over audio keeps preflight bounded; later reads use only that extent,
//! so growth is ignored and truncation remains a failure rather than a short tail.

use std::{io::SeekFrom, path::Path};
use tokio::{
    fs::File,
    io::{AsyncReadExt as _, AsyncSeekExt as _},
};
use vessel_proto::dictation::{DictationPcm, MAX_SEQ, Pcm16Mono, SampleRate};

const MAX_METADATA_BYTES: u64 = 64 * 1024;
const CONTAINER_HEADER_BYTES: usize = 12;
const CHUNK_HEADER_BYTES: usize = 8;
const FORMAT_BYTES: usize = 16;
const RIFF_PREFIX_BYTES: u64 = 8;
const SAMPLE_BYTES: usize = size_of::<i16>();
const PCM_ENCODING: u16 = 1;
const MONO: u16 = 1;
const RIFF_ALIGNMENT: u32 = 2;

#[derive(Debug, thiserror::Error)]
pub(super) enum FileError {
    #[error("dictation input must be a regular file")]
    NotRegular,
    #[error("dictation input could not be read")]
    Io(#[from] std::io::Error),
    #[error("dictation input must be a complete little-endian RIFF/WAVE container")]
    Container,
    #[error("dictation input contains a duplicate format or audio chunk")]
    Duplicate,
    #[error("dictation input must contain mono signed PCM16 with consistent format fields")]
    Format,
    #[error("dictation input has an unsupported sample rate")]
    Rate(#[source] vessel_proto::dictation::DictationDecodeError),
    #[error("dictation input metadata exceeds {MAX_METADATA_BYTES} bytes")]
    Metadata,
    #[error(
        "dictation input must contain complete nonempty samples within the stream duration bound"
    )]
    Duration,
    #[error("dictation input count exceeds this platform's range")]
    Count(#[source] std::num::TryFromIntError),
    #[error("dictation input has an incomplete format field")]
    Width(#[source] std::array::TryFromSliceError),
    #[error("dictation input chunk failed PCM validation")]
    Pcm(#[source] vessel_proto::dictation::DictationDecodeError),
}

#[derive(Clone, Copy)]
struct Extent {
    start: u64,
    bytes: u32,
}
impl Extent {
    fn end(self) -> Result<u64, FileError> {
        self.start
            .checked_add(u64::from(self.bytes))
            .ok_or(FileError::Container)
    }
    fn padded_end(self) -> Result<u64, FileError> {
        self.end()?
            .checked_add(u64::from(self.bytes % RIFF_ALIGNMENT))
            .ok_or(FileError::Container)
    }
}
struct Layout {
    format: Pcm16Mono,
    audio: Extent,
}

/// Own one file and one bounded read at a time; no recording is copied up front.
pub(super) struct FileInput {
    file: File,
    format: Pcm16Mono,
    remaining: u32,
}
impl FileInput {
    /// Both checks reduce accidental pipe/device reads, without claiming a snapshot.
    pub(super) async fn open(path: &Path) -> Result<Self, FileError> {
        if !tokio::fs::metadata(path).await?.is_file() {
            return Err(FileError::NotRegular);
        }
        let mut file = File::open(path).await?;
        let metadata = file.metadata().await?;
        if !metadata.is_file() {
            return Err(FileError::NotRegular);
        }
        let layout = inspect(&mut file, metadata.len()).await?;
        let _position = file.seek(SeekFrom::Start(layout.audio.start)).await?;
        Ok(Self {
            file,
            format: layout.format,
            remaining: layout.audio.bytes,
        })
    }
    pub(super) const fn format(&self) -> Pcm16Mono {
        self.format
    }
    /// A successful read is a full second or the final short complete-sample tail.
    pub(super) async fn next(&mut self) -> Result<Option<DictationPcm>, FileError> {
        if self.remaining == 0 {
            return Ok(None);
        }
        let width = u32::try_from(SAMPLE_BYTES).map_err(FileError::Count)?;
        // A validated rate and PCM16 width fit in u32; saturation cannot engage.
        let second = self.format.rate().get().saturating_mul(width);
        let take = self.remaining.min(second);
        let mut bytes = vec![0; usize::try_from(take).map_err(FileError::Count)?];
        let _filled = self.file.read_exact(&mut bytes).await?;
        let pcm = DictationPcm::new(bytes).map_err(FileError::Pcm)?;
        self.remaining = self.remaining.saturating_sub(take);
        Ok(Some(pcm))
    }
}

async fn inspect(file: &mut File, physical: u64) -> Result<Layout, FileError> {
    let mut header = [0; CONTAINER_HEADER_BYTES];
    let _filled = file.read_exact(&mut header).await?;
    if header.get(..4) != Some(b"RIFF".as_slice()) || header.get(8..) != Some(b"WAVE".as_slice()) {
        return Err(FileError::Container);
    }
    let end = u64::from(word(&header, 4)?)
        .checked_add(RIFF_PREFIX_BYTES)
        .ok_or(FileError::Container)?;
    let mut position = u64::try_from(CONTAINER_HEADER_BYTES).map_err(FileError::Count)?;
    if end < position || end > physical {
        return Err(FileError::Container);
    }
    let mut metadata = position;
    let mut format = None;
    let mut audio = None;
    while position < end {
        let chunk = read_chunk(file, position, end).await?;
        position = chunk.next;
        if chunk.kind == Kind::Audio && audio.replace(chunk.extent).is_some() {
            return Err(FileError::Duplicate);
        }
        let padding = u64::from(chunk.extent.bytes % RIFF_ALIGNMENT);
        let extra = match chunk.kind {
            Kind::Audio => padding,
            Kind::Format | Kind::Other => u64::from(chunk.extent.bytes).saturating_add(padding),
        };
        // Every extent fits the declared container; saturation keeps a future
        // wider file count above the metadata cap rather than wrapping below it.
        metadata = metadata
            .saturating_add(u64::try_from(CHUNK_HEADER_BYTES).map_err(FileError::Count)?)
            .saturating_add(extra);
        if metadata > MAX_METADATA_BYTES {
            return Err(FileError::Metadata);
        }
        if chunk.kind == Kind::Format {
            if format.is_some() {
                return Err(FileError::Duplicate);
            }
            format = Some(read_format(file, chunk.extent.bytes).await?);
        }
        let _position = file.seek(SeekFrom::Start(position)).await?;
    }
    let format = format.ok_or(FileError::Format)?;
    let audio = audio.ok_or(FileError::Duration)?;
    check_duration(format.rate(), audio.bytes)?;
    Ok(Layout { format, audio })
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Format,
    Audio,
    Other,
}
struct Chunk {
    kind: Kind,
    extent: Extent,
    next: u64,
}
async fn read_chunk(file: &mut File, position: u64, end: u64) -> Result<Chunk, FileError> {
    let start = position
        .checked_add(u64::try_from(CHUNK_HEADER_BYTES).map_err(FileError::Count)?)
        .ok_or(FileError::Container)?;
    if start > end {
        return Err(FileError::Container);
    }
    let mut header = [0; CHUNK_HEADER_BYTES];
    let _filled = file.read_exact(&mut header).await?;
    let extent = Extent {
        start,
        bytes: word(&header, 4)?,
    };
    let next = extent.padded_end()?;
    if next > end {
        return Err(FileError::Container);
    }
    let kind = if header.get(..4) == Some(b"data".as_slice()) {
        Kind::Audio
    } else if header.get(..4) == Some(b"fmt ".as_slice()) {
        Kind::Format
    } else {
        Kind::Other
    };
    Ok(Chunk { kind, extent, next })
}
async fn read_format(file: &mut File, size: u32) -> Result<Pcm16Mono, FileError> {
    if size < u32::try_from(FORMAT_BYTES).map_err(FileError::Count)? {
        return Err(FileError::Format);
    }
    let mut bytes = [0; FORMAT_BYTES];
    let _filled = file.read_exact(&mut bytes).await?;
    let width = u16::try_from(SAMPLE_BYTES).map_err(FileError::Count)?;
    if half(&bytes, 0)? != PCM_ENCODING
        || half(&bytes, 2)? != MONO
        || half(&bytes, 12)? != width
        || half(&bytes, 14)? != u16::try_from(i16::BITS).map_err(FileError::Count)?
    {
        return Err(FileError::Format);
    }
    let rate = SampleRate::new(word(&bytes, 4)?).map_err(FileError::Rate)?;
    if word(&bytes, 8)? != rate.get().saturating_mul(u32::from(width)) {
        return Err(FileError::Format);
    }
    Ok(Pcm16Mono::new(rate))
}
fn check_duration(rate: SampleRate, bytes: u32) -> Result<(), FileError> {
    let width = u32::try_from(SAMPLE_BYTES).map_err(FileError::Count)?;
    let seconds = u64::from(MAX_SEQ.get()).saturating_add(1);
    let max = seconds
        .saturating_mul(u64::from(rate.get()))
        .saturating_mul(u64::from(width));
    if bytes == 0 || !bytes.is_multiple_of(width) || u64::from(bytes) > max {
        return Err(FileError::Duration);
    }
    Ok(())
}
fn word(bytes: &[u8], offset: usize) -> Result<u32, FileError> {
    let end = offset
        .checked_add(size_of::<u32>())
        .ok_or(FileError::Container)?;
    let value = bytes.get(offset..end).ok_or(FileError::Container)?;
    let value = <[u8; size_of::<u32>()]>::try_from(value).map_err(FileError::Width)?;
    Ok(u32::from_le_bytes(value))
}
fn half(bytes: &[u8], offset: usize) -> Result<u16, FileError> {
    let end = offset
        .checked_add(size_of::<u16>())
        .ok_or(FileError::Container)?;
    let value = bytes.get(offset..end).ok_or(FileError::Container)?;
    let value = <[u8; size_of::<u16>()]>::try_from(value).map_err(FileError::Width)?;
    Ok(u16::from_le_bytes(value))
}

#[cfg(test)]
pub(in crate::dictation) mod tests;
