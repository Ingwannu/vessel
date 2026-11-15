//! CLI owner lifetime: cancellation settles before any partial report is printed.
mod source;
use crate::{args::Compare, error::CliError, io};
use source::SourceTrail;
use std::{path::Path, sync::Arc};
use tokio::io::AsyncReadExt as _;
use vessel_client::{
    NodeClient,
    comparison::{
        ComparisonInput, ComparisonPlan, Selection, Snapshot, SourceAttachments, TargetStatus,
    },
};
use vessel_proto::workspace::WorktreeBase;

#[derive(Debug, thiserror::Error)]
pub(crate) enum CompareError {
    #[error("cannot read comparison selections")]
    Manifest(#[source] std::io::Error),
    #[error("comparison selections must be a JSON array of bounded structured selections")]
    Selections(#[source] serde_json::Error),
    #[error("comparison selection manifest exceeds 128 KiB")]
    ManifestSize,
    #[error("cannot encode comparison report")]
    Report(#[source] serde_json::Error),
    #[error("comparison did not start every target; inspect the resource report")]
    Partial,
    #[error("comparison stopped; inspect the resource report")]
    Cancelled,
    #[error("source upload exceeded its operation deadline; inspect its phase and known upload id")]
    SourceDeadline,
    #[error("source upload exceeded its total or acknowledged-byte progress budget")]
    SourceBudget,
    #[error("source upload acknowledged an unexpected offset or file metadata")]
    SourceMismatch,
}
pub(super) async fn run(client: &NodeClient, args: Compare) -> Result<(), CliError> {
    if args.prompt.is_none() && args.attach.is_empty() {
        return Err(CliError::Usage("write a prompt or attach a file"));
    }
    if args.attach.len() > 16 {
        return Err(CliError::Usage("attach at most sixteen files"));
    }
    let selections = manifest(&args.targets).await?;
    let plan = ComparisonPlan::new(
        args.project,
        WorktreeBase::Ref {
            name: args.base.clone(),
        },
        selections,
    )?;
    let destination = args.destination.unwrap_or(client.node().id);
    let job = client.comparison(destination, plan);
    let mut source = SourceTrail::default();
    let result = tokio::select! {
        result = perform(client, destination, &job, &args, &mut source) => result,
        signal = tokio::signal::ctrl_c() => {
            match signal {
                Ok(()) => Err(CompareError::Cancelled.into()),
                Err(source) => Err(CompareError::Manifest(source).into()),
            }
        }
    };
    source.interrupted();
    let snapshot = job.cancel().await?;
    show(&snapshot, &source, args.json).await?;
    result?;
    if snapshot
        .targets
        .iter()
        .all(|target| matches!(target.status, TargetStatus::Started))
    {
        Ok(())
    } else {
        Err(CompareError::Partial.into())
    }
}
async fn manifest(path: &Path) -> Result<Vec<Selection>, CliError> {
    const LIMIT: u64 = 128 * 1024;
    let file = tokio::fs::File::open(path)
        .await
        .map_err(CompareError::Manifest)?;
    let mut bytes = Vec::new();
    let _read = file
        .take(LIMIT.saturating_add(1))
        .read_to_end(&mut bytes)
        .await
        .map_err(CompareError::Manifest)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > LIMIT {
        return Err(CompareError::ManifestSize.into());
    }
    Ok(serde_json::from_slice(&bytes).map_err(CompareError::Selections)?)
}
async fn perform(
    client: &NodeClient,
    destination: vessel_proto::NodeId,
    job: &vessel_client::comparison::ComparisonJob,
    args: &Compare,
    source: &mut SourceTrail,
) -> Result<(), CliError> {
    let ready = job.prepare().await?;
    let files = if args.attach.is_empty() {
        None
    } else {
        let session = ready
            .targets
            .iter()
            .find(|target| matches!(target.status, TargetStatus::Ready))
            .and_then(|target| target.resources.session)
            .ok_or(CompareError::Partial)?;
        let files = source::upload(client, destination, session, &args.attach, source).await?;
        Some(SourceAttachments::new(session, files)?)
    };
    let _sent = job
        .start(ComparisonInput::new(args.prompt.clone(), files)?)
        .await?;
    Ok(())
}
async fn show(snapshot: &Arc<Snapshot>, source: &SourceTrail, json: bool) -> Result<(), CliError> {
    #[derive(serde::Serialize)]
    struct Report<'a> {
        comparison: &'a Snapshot,
        source_upload: &'a SourceTrail,
    }
    if json {
        let mut bytes = serde_json::to_vec(&Report {
            comparison: snapshot,
            source_upload: source,
        })
        .map_err(CompareError::Report)?;
        bytes.push(b'\n');
        io::out(&bytes).await?;
    } else {
        for target in &snapshot.targets {
            // The complete typed status includes the reason and resources, so a
            // failure cannot hide a successful target or turn uncertainty into retry.
            let line = serde_json::to_string(target).map_err(CompareError::Report)?;
            io::out(format!("{line}\n").as_bytes()).await?;
        }
        if source.session.is_some() {
            let line = serde_json::to_string(source).map_err(CompareError::Report)?;
            io::out(format!("source_upload {line}\n").as_bytes()).await?;
        }
    }
    Ok(())
}
