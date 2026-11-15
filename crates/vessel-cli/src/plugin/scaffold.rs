//! Exclusive writes protect an existing project, including concurrent writers.
//!
//! [Decision Log]
//! - Purpose: ADR0079 needs a runnable author-port format rather than React files.
//! - Reuse: canonical PluginId/ClientDocument and W17's typed output formatter.
//! - Choice: validate templates first, then require an empty ordinary directory
//!   and exclusively create every fixed file. Never recursively clean a failure.
//! - Why: replacing a directory or rolling back by pathname could remove a
//!   concurrent writer's work. A failed write may leave our partial scaffold;
//!   existing content and existing directory permissions are preserved.
use super::template;
use crate::parity::output::Row;
use serde::Serialize;
use std::{
    fs, io,
    io::Write as _,
    path::{Path, PathBuf},
};
use vessel_proto::plugins::{InvalidPluginValue, PluginId};

#[derive(Debug, thiserror::Error)]
pub(crate) enum InitError {
    #[error("pass --id when the directory has no valid plugin identifier")]
    Identifier(#[source] InvalidPluginValue),
    #[error("plugin scaffold needs a local directory with a final component")]
    Directory,
    #[error("plugin scaffold target must be an empty ordinary directory")]
    NotEmpty,
    #[error("plugin scaffold filesystem operation failed; existing files are not overwritten")]
    Io(#[source] io::Error),
    #[error("plugin scaffold document could not be encoded")]
    Encode(#[source] serde_json::Error),
    #[error("plugin scaffold document failed validation")]
    Document(#[source] vessel_proto::plugins::DocumentError),
}

#[derive(Debug, Serialize)]
pub(super) struct Created {
    id: PluginId,
    directory: PathBuf,
}
impl Row for Created {
    fn id(&self) -> String {
        self.id.as_str().to_owned()
    }
    fn headers() -> &'static [&'static str] {
        &["PLUGIN", "DIRECTORY"]
    }
    fn cells(&self) -> Vec<String> {
        vec![self.id(), self.directory.display().to_string()]
    }
}

pub(super) fn create(directory: &Path, requested: Option<PluginId>) -> Result<Created, InitError> {
    let name = directory.file_name().and_then(|name| name.to_str());
    let id = match requested {
        Some(id) => id,
        None => name
            .ok_or(InitError::Directory)?
            .parse()
            .map_err(InitError::Identifier)?,
    };
    // Prepare all documents before the first filesystem mutation.
    let files = template::files(&id)?;
    prepare_directory(directory)?;
    for (name, bytes) in files {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        let mut file = options.open(directory.join(name)).map_err(InitError::Io)?;
        file.write_all(&bytes).map_err(InitError::Io)?;
        file.sync_all().map_err(InitError::Io)?;
    }
    Ok(Created {
        id,
        directory: fs::canonicalize(directory).map_err(InitError::Io)?,
    })
}

fn prepare_directory(directory: &Path) -> Result<(), InitError> {
    match fs::symlink_metadata(directory) {
        Ok(metadata) if !metadata.file_type().is_dir() => return Err(InitError::NotEmpty),
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let parent = directory
                .parent()
                .filter(|path| !path.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."));
            fs::create_dir_all(parent).map_err(InitError::Io)?;
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt as _;
                builder.mode(0o700);
            }
            // A concurrent target creation refuses instead of acquiring it.
            builder.create(directory).map_err(InitError::Io)?;
        }
        Err(error) => return Err(InitError::Io(error)),
    }
    if fs::read_dir(directory)
        .map_err(InitError::Io)?
        .next()
        .transpose()
        .map_err(InitError::Io)?
        .is_some()
    {
        return Err(InitError::NotEmpty);
    }
    Ok(())
}
