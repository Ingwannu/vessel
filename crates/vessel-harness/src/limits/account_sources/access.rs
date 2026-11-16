//! Capabilities are supplied by a binary; construction reads neither environment credentials nor files.
use std::{
    ffi::OsString,
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};
use vessel_proto::limits::LimitFailure;
use zeroize::Zeroizing;
type Environment = dyn Fn(&str) -> Option<OsString> + Send + Sync;
type CursorDatabase =
    dyn Fn(&Path) -> Result<Option<Zeroizing<String>>, LimitFailure> + Send + Sync;
#[cfg(feature = "fakes")]
type RecordedHttp = dyn Fn() -> futures_util::future::BoxFuture<'static, Result<Vec<u8>, LimitFailure>>
    + Send
    + Sync;
/// Read capabilities used only after explicit source consent and refresh admission.
#[derive(Clone)]
pub struct AccountSourceAccess {
    pub(super) home: PathBuf,
    pub(super) environment: Arc<Environment>,
    pub(super) cursor: Arc<CursorDatabase>,
    #[cfg(feature = "fakes")]
    pub(super) recorded_http: Option<Arc<RecordedHttp>>,
}
impl fmt::Debug for AccountSourceAccess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AccountSourceAccess([owning-machine capabilities])")
    }
}
impl AccountSourceAccess {
    /// The binary gives its own account home and a deferred environment reader, never token values.
    #[must_use]
    pub fn new(
        home: PathBuf,
        environment: impl Fn(&str) -> Option<OsString> + Send + Sync + 'static,
    ) -> Self {
        Self {
            home,
            environment: Arc::new(environment),
            cursor: Arc::new(|_path| Err(LimitFailure::Unsupported)),
            #[cfg(feature = "fakes")]
            recorded_http: None,
        }
    }
    /// The single `SQLite` owner's fixed read-only adapter; never an arbitrary query from a client.
    #[must_use]
    pub fn with_cursor_reader(
        mut self,
        reader: impl Fn(&Path) -> Result<Option<Zeroizing<String>>, LimitFailure>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        self.cursor = Arc::new(reader);
        self
    }
    /// Network-free recorded transport for lifecycle tests, through the same
    /// cancellation boundary as live HTTP. Only a Recorded probe invokes it;
    /// installed node arguments never select this fakes-only capability.
    #[cfg(feature = "fakes")]
    #[must_use]
    pub fn with_recorded_http(
        mut self,
        reader: impl Fn() -> futures_util::future::BoxFuture<'static, Result<Vec<u8>, LimitFailure>>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        self.recorded_http = Some(Arc::new(reader));
        self
    }
    pub(super) fn value(&self, name: &str) -> Option<Zeroizing<String>> {
        (self.environment)(name)
            .and_then(|value| value.into_string().ok())
            .map(Zeroizing::new)
            .filter(|value| !value.trim().is_empty())
    }
    pub(super) fn absolute(&self, name: &str, fallback: PathBuf) -> Result<PathBuf, LimitFailure> {
        match (self.environment)(name) {
            Some(path) => {
                let path = PathBuf::from(path);
                if path.is_absolute() {
                    Ok(path)
                } else {
                    Err(LimitFailure::UnsafeLoginStore)
                }
            }
            None => Ok(fallback),
        }
    }
}
