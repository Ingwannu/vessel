//! Which file is the node's log, and its tail's bounds.

use std::io::Write as _;

use super::*;

#[cfg(unix)]
#[test]
fn the_log_is_the_file_a_stream_writes_to_not_one_that_merely_exists() {
    use std::os::fd::AsRawFd as _;
    let folder = tempfile::tempdir().unwrap();
    let old = folder.path().join("vesseld.log");
    let service = folder.path().join("vessel-service.log");
    std::fs::write(&old, "an old node's lines\n").unwrap();
    let open = std::fs::File::create(&service).unwrap();
    let stream = PathBuf::from(format!("/dev/fd/{}", open.as_raw_fd()));
    let files = [old.clone(), service.clone()];
    assert_eq!(log_file(&stream, &files), Some(service));
    // A stream that is neither (this test's own `/dev/null`).
    let null = std::fs::File::open("/dev/null").unwrap();
    let elsewhere = PathBuf::from(format!("/dev/fd/{}", null.as_raw_fd()));
    assert_eq!(log_file(&elsewhere, &files), None);
    drop(open);
}

#[test]
fn a_tail_keeps_the_last_lines_and_says_what_it_left_out() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("vesseld.log");
    std::fs::write(&path, "one\ntwo\nthree\n").unwrap();
    assert_eq!(
        tail(&path, 2).unwrap(),
        (vec!["two".to_owned(), "three".to_owned()], true)
    );
    assert_eq!(
        tail(&path, 100).unwrap(),
        (
            vec!["one".to_owned(), "two".to_owned(), "three".to_owned()],
            false
        )
    );
    std::fs::write(&path, "").unwrap();
    assert_eq!(tail(&path, 100).unwrap(), (Vec::new(), false));
}

#[test]
fn a_tail_reads_only_the_end_of_a_large_log_and_cuts_long_lines() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("vesseld.log");
    let mut file = std::fs::File::create(&path).unwrap();
    // 200 KiB of short lines, then one line longer than the bound.
    for index in 0..20_000 {
        writeln!(file, "line {index:05}").unwrap();
    }
    writeln!(file, "{}", "x".repeat(MAX_LOG_LINE_BYTES * 2)).unwrap();
    drop(file);
    let (lines, cut) = tail(&path, MAX_LOG_LINES).unwrap();
    assert!(cut);
    let total: usize = lines.iter().map(String::len).sum();
    assert!(total <= usize::try_from(MAX_LOG_TAIL_BYTES).unwrap());
    assert_eq!(lines.last().unwrap().len(), MAX_LOG_LINE_BYTES);
    assert_eq!(lines.len(), usize::try_from(MAX_LOG_LINES).unwrap());
    assert!(lines.first().unwrap().starts_with("line "));
}
