//! What a status's `origin` says its host is (ADR 0123 step B): only the
//! node knows a self-hosted GitHub, and a read never waits for `gh`.

use std::ffi::OsString;
use std::time::Duration;

use vessel_forge::EnvTokens;

use super::*;

fn host(name: &str) -> ForgeHost {
    name.parse().unwrap()
}

/// A node whose environment names a GitHub Enterprise host and its token,
/// with no `gh` on the machine.
fn enterprise_node(data: &std::path::Path) -> Hosts {
    let vars = |name: &str| match name {
        "GH_HOST" => Some(OsString::from("ghe.example.org")),
        "GH_ENTERPRISE_TOKEN" => Some(OsString::from("enterprise")),
        _ => None,
    };
    Hosts::new(
        ForgeSetup {
            env: EnvTokens::from_vars(vars),
            ..ForgeSetup::default()
        },
        data.to_path_buf(),
    )
}

fn asking(hosts: &Hosts) -> usize {
    hosts.asking.lock().unwrap().len()
}

#[tokio::test]
async fn github_s_hosts_and_a_host_with_a_token_are_github() {
    let data = tempfile::tempdir().unwrap();
    let hosts = enterprise_node(data.path());
    assert_eq!(hosts.kind_of(&host("github.com")), Some(ForgeKind::Github));
    assert_eq!(
        hosts.kind_of(&host("ssh.github.com")),
        Some(ForgeKind::Github)
    );
    assert_eq!(
        hosts.kind_of(&host("ghe.example.org")),
        Some(ForgeKind::Github)
    );
    // Another forge's public host is never asked about.
    for other in OTHER_FORGE_HOSTS {
        assert_eq!(hosts.kind_of(&host(other)), None);
    }
    assert_eq!(asking(&hosts), 0);
}

#[tokio::test]
async fn an_unknown_host_is_asked_once_off_the_read_and_its_no_kept() {
    let data = tempfile::tempdir().unwrap();
    let hosts = enterprise_node(data.path());
    let other = host("git.example.org");
    assert_eq!(hosts.kind_of(&other), None);
    // Asked in a task of its own: wait for its answer (no `gh`: a miss).
    for _ in 0..200 {
        if hosts.missed_lately(&other) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(hosts.missed_lately(&other));
    assert_eq!(asking(&hosts), 0);
    // A second read asks nothing: the miss is kept (the link's for a
    // minute, after the credentials' own ten seconds).
    assert_eq!(hosts.kind_of(&other), None);
    assert_eq!(asking(&hosts), 0);
}

#[tokio::test]
async fn at_most_four_hosts_are_asked_at_once() {
    let data = tempfile::tempdir().unwrap();
    let hosts = enterprise_node(data.path());
    {
        let mut asking = hosts.asking.lock().unwrap();
        for n in 0..ASKING {
            let _new = asking.insert(host(&format!("h{n}.example.org")));
        }
    }
    assert_eq!(hosts.kind_of(&host("later.example.org")), None);
    assert_eq!(asking(&hosts), ASKING);
    assert!(!hosts.missed_lately(&host("later.example.org")));
}

#[test]
fn the_misses_kept_are_bounded() {
    let misses = Mutex::new(BTreeMap::new());
    for n in 0..(MAX_LINK_MISSES * 2) {
        note_miss(&misses, &host(&format!("m{n}.example.org")));
    }
    assert_eq!(misses.lock().unwrap().len(), MAX_LINK_MISSES);
}
