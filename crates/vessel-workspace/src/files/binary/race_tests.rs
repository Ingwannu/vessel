//! A real in-place writer changes A→B→A at read/seek barriers, without sleeps.
use super::*;
use sha2::{Digest as _, Sha256};
use std::io::{self, Read, Seek, SeekFrom};
use std::sync::{Arc, Barrier};
use vessel_proto::workspace::ContentHash;

struct RacingFile {
    file: std::fs::File,
    first: bool,
    seeks: usize,
    restored: bool,
    changed: Arc<Barrier>,
    changed_ready: Arc<Barrier>,
    restore: Arc<Barrier>,
    restored_ready: Arc<Barrier>,
}

impl Read for RacingFile {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let read = self.file.read(bytes)?;
        if self.first && read > 0 {
            self.first = false;
            // The caller already holds A bytes. Disk now becomes B, precisely
            // between the old first hash and its separate requested-page read.
            let _wait = self.changed.wait();
            let _wait = self.changed_ready.wait();
        }
        Ok(read)
    }
}

impl Seek for RacingFile {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        // This fixture only counts barrier phases; saturation avoids an
        // unchecked counter without changing any scheduled interleaving.
        self.seeks = self.seeks.saturating_add(1);
        if self.seeks == 3 {
            // A split hash/page/hash implementation reaches this third seek;
            // restore A before its final hash, exposing its labelled-B defect.
            let _wait = self.restore.wait();
            let _wait = self.restored_ready.wait();
            self.restored = true;
        }
        self.file.seek(position)
    }
}

#[test]
fn in_place_a_b_a_cannot_substitute_a_page_under_the_original_digest() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("race.bin");
    let original = vec![b'A'; 128];
    std::fs::write(&path, &original).unwrap();
    let changed = Arc::new(Barrier::new(2));
    let changed_ready = Arc::new(Barrier::new(2));
    let restore = Arc::new(Barrier::new(2));
    let restored_ready = Arc::new(Barrier::new(2));
    let mut reader = RacingFile {
        file: std::fs::File::open(&path).unwrap(),
        first: true,
        seeks: 0,
        restored: false,
        changed: Arc::clone(&changed),
        changed_ready: Arc::clone(&changed_ready),
        restore: Arc::clone(&restore),
        restored_ready: Arc::clone(&restored_ready),
    };
    std::thread::scope(|scope| {
        let writer = scope.spawn(|| {
            let _wait = changed.wait();
            std::fs::write(&path, vec![b'B'; 128]).unwrap();
            let _wait = changed_ready.wait();
            let _wait = restore.wait();
            std::fs::write(&path, &original).unwrap();
            let _wait = restored_ready.wait();
        });
        let page = read_page(
            &mut reader,
            &"race.bin".parse().unwrap(),
            FileOffset::ZERO,
            None,
        );
        if !reader.restored {
            let _wait = restore.wait();
            let _wait = restored_ready.wait();
        }
        writer.join().unwrap();
        let page = page.unwrap();
        assert_eq!(page.bytes.bytes().as_ref(), original);
        assert_eq!(
            page.revision.hash,
            ContentHash::from_sha256(Sha256::digest(&original).into())
        );
    });
    assert_eq!(std::fs::read(path).unwrap(), original);
}
