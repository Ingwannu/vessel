//! Checked canonical construction for local view fixtures, never a transport.
use super::{CatalogBudget, Installation, Problem};
use vessel_proto::{
    NodeId, PROTOCOL_VERSION,
    plugins::{CatalogError, ClientCatalog, ClientInstallation},
};

pub(super) fn catalog(node: NodeId, entries: Vec<Installation>) -> Result<ClientCatalog, Problem> {
    let entries = entries
        .into_iter()
        .map(|entry| {
            ClientInstallation::new(
                entry.address().clone(),
                PROTOCOL_VERSION,
                entry.document().clone(),
            )
            .map_err(|_invalid| Problem::InvalidCatalog)
        })
        .collect::<Result<Vec<_>, _>>()?;
    ClientCatalog::new(node, entries).map_err(|error| match error {
        CatalogError::Installations => Problem::Capacity(CatalogBudget::Installations),
        CatalogError::Themes => Problem::Capacity(CatalogBudget::Themes),
        CatalogError::Bytes
        | CatalogError::Decode(_)
        | CatalogError::Encode(_)
        | CatalogError::Requirement
        | CatalogError::Owner
        | CatalogError::Duplicate => Problem::InvalidCatalog,
    })
}
