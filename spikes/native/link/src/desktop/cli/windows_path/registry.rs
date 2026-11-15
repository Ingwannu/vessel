//! HKCU is the only registry scope; transactions refuse concurrent replacement.

use super::{DesktopError, Entry, Operation, UserPath};
use std::os::windows::ffi::OsStrExt as _;
use std::path::Path;
use vessel_app_view::desktop::cli::Status;
use winreg::{
    RegKey, RegValue,
    enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_EXPAND_SZ, REG_SZ},
    transaction::Transaction,
};

fn entry(path: &Path) -> Result<Entry, DesktopError> {
    Entry::new(path.as_os_str().encode_wide().collect())
}

pub(in super::super) fn validate(path: &Path) -> Result<(), DesktopError> {
    entry(path).map(|_entry| ())
}

fn read(key: &RegKey) -> Result<UserPath, DesktopError> {
    let value = match key.get_raw_value("Path") {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(UserPath(Vec::new()));
        }
        Err(error) => return Err(error.into()),
    };
    if value.vtype != REG_EXPAND_SZ && value.vtype != REG_SZ {
        return Err(DesktopError::InvalidUserPath);
    }
    let path = UserPath::decode(&value.bytes)?;
    // Promoting a literal REG_SZ containing %...% would change its meaning.
    if value.vtype == REG_SZ && path.0.contains(&super::PERCENT) {
        return Err(DesktopError::InvalidUserPath);
    }
    Ok(path)
}

pub(in super::super) fn status(path: &Path) -> Result<Status, DesktopError> {
    let entry = entry(path)?;
    let key = match RegKey::predef(HKEY_CURRENT_USER).open_subkey("Environment") {
        Ok(key) => key,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Status::Absent),
        Err(error) => return Err(error.into()),
    };
    Ok(if read(&key)?.contains(&entry) {
        Status::Installed
    } else {
        Status::Absent
    })
}

pub(in super::super) fn apply(
    path: &Path,
    operation: Operation,
) -> Result<super::super::Change, DesktopError> {
    let entry = entry(path)?;
    let transaction = Transaction::new().map_err(DesktopError::UserPathTransaction)?;
    let (key, _disposition) = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey_transacted_with_flags("Environment", &transaction, KEY_READ | KEY_WRITE)?;
    let previous = read(&key)?;
    let next = previous.changed(&entry, operation)?;
    if next.0 != previous.0 {
        key.set_raw_value(
            "Path",
            &RegValue {
                vtype: REG_EXPAND_SZ,
                bytes: next.encode().into(),
            },
        )?;
        transaction.commit()?;
        Ok(super::super::Change::Changed)
    } else {
        Ok(super::super::Change::Unchanged)
    }
}
