//! Optional scratch FIFO controls one recorded source read; never part of vesseld.
#[cfg(unix)]
use std::io::{Read as _, Write as _};
use std::path::Path;
use vessel_proto::limits::LimitFailure;
const GATE_NAME: &str = "source-gate";
#[cfg(unix)]
const RELEASE_BYTES: usize = 1;
#[cfg(unix)]
fn signal(message: &str) -> Result<(), LimitFailure> {
    let mut out = std::io::stdout().lock();
    out.write_all(message.as_bytes())
        .map_err(|_private| LimitFailure::UnsafeLoginStore)?;
    out.flush()
        .map_err(|_private| LimitFailure::UnsafeLoginStore)
}
#[cfg(unix)]
pub(super) fn wait(root: &Path) -> Result<(), LimitFailure> {
    use std::os::unix::fs::{FileTypeExt as _, MetadataExt as _};
    let path = root.join(GATE_NAME);
    let metadata = match path.symlink_metadata() {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_private) => return Err(LimitFailure::UnsafeLoginStore),
    };
    if !metadata.file_type().is_fifo() {
        return Err(LimitFailure::UnsafeLoginStore);
    }
    let actual = path
        .canonicalize()
        .map_err(|_private| LimitFailure::UnsafeLoginStore)?;
    if !actual.starts_with(root) {
        return Err(LimitFailure::UnsafeLoginStore);
    }
    signal("USAGE_FIXTURE_GATE_WAITING\n")?;
    let mut input =
        std::fs::File::open(actual).map_err(|_private| LimitFailure::UnsafeLoginStore)?;
    let opened = input
        .metadata()
        .map_err(|_private| LimitFailure::UnsafeLoginStore)?;
    if !opened.file_type().is_fifo()
        || opened.dev() != metadata.dev()
        || opened.ino() != metadata.ino()
    {
        return Err(LimitFailure::UnsafeLoginStore);
    }
    let mut release = [0_u8; RELEASE_BYTES];
    input
        .read_exact(&mut release)
        .map_err(|_private| LimitFailure::UnsafeLoginStore)?;
    signal("USAGE_FIXTURE_GATE_RELEASED\n")?;
    // A test callback deliberately supplies no DB token. The real headless
    // fixture sign-in is then read by the one production credential parser.
    Ok(())
}
#[cfg(not(unix))]
pub(super) fn wait(root: &Path) -> Result<(), LimitFailure> {
    if root.join(GATE_NAME).exists() {
        Err(LimitFailure::Unsupported)
    } else {
        Ok(())
    }
}
