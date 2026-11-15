//! Transfers (protocol 6, F4): exports served, imports pulled from this
//! node or another fleet node through the envelope. Routing only.

use vessel_proto::{
    Ack, BlobChunk, CloseTransfer, ExportMemory, ExportSession, ImportMemory, ImportSession,
    MemoryImported, ReadTransfer, SessionId, SessionImported, TransferManifest, WireError,
};

use crate::error::ServiceError;
use crate::transfers::Holder;
use crate::transfers::export::Exporter;
use crate::transfers::import::{Importer, Local, Pull, Remote};

use super::{Connection, NodeService, wire};

/// Transfers (protocol 6, F4): `transfers` exports and serves them, and
/// pulls imports from a source: this node, or another fleet node through
/// the envelope (`pass_on`).
impl NodeService {
    fn exporter(&self) -> Exporter<'_> {
        Exporter {
            sessions: &self.sessions,
            profiles: &self.profiles,
            transfers: &self.transfers,
        }
    }

    fn importer(&self) -> Importer<'_> {
        Importer {
            profiles: &self.profiles,
            transfers: &self.transfers,
            node: self.lifecycle.info().id,
        }
    }

    /// Where an import from `node` pulls, for `connection`'s caller: this
    /// node (an agent only copies a session it started, as for
    /// `session.export`), or another fleet node through the envelope, where
    /// the source applies its own ownership rules to the caller's origin.
    async fn source<'a>(
        &'a self,
        connection: &'a Connection,
        node: vessel_proto::NodeId,
        session: Option<SessionId>,
    ) -> Result<Pull<'a>, WireError> {
        if node != self.lifecycle.info().id {
            return Ok(Pull::Remote(Remote {
                service: self,
                principal: connection.principal(),
                node,
            }));
        }
        if let Some(session) = session {
            self.check_owner(connection.principal(), session).await?;
        }
        Ok(Pull::Local(Local {
            exporter: self.exporter(),
            holder: Holder::of(connection.principal()),
        }))
    }

    /// `session.export` (ownership checked by the guard).
    pub(crate) async fn export_session(
        &self,
        connection: &Connection,
        request: ExportSession,
    ) -> Result<TransferManifest, WireError> {
        self.exporter()
            .session(
                Holder::of(connection.principal()),
                request,
                Some(connection.protocol()),
            )
            .await
            .map_err(|error| wire(&error))
    }

    /// `memory.export`.
    pub(crate) async fn export_memory(
        &self,
        connection: &Connection,
        request: ExportMemory,
    ) -> Result<TransferManifest, WireError> {
        self.exporter()
            .memory(Holder::of(connection.principal()), request)
            .await
            .map_err(|error| wire(&error))
    }

    /// `transfer.read`: only for whoever opened it.
    pub(crate) async fn read_transfer(
        &self,
        connection: &Connection,
        request: ReadTransfer,
    ) -> Result<BlobChunk, WireError> {
        self.transfers
            .read(Holder::of(connection.principal()), request)
            .await
    }

    /// `transfer.close`: only for whoever opened it.
    pub(crate) async fn close_transfer(
        &self,
        connection: &Connection,
        request: CloseTransfer,
    ) -> Result<Ack, WireError> {
        self.transfers
            .close(Holder::of(connection.principal()), request.transfer)
            .await
            .map(|()| Ack)
            .map_err(|error| crate::transfers::wire(&error))
    }

    /// `session.import`: pulled and placed, then resumed if asked, as a
    /// session that records where it came from.
    pub(crate) async fn import_session(
        &self,
        connection: &Connection,
        request: ImportSession,
    ) -> Result<SessionImported, WireError> {
        // A local source can be checked before export creates its snapshot.
        if request.start && request.from.node == self.lifecycle.info().id {
            self.check_owner(connection.principal(), request.from.session)
                .await?;
            let info = self
                .sessions
                .get(request.from.session)
                .await
                .map_err(|error| wire(&error))?;
            crate::label_ingress::imported(&info)?;
        }
        if request.start && self.lifecycle.is_draining() {
            return Err(wire(&ServiceError::Draining));
        }
        let source = self
            .source(connection, request.from.node, Some(request.from.session))
            .await?;
        let placed = self.importer().session(&source, &request).await?;
        let mut imported = placed.imported;
        if request.start {
            let info = self
                .create(connection, placed.spec, Some(placed.origin))
                .await?;
            imported.session = Some(Box::new(info));
        }
        Ok(imported)
    }

    /// `memory.import`.
    pub(crate) async fn import_memory(
        &self,
        connection: &Connection,
        request: ImportMemory,
    ) -> Result<MemoryImported, WireError> {
        let source = self.source(connection, request.from.node, None).await?;
        self.importer().memory(&source, &request).await
    }
}
