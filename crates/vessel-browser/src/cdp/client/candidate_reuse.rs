//! Reopen the same private fixture after the first actual Chrome has stopped.

use std::fs::{self, File};
use std::io::{self, Read as _};
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};

use vessel_proto::browser::{RequiredInput, ViewportSide, ViewportSize, WaitBudget, WaitFor, WaitMatch};

use super::actor::Shutdown;
use super::candidate_page::{PERSISTED_MARKER, PERSISTED_ROUTE};
use super::owner::Owner;
use crate::launch::{Plan, ProxyEndpoint};
use crate::page::Page;
use crate::preparation::Preparation;
use crate::profile::Profile;

const MAX_PROCESSES: usize = 100_000;
const MAX_ARG_BYTES: u64 = 16 * 1024;

pub(super) async fn relaunch(
    scope: &Preparation,
    root: PathBuf,
    temporary: PathBuf,
    proxy: ProxyEndpoint,
    first: Shutdown,
) {
    let receipt = first.receipt.as_ref().unwrap();
    assert_eq!(receipt.code(), Some(0));
    assert!(!receipt.run().is_live());
    let group = receipt.run().pid().get();
    let fixture = root.parent().unwrap().to_owned();
    // Release only the isolated fixture after the original run has a close
    // receipt and a fresh read-only absence observation.
    scope.run(move || observe_quiet(&fixture, group)).await.unwrap().unwrap();
    drop(first);
    let plan = scope.run(move || Plan::inspect(Profile::prepare(&root, &temporary).unwrap(), proxy))
        .await.unwrap().unwrap();
    let owner = Owner::spawn(plan).unwrap();
    let run = owner.lifetime();
    let (mut owner, readiness) = owner.attest(scope).await.unwrap();
    let result = if let Some((client, _events)) = owner.controller() {
        Some(read_marker(client).await)
    } else {
        None
    };
    let closed = owner.close().await.unwrap();
    readiness.unwrap();
    assert!(closed.run().same_run(&run));
    assert_eq!(closed.code(), Some(0));
    assert!(!run.is_live());
    result.unwrap().unwrap();
}

async fn read_marker(client: &super::Client) -> Result<(), crate::page::PageError> {
    let side = ViewportSide::try_from(390).unwrap();
    let viewport = ViewportSize { width: side, height: side };
    let mut page = Page::open(client.clone(), PERSISTED_ROUTE.parse().unwrap(), viewport).await?;
    let expected = RequiredInput::try_from(PERSISTED_MARKER.to_owned()).unwrap();
    let receipt = page.wait(WaitFor::Text(expected), WaitBudget::Default).await?;
    assert_eq!(receipt, WaitMatch::Text);
    page.close().await
}

fn observe_quiet(fixture: &Path, group: u32) -> io::Result<()> {
    let marker = fixture.as_os_str().as_encoded_bytes();
    let uid = nix::unistd::geteuid().as_raw();
    for (index, entry) in fs::read_dir("/proc")?.enumerate() {
        assert!(index < MAX_PROCESSES);
        let entry = entry?;
        let path = entry.path();
        if entry.file_name().to_string_lossy().parse::<u32>().is_err() {
            continue;
        }
        match observe_process(&path, marker, group, uid) {
            Ok(()) => {},
            Err(error) if error.kind() == io::ErrorKind::NotFound => {},
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn observe_process(path: &Path, marker: &[u8], group: u32, uid: u32) -> io::Result<()> {
    let record = fs::read_to_string(path.join("stat"))?;
    let (_, fields) = record.rsplit_once(')').ok_or(io::ErrorKind::InvalidData)?;
    let mut fields = fields.split_whitespace();
    let state = fields.next().ok_or(io::ErrorKind::InvalidData)?;
    let owned_group = fields.nth(1).and_then(|field| field.parse().ok()) == Some(group);
    if matches!(state, "Z" | "X") {
        return Ok(());
    }
    assert!(!owned_group, "the first browser still has a live group member");
    if path.metadata()?.uid() != uid || !fs::read_to_string(path.join("comm"))?.starts_with("chrome") {
        return Ok(());
    }
    let mut arguments = Vec::new();
    let read = File::open(path.join("cmdline"))?.take(MAX_ARG_BYTES.saturating_add(1)).read_to_end(&mut arguments)?;
    if !u64::try_from(read).is_ok_and(|length| length <= MAX_ARG_BYTES) {
        return Err(io::ErrorKind::InvalidData.into());
    }
    assert!(!arguments.windows(marker.len()).any(|window| window == marker), "the first browser still has a live profile process");
    Ok(())
}
