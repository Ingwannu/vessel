//! The rule's cases: the same hostname and OS, and what keeps a machine out.

use super::{Host, is_this_computer};

const fn host<'a>(hostname: &'a str, os: &'a str) -> Host<'a> {
    Host { hostname, os }
}

#[test]
fn the_same_hostname_and_os_is_this_computer() {
    let own = host("studio", "macos");
    assert!(is_this_computer(host("studio", "macos"), own));
    assert!(is_this_computer(host("Studio", "macos"), own));
    assert!(is_this_computer(host(" studio\n", "macos"), own));
}

#[test]
fn another_hostname_is_not_even_on_loopback() {
    // A tunnelled or containerised node reports its own hostname.
    let own = host("studio", "linux");
    assert!(!is_this_computer(host("build-server", "linux"), own));
    assert!(!is_this_computer(host("3f2a9c1d7e0b", "linux"), own));
}

#[test]
fn another_os_is_not_this_computer() {
    // A WSL node on Windows keeps the computer's name but runs Linux.
    let own = host("DESKTOP-1", "windows");
    assert!(!is_this_computer(host("DESKTOP-1", "linux"), own));
}

#[test]
fn an_empty_hostname_never_matches() {
    assert!(!is_this_computer(host("", "linux"), host("", "linux")));
    assert!(!is_this_computer(host("  ", "linux"), host("  ", "linux")));
}
