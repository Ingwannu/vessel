//! Owned test CLI processes are signalled only after checking their executable.

/// A failed assertion must not leave the owned CLI alive.
pub(crate) struct ChildGuard(pub(crate) tokio::process::Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _already_ended = self.0.start_kill();
    }
}

/// Check ownership of the still-unreaped child before signalling its PID.
#[cfg(target_os = "linux")]
pub(crate) async fn interrupt(child: &mut tokio::process::Child) {
    assert!(child.try_wait().unwrap().is_none());
    let pid = child.id().unwrap();
    let executable = tokio::fs::read_link(format!("/proc/{pid}/exe"))
        .await
        .unwrap();
    let expected = tokio::fs::canonicalize(env!("CARGO_BIN_EXE_vessel"))
        .await
        .unwrap();
    assert_eq!(executable, expected);
    let status = tokio::process::Command::new(which::which("kill").unwrap())
        .arg("-INT")
        .arg(pid.to_string())
        .status()
        .await
        .unwrap();
    assert!(status.success());
}
