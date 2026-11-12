//! Reading git's output for diffs: `--raw -z` and `--numstat -z` records
//! (which files, how), and one file's unified patch (its hunks). Pure
//! functions over bytes, so each is tested on recorded output.

use vessel_proto::workspace::{ChangeStatus, CommitId, DiffLine, Hunk, LineKind, RelPath};

/// The longest line text sent; the rest is cut (`DiffLine.cut`).
pub(crate) const MAX_LINE_BYTES: usize = 4096;

/// One `--raw` record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Raw {
    pub(crate) old_mode: u32,
    pub(crate) new_mode: u32,
    pub(crate) old_id: Option<CommitId>,
    pub(crate) new_id: Option<CommitId>,
    pub(crate) status: ChangeStatus,
    pub(crate) path: String,
    pub(crate) old_path: Option<String>,
}

/// Read `--raw -z` records (lines starting with `:`); other records are
/// skipped, so the same output may hold `--numstat` too.
pub(crate) fn raw_records(output: &[u8]) -> Vec<Raw> {
    let mut records = Vec::new();
    let mut fields = output.split(|byte| *byte == 0).peekable();
    while let Some(field) = fields.next() {
        let Some(header) = field.strip_prefix(b":") else {
            continue;
        };
        let header = String::from_utf8_lossy(header);
        let mut parts = header.split(' ');
        let (Some(old_mode), Some(new_mode), Some(old_id), Some(new_id), Some(letter)) = (
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
        ) else {
            continue;
        };
        let status = change_status(letter);
        let two_paths = matches!(status, ChangeStatus::Renamed | ChangeStatus::Copied);
        let Some(first) = fields.next() else { break };
        let first = String::from_utf8_lossy(first).into_owned();
        let (path, old_path) = if two_paths {
            let Some(second) = fields.next() else { break };
            (String::from_utf8_lossy(second).into_owned(), Some(first))
        } else {
            (first, None)
        };
        records.push(Raw {
            old_mode: u32::from_str_radix(old_mode, 8).unwrap_or(0),
            new_mode: u32::from_str_radix(new_mode, 8).unwrap_or(0),
            old_id: object_id(old_id),
            new_id: object_id(new_id),
            status,
            path,
            old_path,
        });
    }
    records
}

/// An object id from a raw record; `None` for the all-zero one (no object
/// on that side, or the working tree's).
fn object_id(text: &str) -> Option<CommitId> {
    if text.bytes().all(|byte| byte == b'0') {
        return None;
    }
    text.parse().ok()
}

/// A raw record's status letter (with its score, `R100`).
fn change_status(letter: &str) -> ChangeStatus {
    match letter.chars().next() {
        Some('A') => ChangeStatus::Added,
        Some('D') => ChangeStatus::Deleted,
        Some('R') => ChangeStatus::Renamed,
        Some('C') => ChangeStatus::Copied,
        Some('T') => ChangeStatus::TypeChanged,
        Some('U') => ChangeStatus::Conflicted,
        _ => ChangeStatus::Modified,
    }
}

/// Lines added and removed per path, from `--numstat -z` records (lines not
/// starting with `:`); `None` counts for a binary file.
pub(crate) fn numstat_records(output: &[u8]) -> Vec<(String, Option<(u32, u32)>)> {
    let mut records = Vec::new();
    let mut fields = output.split(|byte| *byte == 0);
    while let Some(field) = fields.next() {
        if field.is_empty() || field.starts_with(b":") {
            // A raw record's paths follow it; skip them by their count.
            if let Some(header) = field.strip_prefix(b":") {
                let letter = String::from_utf8_lossy(header)
                    .split(' ')
                    .nth(4)
                    .and_then(|letter| letter.chars().next());
                let paths = if matches!(letter, Some('R' | 'C')) {
                    2
                } else {
                    1
                };
                for _ in 0..paths {
                    let _path = fields.next();
                }
            }
            continue;
        }
        let text = String::from_utf8_lossy(field);
        let mut parts = text.splitn(3, '\t');
        let (Some(added), Some(removed), Some(path)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let counts = added.parse().ok().zip(removed.parse().ok());
        let path = if path.is_empty() {
            // A rename: the old path, then the new one, as their own fields.
            let _old = fields.next();
            fields
                .next()
                .map(|new| String::from_utf8_lossy(new).into_owned())
                .unwrap_or_default()
        } else {
            path.to_owned()
        };
        records.push((path, counts));
    }
    records
}

/// What one file's patch says, read from `git diff`'s unified output.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Patch {
    pub(crate) binary: bool,
    pub(crate) hunks: Vec<Hunk>,
    /// The index of the first hunk not kept, when `max_lines` ran out.
    pub(crate) more_from: Option<u32>,
}

/// The hunks of a one-file patch, from the `from`-th on, keeping at most
/// `max_lines` lines in all, each cut at [`MAX_LINE_BYTES`]. A hunk that
/// does not fit whole starts the next page (`more_from`), unless it is the
/// page's first: then it is cut at `max_lines` (its header still says how
/// long it is), so every page moves forward. Lines of hunks before `from`,
/// and past the bound, are never held.
pub(crate) fn hunks(output: &[u8], from: u32, max_lines: usize) -> Patch {
    let mut reader = Hunks {
        patch: Patch::default(),
        current: None,
        index: 0,
        from,
        max_lines,
        kept: 0,
        overflow: false,
    };
    for line in output.split(|byte| *byte == b'\n') {
        reader.line(line);
        if reader.patch.more_from.is_some() {
            break;
        }
    }
    reader.finish();
    reader.patch
}

/// [`hunks`]'s state.
struct Hunks {
    patch: Patch,
    current: Option<Hunk>,
    index: u32,
    from: u32,
    max_lines: usize,
    kept: usize,
    overflow: bool,
}

impl Hunks {
    fn line(&mut self, line: &[u8]) {
        if let Some(header) = line.strip_prefix(b"@@ ") {
            if self.current.is_some() {
                self.finish();
                self.index = self.index.saturating_add(1);
            }
            self.current = hunk_header(header);
            return;
        }
        let Some(hunk) = self.current.as_mut() else {
            if line.starts_with(b"Binary files ") || line.starts_with(b"GIT binary patch") {
                self.patch.binary = true;
            }
            return;
        };
        let (kind, text) = match line.first() {
            Some(b' ') => (LineKind::Context, line.get(1..).unwrap_or_default()),
            Some(b'+') => (LineKind::Added, line.get(1..).unwrap_or_default()),
            Some(b'-') => (LineKind::Removed, line.get(1..).unwrap_or_default()),
            Some(b'\\') => (LineKind::NoNewline, &b""[..]),
            // The next file's header or the output's end.
            _ => return,
        };
        if self.index < self.from {
            return;
        }
        if self.kept.saturating_add(hunk.lines.len()) >= self.max_lines {
            self.overflow = true;
            return;
        }
        hunk.lines.push(diff_line(kind, text));
    }

    /// The current hunk is complete: keep it, cut it, or start the next page
    /// with it.
    fn finish(&mut self) {
        let Some(hunk) = self.current.take() else {
            return;
        };
        let overflow = std::mem::take(&mut self.overflow);
        if self.index < self.from || self.patch.more_from.is_some() {
            return;
        }
        if overflow && !self.patch.hunks.is_empty() {
            self.patch.more_from = Some(self.index);
            return;
        }
        self.kept = self.kept.saturating_add(hunk.lines.len());
        self.patch.hunks.push(hunk);
        if overflow {
            self.patch.more_from = Some(self.index.saturating_add(1));
        }
    }
}

/// One line, without a carriage return git kept, cut at
/// [`MAX_LINE_BYTES`] on a character boundary.
pub(crate) fn diff_line(kind: LineKind, bytes: &[u8]) -> DiffLine {
    let bytes = bytes.strip_suffix(b"\r").unwrap_or(bytes);
    let text = String::from_utf8_lossy(bytes);
    let (text, cut) = cut_text(&text, MAX_LINE_BYTES);
    DiffLine { kind, text, cut }
}

/// `text` cut at `max` bytes on a character boundary, and whether it was.
pub(crate) fn cut_text(text: &str, max: usize) -> (String, bool) {
    if text.len() <= max {
        return (text.to_owned(), false);
    }
    let end = (0..=max)
        .rev()
        .find(|index| text.is_char_boundary(*index))
        .unwrap_or(0);
    (text.get(..end).unwrap_or_default().to_owned(), true)
}

/// `-a,b +c,d @@ section` after `@@ `.
fn hunk_header(header: &[u8]) -> Option<Hunk> {
    let text = String::from_utf8_lossy(header);
    let (ranges, rest) = text.split_once(" @@")?;
    let (old, new) = ranges.split_once(' ')?;
    let (old_start, old_lines) = range(old.strip_prefix('-')?)?;
    let (new_start, new_lines) = range(new.strip_prefix('+')?)?;
    let section = rest.trim();
    Some(Hunk {
        old_start,
        old_lines,
        new_start,
        new_lines,
        section: (!section.is_empty()).then(|| cut_text(section, 256).0),
        lines: Vec::new(),
    })
}

/// `start,count` or `start` (a count of one).
fn range(text: &str) -> Option<(u32, u32)> {
    match text.split_once(',') {
        Some((start, count)) => Some((start.parse().ok()?, count.parse().ok()?)),
        None => Some((text.parse().ok()?, 1)),
    }
}

/// A path git printed, as a `RelPath`; `None` for one this version cannot
/// name.
pub(crate) fn rel(path: &str) -> Option<RelPath> {
    path.parse().ok().filter(|path: &RelPath| !path.is_root())
}

#[cfg(test)]
mod tests {
    use super::*;

    const RAW: &[u8] = b":100644 100644 aaaaaaa1111111111111111111111111111111111 bbbbbbb2222222222222222222222222222222222 M\0src/lib.rs\0\
:100644 100755 ccccccc3333333333333333333333333333333333 ccccccc3333333333333333333333333333333333 M\0run.sh\0\
:100644 100644 ddddddd4444444444444444444444444444444444 ddddddd4444444444444444444444444444444444 R100\0old.txt\0new.txt\0\
:000000 100644 0000000000000000000000000000000000000000 eeeeeee5555555555555555555555555555555555 A\0added.bin\0\
3\t1\tsrc/lib.rs\0\
0\t0\trun.sh\0\
0\t0\t\0old.txt\0new.txt\0\
-\t-\tadded.bin\0";

    #[test]
    fn raw_and_numstat_records_are_read_from_one_output() {
        let raw = raw_records(RAW);
        let summary: Vec<_> = raw
            .iter()
            .map(|record| {
                (
                    record.path.as_str(),
                    record.old_path.as_deref(),
                    record.status,
                    record.old_mode,
                    record.new_mode,
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                (
                    "src/lib.rs",
                    None,
                    ChangeStatus::Modified,
                    0o100_644,
                    0o100_644
                ),
                ("run.sh", None, ChangeStatus::Modified, 0o100_644, 0o100_755),
                (
                    "new.txt",
                    Some("old.txt"),
                    ChangeStatus::Renamed,
                    0o100_644,
                    0o100_644
                ),
                ("added.bin", None, ChangeStatus::Added, 0, 0o100_644),
            ]
        );
        assert_eq!(raw[3].old_id, None, "nothing on the old side");
        let counts = numstat_records(RAW);
        assert_eq!(
            counts,
            [
                ("src/lib.rs".to_owned(), Some((3, 1))),
                ("run.sh".to_owned(), Some((0, 0))),
                ("new.txt".to_owned(), Some((0, 0))),
                ("added.bin".to_owned(), None),
            ]
        );
    }

    const PATCH: &[u8] = concat!(
        "diff --git a/f b/f\n",
        "index 1..2 100644\n",
        "--- a/f\n",
        "+++ b/f\n",
        "@@ -1,3 +1,3 @@ fn main() {\n",
        " one\n",
        "-two\n",
        "+TWO\r\n",
        " three\n",
        "@@ -10 +10,2 @@\n",
        "-ten\n",
        "+ten\n",
        "+eleven\n",
        "\\ No newline at end of file\n",
    )
    .as_bytes();

    #[test]
    fn hunks_are_typed_lines_and_page_by_hunk() {
        let patch = hunks(PATCH, 0, 5000);
        assert!(!patch.binary);
        assert_eq!(patch.more_from, None);
        let first = patch.hunks.first().unwrap();
        assert_eq!(
            (
                first.old_start,
                first.old_lines,
                first.new_start,
                first.new_lines
            ),
            (1, 3, 1, 3)
        );
        assert_eq!(first.section.as_deref(), Some("fn main() {"));
        let kinds: Vec<_> = first
            .lines
            .iter()
            .map(|line| (line.kind, line.text.as_str()))
            .collect();
        assert_eq!(
            kinds,
            [
                (LineKind::Context, "one"),
                (LineKind::Removed, "two"),
                (LineKind::Added, "TWO"),
                (LineKind::Context, "three"),
            ]
        );
        let second = &patch.hunks[1];
        assert_eq!((second.old_start, second.old_lines), (10, 1));
        assert_eq!(second.lines.last().unwrap().kind, LineKind::NoNewline);

        let paged = hunks(PATCH, 0, 5);
        assert_eq!(paged.hunks.len(), 1);
        assert_eq!(paged.more_from, Some(1));
        let rest = hunks(PATCH, 1, 5);
        assert_eq!(rest.hunks.len(), 1);
        assert_eq!(rest.hunks.first().unwrap().old_start, 10);
        assert_eq!(rest.more_from, None);
        // A first hunk longer than a page is cut, and the page moves on.
        let cut = hunks(PATCH, 0, 2);
        assert_eq!(cut.hunks.len(), 1);
        assert_eq!(cut.hunks.first().unwrap().lines.len(), 2);
        assert_eq!(cut.more_from, Some(1));
    }

    #[test]
    fn binary_patches_and_long_lines() {
        let binary = hunks(
            b"diff --git a/x b/x\nBinary files a/x and b/x differ\n",
            0,
            10,
        );
        assert!(binary.binary);
        assert!(binary.hunks.is_empty());
        let long = diff_line(LineKind::Added, "가".repeat(2000).as_bytes());
        assert!(long.cut);
        assert!(long.text.len() <= MAX_LINE_BYTES);
    }
}
