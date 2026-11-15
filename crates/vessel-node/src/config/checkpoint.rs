//! Ignore defaults are explicit node configuration; libraries never read HOME.
//! Binaries snapshot XDG once, while in-process nodes keep their supplied home.
use super::Config;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};
use vessel_proto::checkpoint::CheckpointError;
use vessel_workspace::checkpoint::IgnoreDefaults;

pub(super) fn defaults(
    home: &Path,
    config: Option<OsString>,
) -> Result<IgnoreDefaults, CheckpointError> {
    let config = config
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    IgnoreDefaults::new(config.join("git").join("ignore")).map_err(|error| error.reason())
}
impl Config {
    /// Snapshot the binary's nonsecret ignore location without consulting the
    /// library process environment. Tests omit this and keep their explicit home.
    #[must_use]
    pub fn with_checkpoint_environment(
        mut self,
        read: impl FnOnce(&str) -> Option<OsString>,
    ) -> Self {
        self.checkpoint_ignores = defaults(self.harness_env.user_home(), read("XDG_CONFIG_HOME"));
        self
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use vessel_harness::HarnessEnv;
    use vessel_store::DataDir;
    #[test]
    fn checkpoint_defaults_do_not_escape_an_in_process_nodes_explicit_home() {
        let home = tempfile::tempdir().unwrap();
        let config = Config::new(
            DataDir::new(home.path()),
            HarnessEnv::new(home.path().to_owned()),
        );
        assert_eq!(
            config.checkpoint_ignores,
            IgnoreDefaults::new(home.path().join(".config/git/ignore"))
                .map_err(|error| error.reason())
        );
        let xdg = tempfile::tempdir().unwrap();
        let config = config.with_checkpoint_environment(|name| {
            assert_eq!(name, "XDG_CONFIG_HOME");
            Some(xdg.path().as_os_str().to_owned())
        });
        assert_eq!(
            config.checkpoint_ignores,
            IgnoreDefaults::new(xdg.path().join("git/ignore")).map_err(|error| error.reason())
        );
    }
    #[test]
    fn checkpoint_relative_environment_refuses_and_empty_environment_uses_the_explicit_home() {
        let home = tempfile::tempdir().unwrap();
        assert_eq!(
            defaults(home.path(), Some("relative".into())),
            Err(CheckpointError::InvalidPath)
        );
        assert_eq!(
            defaults(home.path(), Some(OsString::new())),
            defaults(home.path(), None)
        );
    }
}
