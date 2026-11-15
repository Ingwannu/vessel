//! Startup LSP registry configuration, separated from assembling node owners.

use crate::Config;

/// The LSP manager's configuration with the operator's roots filled in:
/// executables and toolchains inside any configured root are not trusted
/// for conservative analysis, so the registry needs the full list.
pub(super) fn config(config: &Config) -> vessel_lsp::Config {
    let mut lsp = config.lsp.clone();
    lsp.registry.configured_roots = config
        .lsp_roots
        .iter()
        .map(|root| root.path.clone())
        .collect();
    lsp
}
