//! Bounded assembly of pages from one locally correlated file read.
//!
//! [Decision Log]
//! - Reuse `FileDiff` and the node's hunk offsets; no patch parser or second row
//!   model. The page's diff-rows.test.ts stable-key case remains the projection.
//! - A continuation must match the accepted body's ticket, hunk count and file
//!   metadata. Neither equal text nor a path alone proves that lineage.
//! - Keep every `FileDiff` field and every typed line marker. Limit retained
//!   strings and hunk count before appending; these are payload bounds, not a
//!   transport or total-memory bound. Global/editor retention is still separate.
//! - Mutable working files can change between calls: reject detectable changes
//!   and require refresh, without claiming an atomic filesystem snapshot.
use super::{FileTicket, ReadFailure};
use crate::workspace::git::RequestKey;
use vessel_proto::workspace::{DiffContent, FileDiff, GitHunkOffset};

/// Maximum retained UTF-8 field bytes in one assembled file preview.
/// The node bounds raw patches at four MiB; allow encoded text metadata while
/// refusing an unlimited sequence of individually bounded pages.
pub const MAX_FILE_TEXT_BYTES: usize = 8 * 1024 * 1024;
/// Bound per-hunk overhead as well as line strings, including empty hunks.
pub const MAX_FILE_HUNKS: usize = 32_768;
/// Empty source strings still consume line objects; bound that overhead too.
pub const MAX_FILE_LINES: usize = 100_000;

/// Typed reason that a page cannot safely extend the displayed file.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PageMismatch {
    /// The retained data is not the prefix the request asked to extend.
    #[error("the requested hunk offset does not match the retained prefix")]
    Offset,
    /// The node must continue immediately after the hunks it actually sent.
    #[error("the continuation does not follow the returned hunks")]
    Continuation,
    /// Renaming, status or mode changed while the file was being paged.
    #[error("file metadata changed between pages")]
    Metadata,
    /// A text continuation cannot silently replace earlier hunks with a notice.
    #[error("file content kind changed between pages")]
    Content,
}

/// An accepted body retains its receipt as well as its complete wire facts.
#[derive(Debug)]
pub struct FilePages {
    file: FileDiff,
    ticket: FileTicket,
}
impl FilePages {
    /// Original file metadata and all accepted hunks, with the latest continuation.
    #[must_use]
    pub const fn file(&self) -> &FileDiff {
        &self.file
    }

    /// Receipt for this immutable assembled snapshot, distinct from its path.
    #[must_use]
    pub const fn ticket(&self) -> &FileTicket {
        &self.ticket
    }

    pub(super) const fn key(&self) -> RequestKey {
        self.ticket.request_key()
    }

    pub(super) fn accept(
        previous: Option<&Self>,
        ticket: &FileTicket,
        mut page: FileDiff,
    ) -> Result<Self, ReadFailure> {
        validate_page(&page, ticket.from_hunk())?;
        if ticket.from_hunk().get() != 0 {
            let before = previous.ok_or(ReadFailure::Page(PageMismatch::Offset))?;
            if before.file.more_from != Some(ticket.from_hunk().get()) {
                return Err(ReadFailure::Page(PageMismatch::Offset));
            }
            if before.file.path != page.path
                || before.file.old_path != page.old_path
                || before.file.status != page.status
                || before.file.mode != page.mode
            {
                return Err(ReadFailure::Page(PageMismatch::Metadata));
            }
            let (DiffContent::Hunks { hunks: kept }, DiffContent::Hunks { hunks: added }) =
                (&before.file.content, &page.content)
            else {
                return Err(ReadFailure::Page(PageMismatch::Content));
            };
            if usize::try_from(ticket.from_hunk().get()).ok() != Some(kept.len()) {
                return Err(ReadFailure::Page(PageMismatch::Offset));
            }
            // Saturation refuses overflow as too large, never wraps into admission.
            if kept.len().saturating_add(added.len()) > MAX_FILE_HUNKS
                || line_count(kept).saturating_add(line_count(added)) > MAX_FILE_LINES
                || text_bytes(&before.file).saturating_add(text_bytes(&page)) > MAX_FILE_TEXT_BYTES
            {
                return Err(ReadFailure::TooLarge);
            }
            let DiffContent::Hunks { hunks: added } = &mut page.content else {
                return Err(ReadFailure::Page(PageMismatch::Content));
            };
            let mut combined = kept.clone();
            combined.append(added);
            *added = combined;
        }
        Ok(Self {
            file: page,
            ticket: ticket.clone(),
        })
    }
}

fn validate_page(page: &FileDiff, from: GitHunkOffset) -> Result<(), ReadFailure> {
    if text_bytes(page) > MAX_FILE_TEXT_BYTES {
        return Err(ReadFailure::TooLarge);
    }
    match &page.content {
        DiffContent::Hunks { hunks } => {
            if hunks.len() > MAX_FILE_HUNKS || line_count(hunks) > MAX_FILE_LINES {
                return Err(ReadFailure::TooLarge);
            }
            if let Some(more) = page.more_from {
                let count = u32::try_from(hunks.len()).map_err(|_| ReadFailure::TooLarge)?;
                if count == 0 || from.get().checked_add(count) != Some(more) {
                    return Err(ReadFailure::Page(PageMismatch::Continuation));
                }
            }
        }
        DiffContent::Binary
        | DiffContent::Submodule { .. }
        | DiffContent::Unchanged
        | DiffContent::TooLarge { .. }
        | DiffContent::Unknown => {
            if page.more_from.is_some() {
                return Err(ReadFailure::Page(PageMismatch::Continuation));
            }
        }
    }
    Ok(())
}

fn text_bytes(file: &FileDiff) -> usize {
    let paths = file
        .path
        .as_str()
        .len()
        .saturating_add(file.old_path.as_ref().map_or(0, |path| path.as_str().len()));
    let hunks = match &file.content {
        DiffContent::Hunks { hunks } => hunks.as_slice(),
        DiffContent::Binary
        | DiffContent::Submodule { .. }
        | DiffContent::Unchanged
        | DiffContent::TooLarge { .. }
        | DiffContent::Unknown => &[],
    };
    hunks.iter().fold(paths, |bytes, hunk| {
        hunk.lines.iter().fold(
            bytes.saturating_add(hunk.section.as_ref().map_or(0, String::len)),
            |bytes, line| bytes.saturating_add(line.text.len()),
        )
    })
}

fn line_count(hunks: &[vessel_proto::workspace::Hunk]) -> usize {
    hunks.iter().fold(0_usize, |count, hunk| {
        count.saturating_add(hunk.lines.len())
    })
}
