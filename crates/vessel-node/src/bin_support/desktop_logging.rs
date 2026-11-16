//! Managed nodes retain their existing private Store log after launcher exit.
//!
//! [Decision Log]
//! - Purpose: a kept node must not depend on the app's stderr pipe for diagnostics.
//! - Reuse: `DataDir` owns the same filename, private modes and rotation as attach.
//! - Choice: only explicit desktop-managed mode tees formatted records to that
//!   file; other CLI modes preserve stderr-only logging. The file opens once per
//!   record, so the existing size rotation also applies during a long app run.
//! - Tradeoff: synchronous small diagnostic writes; logging failure never stops a node.

use std::fs::File;
use std::io::{self, Write as _};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use tracing_subscriber::fmt::MakeWriter;
use vessel_store::DataDir;

#[derive(Clone)]
struct Writer {
    managed: Option<Arc<Mutex<DataDir>>>,
}

struct Record<'a> {
    file: Option<File>,
    _serial: Option<MutexGuard<'a, DataDir>>,
    stderr: io::Stderr,
}

impl<'a> MakeWriter<'a> for Writer {
    type Writer = Record<'a>;
    fn make_writer(&'a self) -> Self::Writer {
        let serial = self
            .managed
            .as_ref()
            .and_then(|managed| managed.lock().ok());
        let file = serial
            .as_ref()
            .and_then(|directory| directory.open_node_log().ok().map(|(_path, file)| file));
        Record {
            file,
            _serial: serial,
            stderr: io::stderr(),
        }
    }
}

impl io::Write for Record<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        // The app may deliberately close stderr on keep-running quit. A broken
        // app pipe must not prevent the node from writing its retained log.
        if let Some(file) = &mut self.file {
            let saved = file.write_all(bytes);
            let printed = self.stderr.write_all(bytes);
            if saved.is_ok() || printed.is_ok() {
                Ok(bytes.len())
            } else {
                saved.map(|()| bytes.len())
            }
        } else {
            self.stderr.write(bytes)
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        if let Some(file) = &mut self.file {
            let saved = file.flush();
            let printed = self.stderr.flush();
            if saved.is_ok() || printed.is_ok() {
                return Ok(());
            }
            return saved;
        }
        self.stderr.flush()
    }
}

pub(super) fn init(attaching: bool, desktop: bool, root: Option<&Path>) {
    if !desktop {
        crate::daemon_logging::init(attaching);
        return;
    }
    let managed = (if desktop {
        root.map(Path::to_path_buf).or_else(DataDir::default_root)
    } else {
        None
    })
    .map(DataDir::new)
    .and_then(|directory| {
        let result = directory.prepare().and_then(|()| directory.open_node_log());
        match result {
            Ok((_path, file)) => {
                drop(file);
                Some(Arc::new(Mutex::new(directory)))
            }
            Err(error) => {
                let _failed =
                    writeln!(io::stderr().lock(), "managed node log unavailable: {error}");
                None
            }
        }
    });
    // Keep the binary owner's independent portable_pty diagnostic upper bound
    // even when the managed node tees into its retained private Store log.
    crate::daemon_logging::init_with_writer(attaching, Writer { managed }, false);
}
