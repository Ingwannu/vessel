//! The one `SQLite` owner supplies Cursor's fixed read-only credential query to the source adapter.
mod decode;
use crate::StoreError;
use rusqlite::{Connection, OpenFlags, OptionalExtension as _};
use std::{io, path::Path};
use zeroize::Zeroizing;
const MAX_CURSOR_SIGN_IN_BYTES: usize = 65_536;
#[derive(serde::Deserialize)]
struct Legacy {
    #[serde(rename = "accessToken")]
    access_token: Option<Secret>,
}
#[derive(serde::Deserialize)]
#[serde(from = "String")]
struct Secret(Zeroizing<String>);
impl From<String> for Secret {
    fn from(value: String) -> Self {
        Self(Zeroizing::new(value))
    }
}
/// Read only Cursor's known `ItemTable` keys; never create, migrate or modify the database.
///
/// Active WAL is refused because immutable read-only mode cannot safely see it. The source
/// may try its separately disclosed headless store. No `SQLite` dependency crosses this seam.
/// # Errors
/// Unsafe location, active WAL, invalid database/schema or oversized credential value.
pub fn read_cursor_usage_sign_in(path: &Path) -> Result<Option<Zeroizing<String>>, StoreError> {
    let mut wal = path.as_os_str().to_os_string();
    wal.push("-wal");
    match std::fs::metadata(&wal) {
        Ok(metadata) if metadata.len() > 0 => {
            return Err(refused(path, "active Cursor WAL cannot be read immutably"));
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(StoreError::io(path)(error)),
    }
    let canonical = path.canonicalize().map_err(StoreError::io(path))?;
    if !canonical.is_file() {
        return Err(refused(
            path,
            "Cursor sign-in database must be a regular file",
        ));
    }
    let mut uri = url::Url::from_file_path(&canonical)
        .map_err(|()| refused(path, "Cursor database path is not an absolute file path"))?;
    uri.set_query(Some("mode=ro&immutable=1"));
    let connection = Connection::open_with_flags(
        uri.as_str(),
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|cause| StoreError::io(path)(io::Error::other(cause)))?;
    if let Some(value) = row(&connection, path, "cursorAuth/accessToken")? {
        let token = std::str::from_utf8(&value)
            .map(str::to_owned)
            .map(Zeroizing::new)
            .map_err(|cause| StoreError::io(path)(io::Error::other(cause)))?;
        if !token.trim().is_empty() {
            return Ok(Some(token));
        }
    }
    let Some(value) = row(&connection, path, "cursorAuthStatus")? else {
        return Ok(None);
    };
    let mut legacy: Legacy = serde_json::from_slice(&value).map_err(|cause| {
        StoreError::io(path)(io::Error::other(decode::CursorDecodeFailure::new(cause)))
    })?;
    Ok(legacy
        .access_token
        .take()
        .map(|secret| secret.0)
        .filter(|token| !token.trim().is_empty()))
}
fn refused(path: &Path, reason: &'static str) -> StoreError {
    StoreError::io(path)(io::Error::other(reason))
}
fn row(
    connection: &Connection,
    path: &Path,
    key: &str,
) -> Result<Option<Zeroizing<Vec<u8>>>, StoreError> {
    let maximum = i64::try_from(MAX_CURSOR_SIGN_IN_BYTES)
        .map_err(|cause| StoreError::io(path)(io::Error::other(cause)))?;
    let result:Option<(i64,Vec<u8>)>=connection.query_row("SELECT length(CAST(value AS BLOB)), substr(CAST(value AS BLOB), 1, ?2) FROM ItemTable WHERE key = ?1",rusqlite::params![key,maximum.saturating_add(1)],|row|Ok((row.get(0)?,row.get(1)?))).optional().map_err(|cause|StoreError::io(path)(io::Error::other(cause)))?;
    let Some((length, bytes)) = result else {
        return Ok(None);
    };
    let bytes = Zeroizing::new(bytes);
    if length > maximum {
        return Err(refused(
            path,
            "Cursor sign-in value exceeds its supported size",
        ));
    }
    Ok(Some(bytes))
}
#[cfg(test)]
mod tests;
