//! Synthetic regular files exercise preflight and bounded reads without a node.
use super::*;
use std::io::Write as _;

pub(in crate::dictation) fn format(rate: SampleRate) -> Vec<u8> {
    let width = u16::try_from(SAMPLE_BYTES).unwrap();
    let mut value = Vec::new();
    value.extend_from_slice(&PCM_ENCODING.to_le_bytes());
    value.extend_from_slice(&MONO.to_le_bytes());
    value.extend_from_slice(&rate.get().to_le_bytes());
    value.extend_from_slice(
        &rate
            .get()
            .checked_mul(u32::from(width))
            .unwrap()
            .to_le_bytes(),
    );
    value.extend_from_slice(&width.to_le_bytes());
    value.extend_from_slice(&u16::try_from(i16::BITS).unwrap().to_le_bytes());
    value
}
pub(in crate::dictation) fn wave(parts: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
    let mut output = b"RIFF\0\0\0\0WAVE".to_vec();
    for (name, data) in parts {
        output.extend_from_slice(name.as_slice());
        output.extend_from_slice(&u32::try_from(data.len()).unwrap().to_le_bytes());
        output.extend_from_slice(data);
        if !data
            .len()
            .is_multiple_of(usize::try_from(RIFF_ALIGNMENT).unwrap())
        {
            output.push(0);
        }
    }
    let length = u32::try_from(
        output
            .len()
            .checked_sub(usize::try_from(RIFF_PREFIX_BYTES).unwrap())
            .unwrap(),
    )
    .unwrap();
    output[4..8].copy_from_slice(&length.to_le_bytes());
    output
}
pub(in crate::dictation) fn file(bytes: &[u8]) -> (tempfile::TempDir, std::path::PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("recording.unrelated");
    std::fs::write(&path, bytes).unwrap();
    (directory, path)
}
fn short() -> Vec<u8> {
    1000_i16.to_le_bytes().to_vec()
}

#[tokio::test]
async fn preflight_accepts_chunk_order_padding_extensions_and_one_second_plus_tail() {
    for rate in [
        SampleRate::MIN,
        SampleRate::new(24_000).unwrap(),
        SampleRate::MAX,
    ] {
        let second = usize::try_from(rate.get())
            .unwrap()
            .checked_mul(SAMPLE_BYTES)
            .unwrap();
        let audio = 1000_i16
            .to_le_bytes()
            .repeat(usize::try_from(rate.get()).unwrap().checked_add(1).unwrap());
        let mut fmt = format(rate);
        fmt.extend_from_slice(&[0, 0]);
        let bytes = wave(&[(b"JUNK", &[7]), (b"data", &audio), (b"fmt ", &fmt)]);
        let (_directory, path) = file(&bytes);
        let mut input = FileInput::open(&path).await.unwrap();
        assert_eq!(input.format().rate(), rate);
        let full = input.next().await.unwrap().unwrap();
        assert_eq!(full.as_slice().len(), second);
        assert!(full.as_slice() == &audio[..second]);
        let tail = input.next().await.unwrap().unwrap();
        assert_eq!(tail.as_slice().len(), SAMPLE_BYTES);
        assert!(input.next().await.unwrap().is_none());
    }
}

#[tokio::test]
async fn duplicate_and_missing_chunks_are_refused_even_after_the_audio_extent() {
    let fmt = format(SampleRate::MIN);
    let audio = short();
    for parts in [
        vec![
            (b"fmt ", fmt.as_slice()),
            (b"data", audio.as_slice()),
            (b"fmt ", fmt.as_slice()),
        ],
        vec![
            (b"fmt ", fmt.as_slice()),
            (b"data", audio.as_slice()),
            (b"data", audio.as_slice()),
        ],
    ] {
        let (_directory, path) = file(&wave(&parts));
        assert!(matches!(
            FileInput::open(&path).await,
            Err(FileError::Duplicate)
        ));
    }
    let (_directory, path) = file(&wave(&[(b"data", &audio)]));
    assert!(matches!(
        FileInput::open(&path).await,
        Err(FileError::Format)
    ));
    let (_directory, path) = file(&wave(&[(b"fmt ", &fmt)]));
    assert!(matches!(
        FileInput::open(&path).await,
        Err(FileError::Duration)
    ));
}

#[tokio::test]
async fn every_format_field_is_checked_before_admission() {
    let original = format(SampleRate::MIN);
    for (offset, value) in [(0_usize, 3_u16), (2, 2), (12, 4), (14, 8)] {
        let mut fmt = original.clone();
        fmt[offset..offset.checked_add(size_of::<u16>()).unwrap()]
            .copy_from_slice(&value.to_le_bytes());
        let (_directory, path) = file(&wave(&[(b"fmt ", &fmt), (b"data", &short())]));
        assert!(matches!(
            FileInput::open(&path).await,
            Err(FileError::Format)
        ));
    }
    let mut fmt = original.clone();
    fmt[8..12].copy_from_slice(&0_u32.to_le_bytes());
    let (_directory, path) = file(&wave(&[(b"fmt ", &fmt), (b"data", &short())]));
    assert!(matches!(
        FileInput::open(&path).await,
        Err(FileError::Format)
    ));
    for invalid in [
        SampleRate::MIN.get().checked_sub(1).unwrap(),
        SampleRate::MAX.get().checked_add(1).unwrap(),
    ] {
        let mut fmt = original.clone();
        fmt[4..8].copy_from_slice(&invalid.to_le_bytes());
        let (_directory, path) = file(&wave(&[(b"fmt ", &fmt), (b"data", &short())]));
        assert!(matches!(
            FileInput::open(&path).await,
            Err(FileError::Rate(_))
        ));
    }
}

#[tokio::test]
async fn declared_extents_padding_and_metadata_truncation_cannot_be_seeked_past() {
    let fmt = format(SampleRate::MIN);
    let original = wave(&[(b"fmt ", &fmt), (b"data", &short())]);
    for bytes in [
        original[..original.len().checked_sub(1).unwrap()].to_vec(),
        b"RIFX\0\0\0\0WAVE".to_vec(),
        wave(&[(b"fmt ", &[0, 0]), (b"data", &short())]),
    ] {
        let (_directory, path) = file(&bytes);
        assert!(FileInput::open(&path).await.is_err());
    }
    let mut missing_padding = wave(&[(b"fmt ", &fmt), (b"data", &short()), (b"JUNK", &[1])]);
    let _removed = missing_padding.pop();
    let declared = u32::try_from(
        missing_padding
            .len()
            .checked_sub(usize::try_from(RIFF_PREFIX_BYTES).unwrap())
            .unwrap(),
    )
    .unwrap();
    missing_padding[4..8].copy_from_slice(&declared.to_le_bytes());
    let (_directory, path) = file(&missing_padding);
    assert!(matches!(
        FileInput::open(&path).await,
        Err(FileError::Container)
    ));
    assert!(matches!(
        Extent {
            start: u64::MAX,
            bytes: 1
        }
        .end(),
        Err(FileError::Container)
    ));
    assert!(matches!(
        Extent {
            start: u64::MAX.checked_sub(1).unwrap(),
            bytes: 1
        }
        .padded_end(),
        Err(FileError::Container)
    ));
}

#[tokio::test]
async fn metadata_budget_counts_unknown_chunks_and_format_extensions() {
    let fmt = format(SampleRate::MIN);
    let overhead = CONTAINER_HEADER_BYTES
        .checked_add(CHUNK_HEADER_BYTES.checked_mul(3).unwrap())
        .unwrap()
        .checked_add(FORMAT_BYTES)
        .unwrap();
    let allowed = usize::try_from(MAX_METADATA_BYTES)
        .unwrap()
        .checked_sub(overhead)
        .unwrap();
    let filler = vec![0; allowed];
    let (_directory, path) = file(&wave(&[
        (b"fmt ", &fmt),
        (b"JUNK", &filler),
        (b"data", &short()),
    ]));
    assert!(FileInput::open(&path).await.is_ok());
    let excess = vec![0; allowed.checked_add(SAMPLE_BYTES).unwrap()];
    let audio = short();
    let enormous_fmt = vec![0; usize::try_from(MAX_METADATA_BYTES).unwrap()];
    for parts in [
        vec![
            (b"fmt ", fmt.as_slice()),
            (b"JUNK", excess.as_slice()),
            (b"data", audio.as_slice()),
        ],
        vec![
            (b"fmt ", enormous_fmt.as_slice()),
            (b"data", audio.as_slice()),
        ],
    ] {
        let (_directory, path) = file(&wave(&parts));
        assert!(matches!(
            FileInput::open(&path).await,
            Err(FileError::Metadata)
        ));
    }
}

#[tokio::test]
async fn duration_and_empty_odd_samples_are_refused_without_reading_a_recording() {
    let rate = SampleRate::MIN;
    let fmt = format(rate);
    for audio in [&[][..], &[0][..]] {
        let (_directory, path) = file(&wave(&[(b"fmt ", &fmt), (b"data", audio)]));
        assert!(matches!(
            FileInput::open(&path).await,
            Err(FileError::Duration)
        ));
    }
    let width = u32::try_from(SAMPLE_BYTES).unwrap();
    let maximum = MAX_SEQ
        .get()
        .checked_add(1)
        .unwrap()
        .checked_mul(rate.get())
        .unwrap()
        .checked_mul(width)
        .unwrap();
    assert!(check_duration(rate, maximum).is_ok());
    let excessive = maximum.checked_add(width).unwrap();
    let mut bytes = wave(&[(b"fmt ", &fmt), (b"data", &[])]);
    let data_offset = u64::try_from(bytes.len()).unwrap();
    let length_offset = bytes.len().checked_sub(size_of::<u32>()).unwrap();
    bytes[length_offset..].copy_from_slice(&excessive.to_le_bytes());
    let container = u32::try_from(data_offset.checked_sub(RIFF_PREFIX_BYTES).unwrap())
        .unwrap()
        .checked_add(excessive)
        .unwrap();
    bytes[4..8].copy_from_slice(&container.to_le_bytes());
    let (_directory, path) = file(&bytes);
    std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(data_offset.checked_add(u64::from(excessive)).unwrap())
        .unwrap();
    assert!(matches!(
        FileInput::open(&path).await,
        Err(FileError::Duration)
    ));
}

#[tokio::test]
async fn growth_is_ignored_and_a_later_truncation_is_an_io_failure() {
    let fmt = format(SampleRate::MIN);
    let original = wave(&[(b"fmt ", &fmt), (b"data", &short())]);
    let (_directory, path) = file(&original);
    let mut input = FileInput::open(&path).await.unwrap();
    std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(&short())
        .unwrap();
    assert_eq!(
        input.next().await.unwrap().unwrap().as_slice().len(),
        SAMPLE_BYTES
    );
    assert!(input.next().await.unwrap().is_none());
    let mut input = FileInput::open(&path).await.unwrap();
    std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(0)
        .unwrap();
    assert!(matches!(input.next().await, Err(FileError::Io(_))));
    assert!(matches!(
        FileInput::open(path.parent().unwrap()).await,
        Err(FileError::NotRegular)
    ));
}
