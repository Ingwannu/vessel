//! The fleet's home side (ADR 0012) and this node's secrets (protocol 6,
//! F3), copying one to another node included. Routing only.

use vessel_proto::{
    Ack, BlobChunk, CopySecret, DeleteSecret, ErrorCode, FleetMember, Generation, GetManifest,
    ListMembers, ListSecrets, Manifest, ReadBlob, ReportApplied, SecretName, SetSecret, WatchFleet,
    WireError,
};

use crate::error::ServiceError;
use crate::principal::Principal;
use crate::secret_copy::Copied;

use super::{Connection, NodeService, wire};

/// The fleet (ADR 0012, protocol 5), served by the home's side of
/// `fleet`; a node that is not a home answers `unavailable`.
impl NodeService {
    #[expect(
        clippy::unused_async,
        reason = "every call has the same async shape, so the guard's rows are uniform"
    )]
    pub(crate) async fn get_manifest(
        &self,
        _: &Connection,
        _: GetManifest,
    ) -> Result<Manifest, WireError> {
        self.fleet.manifest()
    }

    pub(crate) async fn read_blob(
        &self,
        _: &Connection,
        request: ReadBlob,
    ) -> Result<BlobChunk, WireError> {
        self.fleet.blob(request.hash, request.offset).await
    }

    pub(crate) async fn watch_fleet(
        &self,
        _: &Connection,
        request: WatchFleet,
    ) -> Result<Generation, WireError> {
        self.fleet.watch(request.after).await
    }

    /// `fleet.report`: only a paired device (a member) reports; its device
    /// id is whose report it is.
    pub(crate) async fn report_applied(
        &self,
        connection: &Connection,
        request: ReportApplied,
    ) -> Result<Ack, WireError> {
        let Principal::Device { id, .. } = connection.principal() else {
            return Err(WireError::new(
                ErrorCode::InvalidRequest,
                "only a member node reports to its fleet's home".to_owned(),
            ));
        };
        self.fleet.report(*id, request).await.map(|()| Ack)
    }

    pub(crate) async fn list_members(
        &self,
        _: &Connection,
        _: ListMembers,
    ) -> Result<Vec<FleetMember>, WireError> {
        let devices = self.devices.list().await.map_err(|error| wire(&error))?;
        Ok(self.fleet.members(devices).await)
    }
}

/// This node's secrets (protocol 6, F3): the store `vessel fleet secret`
/// uses (`fleet::secrets`), on the wire. All admin. A value only ever goes
/// in; there is no call that reads one back.
impl NodeService {
    pub(crate) async fn set_secret(
        &self,
        _: &Connection,
        request: SetSecret,
    ) -> Result<Ack, WireError> {
        let files = self.fleet.files().clone();
        tokio::task::spawn_blocking(move || {
            crate::fleet::set_secret(&files, &request.name, request.value.expose())
        })
        .await
        .map_err(|error| wire(&ServiceError::Task(error)))?
        .map(|()| Ack)
        .map_err(|error| secrets_wire(&error))
    }

    pub(crate) async fn list_secrets(
        &self,
        _: &Connection,
        _: ListSecrets,
    ) -> Result<Vec<SecretName>, WireError> {
        let files = self.fleet.files().clone();
        tokio::task::spawn_blocking(move || crate::fleet::secret_names(&files))
            .await
            .map_err(|error| wire(&ServiceError::Task(error)))?
            .map_err(|error| secrets_wire(&error))
    }

    pub(crate) async fn delete_secret(
        &self,
        _: &Connection,
        request: DeleteSecret,
    ) -> Result<Ack, WireError> {
        let files = self.fleet.files().clone();
        let name = request.name.clone();
        let deleted =
            tokio::task::spawn_blocking(move || crate::fleet::delete_secret(&files, &name))
                .await
                .map_err(|error| wire(&ServiceError::Task(error)))?
                .map_err(|error| secrets_wire(&error))?;
        if deleted {
            Ok(Ack)
        } else {
            Err(WireError::new(
                ErrorCode::NotFound,
                format!("secret {} is not set on this node", request.name),
            ))
        }
    }
}

/// `secret.copy` (protocol 6, F3): this node's value of a secret, sent to
/// another fleet node's store as `secret.set`.
///
/// [Decision Log]
/// - Purpose: one secret set once, on several nodes, without typing it on
///   each; a value still leaves a node only into another node's store
///   (there is no `secret.get`).
/// - The value is read here and sent as `secret.set` through the envelope
///   (`pass_on`) as the caller, so the target runs it as the home's device
///   cut down to the caller's origin: copying needs the target to have
///   granted the home `admin`, which the default grant (`read`, `operate`)
///   leaves out on purpose. The target is asked a value-free `secret.list`
///   first, and the value goes only if that passes (`secret_copy`), so a
///   target that would refuse it never receives it. A refusal for that says so and how to grant it
///   (join again with `--grant`); the home itself grants its members only
///   `sync` and `relay`, so a secret is set there directly instead.
/// - The value is never logged and never in an error: `SecretValue`
///   redacts its `Debug`, `vessel-rpc` logs method names only, and the
///   errors here name the secret, never its value.
impl NodeService {
    pub(crate) async fn copy_secret(
        &self,
        connection: &Connection,
        request: CopySecret,
    ) -> Result<Ack, WireError> {
        let CopySecret { name, to } = request;
        if to == self.lifecycle.info().id {
            return Err(WireError::new(
                ErrorCode::InvalidRequest,
                format!("secret {name} is on this node already; name another fleet node"),
            ));
        }
        let value = {
            let files = self.fleet.files().clone();
            let name = name.clone();
            tokio::task::spawn_blocking(move || crate::fleet::secret_value(&files, &name))
                .await
                .map_err(|error| wire(&ServiceError::Task(error)))?
                .map_err(|error| secrets_wire(&error))?
        };
        let Some(value) = value else {
            return Err(WireError::new(
                ErrorCode::NotFound,
                format!("secret {name} is not set on this node"),
            ));
        };
        let set = SetSecret {
            name: name.clone(),
            value,
        };
        // Asked first, the value sent only if that passes (`secret_copy`).
        let principal = connection.principal();
        let copied =
            crate::secret_copy::copy(set, |call| self.pass_on(principal, to, Box::new(call)))
                .await?;
        match copied {
            Copied::Stored => {
                tracing::info!(secret = %name, %to, "secret copied");
                Ok(Ack)
            }
            Copied::NotGranted => Err(self.not_granted(&name, to).await),
        }
    }

    /// Why `to` refused to store a secret for want of `admin`, and what to
    /// do about it.
    async fn not_granted(&self, name: &SecretName, to: vessel_proto::NodeId) -> WireError {
        let message = if self.links.home().await == Some(to) {
            format!(
                "the fleet's home grants its members only `sync` and `relay`, so secret {name} \
                 cannot be copied to it; set it there with `vessel fleet secret set {name}`"
            )
        } else {
            format!(
                "node {to} has not granted the fleet's home `admin`, which copying secret \
                 {name} there needs (a member grants `read` and `operate` unless asked). On \
                 that node, join again granting it: `vessel fleet leave`, then `vessel fleet \
                 join --grant read,operate,admin` with a new link from `vessel fleet invite` \
                 on the home"
            )
        };
        WireError::new(ErrorCode::Forbidden, message)
    }
}

/// A `secrets.toml` problem on the wire; the message names no value.
fn secrets_wire(error: &crate::fleet::SecretsError) -> WireError {
    let code = match error {
        crate::fleet::SecretsError::File(_) => ErrorCode::Internal,
        crate::fleet::SecretsError::Parse { .. } | crate::fleet::SecretsError::Entry { .. } => {
            ErrorCode::Conflict
        }
    };
    WireError::new(code, error.to_string())
}
