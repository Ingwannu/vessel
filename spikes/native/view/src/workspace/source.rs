//! Bounded native source acquisition over the existing typed paged file read.
//! This checks continuation and version consistency, not immutable snapshots:
//! the legacy node's separate version/content reads retain their documented race.
use super::problem::Problem;
mod text;
use std::future::Future;
pub use text::{SourceText, SourceTooLarge};
use vessel_proto::{
    ErrorCode,
    workspace::{FileContent, FileRead, ReadFile},
};

/// Paseo's native highlighted-source ceiling.
pub const MAX_HIGHLIGHTED_BYTES: u64 = 1024 * 1024;
/// Paseo's native plain-source ceiling; larger sources are explicitly unsupported.
pub const MAX_SOURCE_BYTES: u64 = 10 * 1024 * 1024;
/// Existing file.read accepts at most one MiB per page.
const SOURCE_PAGE_BYTES: u32 = 1024 * 1024;
/// Bound round trips even if a malformed peer advances by a single byte.
const MAX_SOURCE_PAGES: usize = 64;

/// Acquire a complete source within native display budgets. Nontext and oversize
/// first pages are returned without further calls for the owner's typed fallback.
/// Every continuation retains the same original root/path and expected version.
///
/// # Errors
/// Fails without returning partial source on changed versions, malformed offsets,
/// inconsistent EOF, excess round trips, or any reader refusal. It never retries.
pub async fn read_source<R, RF>(mut ask: ReadFile, mut read: R) -> Result<FileRead, Problem>
where
    R: FnMut(ReadFile) -> RF,
    RF: Future<Output = Result<FileRead, Problem>>,
{
    if ask.offset != 0 {
        return Err(invalid());
    }
    ask.limit = Some(SOURCE_PAGE_BYTES);
    let first = read(ask.clone()).await?;
    if first.version.size > MAX_SOURCE_BYTES {
        return Ok(first);
    }
    let version = first.version.clone();
    let FileContent::Text { text, next } = first.content else {
        return Ok(first);
    };
    let mut source = text;
    let mut next = continuation(0, source.len(), next, version.size)?;
    for _ in 1..MAX_SOURCE_PAGES {
        let Some(offset) = next else {
            return Ok(FileRead {
                version,
                content: FileContent::Text {
                    text: source,
                    next: None,
                },
            });
        };
        ask.offset = offset;
        let page = read(ask.clone()).await?;
        if page.version != version {
            return Err(changed());
        }
        let FileContent::Text {
            text,
            next: following,
        } = page.content
        else {
            return Err(changed());
        };
        next = continuation(offset, text.len(), following, version.size)?;
        source.push_str(&text);
    }
    if next.is_some() {
        return Err(invalid());
    }
    Ok(FileRead {
        version,
        content: FileContent::Text {
            text: source,
            next: None,
        },
    })
}
fn continuation(
    offset: u64,
    length: usize,
    next: Option<u64>,
    size: u64,
) -> Result<Option<u64>, Problem> {
    let bytes = u64::try_from(length).map_err(|_large| invalid())?;
    if bytes > u64::from(SOURCE_PAGE_BYTES) {
        return Err(invalid());
    }
    let end = offset.checked_add(bytes).ok_or_else(invalid)?;
    if end > size || next != (end < size).then_some(end) || (next.is_some() && bytes == 0) {
        return Err(invalid());
    }
    Ok(next)
}
fn invalid() -> Problem {
    Problem::Node {
        code: ErrorCode::InvalidRequest,
        message: String::new(),
    }
}
fn changed() -> Problem {
    Problem::Node {
        code: ErrorCode::Conflict,
        message: String::new(),
    }
}
#[cfg(test)]
mod tests;
