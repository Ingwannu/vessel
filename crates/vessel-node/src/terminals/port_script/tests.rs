//! The staged helper's real preparation boundary; no product caller is enabled.

use std::error::Error as _;
use std::path::PathBuf;

use vessel_proc::EnvValue;
use vessel_proto::workspace::BranchName;
use vessel_proto::{ProjectId, WorktreeId};

use super::*;

struct Sandbox {
    temp: tempfile::TempDir,
    root: ScriptRoot,
}

impl Sandbox {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let project_root = temp.path().join("project");
        let root = temp.path().join("checkout");
        std::fs::create_dir(&project_root).unwrap();
        std::fs::create_dir(&root).unwrap();
        Self {
            temp,
            root: ScriptRoot {
                project: ProjectId::random(),
                project_root,
                root,
                worktree: Some(WorktreeId::random()),
                branch: Some("topic/$(untrusted)&branch".parse::<BranchName>().unwrap()),
            },
        }
    }

    fn file(&self, name: &str) -> PathBuf {
        let file = self.root.root.join(name);
        std::fs::write(&file, "not executed by a preparation test").unwrap();
        file
    }

    #[cfg(unix)]
    fn executable(path: &std::path::Path, text: &str) {
        vessel_testkit::write_executable(path, text).unwrap();
    }
}

#[tokio::test]
async fn literal_arguments_match_the_environment_and_credentials_are_removed() {
    let sandbox = Sandbox::new();
    let expected = std::fs::canonicalize(sandbox.file("port-tool")).unwrap();
    let name: ScriptName = "web-app".parse().unwrap();
    let config = crate::TerminalsConfig::default();
    let spec = prepare(
        &sandbox.root,
        &name,
        &"port-tool".parse().unwrap(),
        &config.removed,
    )
    .await
    .unwrap();
    let folder = std::fs::canonicalize(&sandbox.root.root).unwrap();
    assert_eq!(spec.program, expected);
    assert_eq!(spec.cwd, Some(folder.clone()));
    let values = [
        ("VESSEL_SCRIPT_NAME", OsString::from(name.as_str())),
        (
            "VESSEL_PLACE_ID",
            sandbox.root.worktree.unwrap().to_string().into(),
        ),
        (
            "VESSEL_BRANCH",
            sandbox.root.branch.as_ref().unwrap().as_str().into(),
        ),
        ("VESSEL_WORKTREE", folder.into_os_string()),
    ];
    assert_eq!(
        spec.args,
        values
            .iter()
            .map(|(_, value)| value.clone())
            .collect::<Vec<_>>()
    );
    for (name, value) in values {
        assert_eq!(spec.env_change(name), Some(&EnvValue::Set(value)));
    }
    for name in config.removed {
        assert_eq!(
            spec.env_change(name.to_str().unwrap()),
            Some(&EnvValue::Remove)
        );
    }
    for name in ["TERM", "COLORTERM", "PATH"] {
        assert_eq!(spec.env_change(name), None);
    }
    let mut project = sandbox.root.clone();
    project.worktree = None;
    project.branch = None;
    let spec = prepare(&project, &name, &"port-tool".parse().unwrap(), &[])
        .await
        .unwrap();
    assert_eq!(
        spec.args.get(1),
        Some(&OsString::from(project.project.to_string()))
    );
    assert_eq!(spec.args.get(2), Some(&OsString::new()));
}

#[tokio::test]
async fn directories_missing_files_and_foreign_grammars_are_typed_refusals() {
    let sandbox = Sandbox::new();
    let name = "web".parse().unwrap();
    std::fs::create_dir(sandbox.root.root.join("folder")).unwrap();
    assert!(matches!(
        prepare(&sandbox.root, &name, &"folder".parse().unwrap(), &[]).await,
        Err(PortScriptError::NotFile)
    ));
    let missing = prepare(&sandbox.root, &name, &"missing".parse().unwrap(), &[])
        .await
        .unwrap_err();
    assert!(matches!(
        missing,
        PortScriptError::Path(PathError::NotFound(_))
    ));
    assert!(missing.source().unwrap().is::<PathError>());
    let foreign = if cfg!(windows) {
        "/foreign/port"
    } else {
        r"C:\foreign\port.exe"
    };
    assert!(matches!(
        prepare(&sandbox.root, &name, &foreign.parse().unwrap(), &[]).await,
        Err(PortScriptError::Platform)
    ));
}

#[test]
fn windows_command_names_are_checked_before_normalization_or_interpreter_insertion() {
    for text in [
        r"C:\tools\port.CmD",
        r"C:\tools\port.bAt. ",
        r"\\?\C:\tools\port.CMD",
        r"\\server\share\port.bat",
        "tools/port.cmd ",
    ] {
        assert!(
            matches!(
                program::windows_name(text),
                Err(PortScriptError::WindowsCommandScript)
            ),
            "{text}"
        );
    }
    for text in [
        r"C:\tools.\port.exe",
        r"C:\tools\port.exe:stream",
        r"\\?\C:\tools\port.exe ",
        r"C:\tools\C:\port.exe",
    ] {
        assert!(
            matches!(
                program::windows_name(text),
                Err(PortScriptError::WindowsPath)
            ),
            "{text}"
        );
    }
    for text in [
        r"C:\tools.cmd\port.exe",
        r"\\?\C:\tools\port.exe",
        r"\\.\device\port.exe",
        r"\\server\share\port.exe",
        "tools/port.exe",
    ] {
        program::windows_name(text).unwrap();
    }
}

#[cfg(unix)]
#[tokio::test]
async fn relative_symlinks_cannot_escape_the_checkout() {
    let sandbox = Sandbox::new();
    let outside = sandbox.temp.path().join("outside");
    std::fs::write(&outside, "not executed").unwrap();
    std::os::unix::fs::symlink(&outside, sandbox.root.root.join("port-tool")).unwrap();
    let error = prepare(
        &sandbox.root,
        &"web".parse().unwrap(),
        &"port-tool".parse().unwrap(),
        &[],
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        PortScriptError::Path(PathError::Outside(_))
    ));
}

#[cfg(windows)]
#[tokio::test]
async fn windows_command_scripts_are_refused_before_spawn() {
    let sandbox = Sandbox::new();
    for text in ["port.CmD", "port.bAt. "] {
        let error = prepare(
            &sandbox.root,
            &"web".parse().unwrap(),
            &text.parse().unwrap(),
            &[],
        )
        .await
        .unwrap_err();
        assert!(matches!(error, PortScriptError::WindowsCommandScript));
    }
    // This fixture is only decoded/refused, never written as an executable.
    let batch = sandbox.root.root.join("port.CmD");
    std::fs::write(&batch, "@echo off\r\necho unsafe>started\r\n").unwrap();
    let error = prepare(
        &sandbox.root,
        &"web".parse().unwrap(),
        &batch.to_str().unwrap().parse().unwrap(),
        &[],
    )
    .await
    .unwrap_err();
    assert!(matches!(error, PortScriptError::WindowsCommandScript));
    assert!(!sandbox.root.root.join("started").exists());
}

#[test]
fn only_utf8_one_port_output_is_decoded_with_javascript_trim() {
    assert_eq!(
        capture::decode("\u{feff}\u{2028}003210\u{2029}\n".as_bytes())
            .unwrap()
            .get(),
        3210
    );
    for output in [
        "",
        "3210 3211",
        "3210\n3211",
        "\u{0085}3210",
        "+3210",
        "3.5",
        "secret text",
    ] {
        let error = capture::decode(output.as_bytes()).unwrap_err();
        assert!(matches!(error, PortScriptError::Port(_)));
        assert!(!format!("{error:?} {error}").contains(output) || output.is_empty());
    }
    let below = ServicePort::MIN.get() - ServicePort::MIN.get();
    let above = u32::from(ServicePort::MAX.get()) + u32::from(ServicePort::MIN.get());
    for output in [below.to_string(), above.to_string()] {
        assert!(matches!(
            capture::decode(output.as_bytes()),
            Err(PortScriptError::Port(_))
        ));
    }
    let error = capture::decode(&[0xff]).unwrap_err();
    assert!(matches!(error, PortScriptError::Utf8(_)));
    assert!(error.source().unwrap().is::<std::str::Utf8Error>());
}

#[tokio::test]
async fn each_output_pipe_has_an_independent_exact_byte_cap_and_read_errors_keep_their_source() {
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use tokio::io::{AsyncRead, ReadBuf};
    use vessel_proc::Stream;

    struct Failed;
    impl AsyncRead for Failed {
        fn poll_read(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
            _: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            Poll::Ready(Err(io::Error::from(io::ErrorKind::BrokenPipe)))
        }
    }
    for stream in [Stream::Stdout, Stream::Stderr] {
        let exact = vec![b'x'; usize::from(MAX_OUTPUT_BYTES)];
        assert_eq!(
            capture::read(exact.as_slice(), stream).await.unwrap(),
            exact
        );
        let overflow = vec![b'x'; usize::from(MAX_OUTPUT_BYTES) + 1];
        assert!(matches!(capture::read(overflow.as_slice(), stream).await,
            Err(PortScriptError::OutputLimit(found)) if found == stream));
        let error = capture::read(Failed, stream).await.unwrap_err();
        assert!(matches!(&error, PortScriptError::Read { stream: found, .. } if *found == stream));
        assert_eq!(
            error
                .source()
                .unwrap()
                .downcast_ref::<io::Error>()
                .unwrap()
                .kind(),
            io::ErrorKind::BrokenPipe
        );
    }
}

#[cfg(unix)]
#[tokio::test]
async fn bare_port_script_is_resolved_under_the_approved_root() {
    let sandbox = Sandbox::new();
    let decoy = sandbox.temp.path().join("decoy");
    std::fs::create_dir(&decoy).unwrap();
    Sandbox::executable(&decoy.join("port-tool"), "#!/bin/sh\nprintf '3212\\n'\n");
    Sandbox::executable(
        &sandbox.root.project_root.join("port-tool"),
        "#!/bin/sh\nprintf '3210\\n'\n",
    );
    Sandbox::executable(
        &sandbox.root.root.join("port-tool"),
        "#!/bin/sh\nprintf '3211\\n'\n",
    );
    let name = "web".parse().unwrap();
    let program = "port-tool".parse().unwrap();
    let spec = prepare(&sandbox.root, &name, &program, &[])
        .await
        .unwrap()
        .env_clear()
        .env("HOME", sandbox.temp.path())
        .env("PATH", &decoy);
    assert_eq!(
        capture::port(vessel_proc::spawn(&spec).unwrap())
            .await
            .unwrap()
            .get(),
        3211
    );
    let mut project = sandbox.root.clone();
    project.root = project.project_root.clone();
    project.worktree = None;
    let spec = prepare(&project, &name, &program, &[])
        .await
        .unwrap()
        .env_clear()
        .env("HOME", sandbox.temp.path())
        .env("PATH", &decoy);
    assert_eq!(
        capture::port(vessel_proc::spawn(&spec).unwrap())
            .await
            .unwrap()
            .get(),
        3210
    );
}

#[cfg(unix)]
#[tokio::test]
async fn direct_invocation_preserves_metacharacters_and_matching_argument_environment_pairs() {
    let sandbox = Sandbox::new();
    Sandbox::executable(
        &sandbox.root.root.join("port-tool"),
        r#"#!/bin/sh
IFS= read -r unexpected && exit 6
[ "$#" = 4 ] || exit 7
[ "$1" = "$VESSEL_SCRIPT_NAME" ] || exit 8
[ "$2" = "$VESSEL_PLACE_ID" ] || exit 9
[ "$3" = "$VESSEL_BRANCH" ] || exit 11
[ "$4" = "$VESSEL_WORKTREE" ] || exit 12
[ "$PWD" = "$4" ] || exit 13
[ "$3" = 'topic/$(untrusted)&branch' ] || exit 14
printf '\357\273\2773210\n'
"#,
    );
    let port = invoke(
        &sandbox.root,
        &"web".parse().unwrap(),
        &"port-tool".parse().unwrap(),
        &crate::TerminalsConfig::default().removed,
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(port.get(), 3210);
    assert!(!sandbox.root.root.join("untrusted").exists());
}

#[cfg(unix)]
#[tokio::test]
async fn unsuccessful_exit_and_overflow_are_refused_without_output_in_diagnostics() {
    enum Expected {
        Exit,
        Stdout,
        Stderr,
        Exact,
    }
    let sandbox = Sandbox::new();
    for (name, script, expected) in [
        (
            "failed",
            "#!/bin/sh\nprintf '3210\\n'\nprintf 'private-stderr' >&2\nexit 7\n".to_owned(),
            Expected::Exit,
        ),
        (
            "stdout",
            format!(
                "#!/bin/sh\nprintf '%s' '{}'\n",
                "x".repeat(usize::from(MAX_OUTPUT_BYTES) + 1)
            ),
            Expected::Stdout,
        ),
        (
            "stderr",
            format!(
                "#!/bin/sh\nprintf '3210\\n'\nprintf '%s' '{}' >&2\n",
                "x".repeat(usize::from(MAX_OUTPUT_BYTES) + 1)
            ),
            Expected::Stderr,
        ),
        (
            "exact",
            format!(
                "#!/bin/sh\nprintf '%s3210' '{}'\nprintf '%s' '{}' >&2\n",
                " ".repeat(usize::from(MAX_OUTPUT_BYTES) - "3210".len()),
                "x".repeat(usize::from(MAX_OUTPUT_BYTES))
            ),
            Expected::Exact,
        ),
    ] {
        Sandbox::executable(&sandbox.root.root.join(name), &script);
        let result = invoke(
            &sandbox.root,
            &"web".parse().unwrap(),
            &name.parse().unwrap(),
            &[],
            &CancellationToken::new(),
        )
        .await;
        if let Err(error) = &result {
            assert!(!format!("{error:?} {error}").contains("private-stderr"));
        }
        match expected {
            Expected::Exit => assert!(matches!(result, Err(PortScriptError::Exit(Some(7))))),
            Expected::Stdout => assert!(matches!(
                result,
                Err(PortScriptError::OutputLimit(vessel_proc::Stream::Stdout))
            )),
            Expected::Stderr => assert!(matches!(
                result,
                Err(PortScriptError::OutputLimit(vessel_proc::Stream::Stderr))
            )),
            Expected::Exact => assert_eq!(result.unwrap().get(), 3210),
        }
    }
}

#[tokio::test]
async fn cancellation_before_resolution_never_starts_a_child() {
    let sandbox = Sandbox::new();
    let cancel = CancellationToken::new();
    cancel.cancel();
    let result = invoke(
        &sandbox.root,
        &"web".parse().unwrap(),
        &"absent".parse().unwrap(),
        &[],
        &cancel,
    )
    .await;
    assert!(matches!(result, Err(PortScriptError::Cancelled)));
}

#[cfg(unix)]
#[tokio::test]
async fn a_helper_that_never_exits_is_bounded_by_the_named_deadline() {
    let sandbox = Sandbox::new();
    Sandbox::executable(
        &sandbox.root.root.join("port-tool"),
        &format!("#!/bin/sh\nexec /bin/sleep {}\n", TIMEOUT.as_secs() + 1),
    );
    let result = invoke(
        &sandbox.root,
        &"web".parse().unwrap(),
        &"port-tool".parse().unwrap(),
        &[],
        &CancellationToken::new(),
    )
    .await;
    assert!(matches!(result, Err(PortScriptError::Timeout)));
}

#[tokio::test]
async fn cancelling_or_timing_out_live_capture_kills_the_child_and_its_inherited_pipe_holder() {
    use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, BufReader};
    use vessel_proc::Spawned;
    use vessel_testkit::Fake;

    const TEST_WAIT: Duration = Duration::from_secs(60);
    #[derive(Clone, Copy)]
    enum Stop {
        Cancel,
        Deadline,
    }

    vessel_testkit::within(TEST_WAIT, async {
        let sandbox = Sandbox::new();
        for stop in [Stop::Cancel, Stop::Deadline] {
            let mut spec = CommandSpec::new(Fake::Tui.path().unwrap())
                .env_clear()
                .env("HOME", sandbox.temp.path())
                .env("USERPROFILE", sandbox.temp.path());
            if cfg!(windows) {
                spec = spec.env("SystemRoot", std::env::var_os("SystemRoot").unwrap());
            }
            let Spawned {
                child,
                stdin,
                stdout,
                stderr,
            } = vessel_proc::spawn(&spec).unwrap();
            let mut observer = BufReader::new(stdout);
            let mut line = String::new();
            let mut grandchild = false;
            loop {
                line.clear();
                assert_ne!(observer.read_line(&mut line).await.unwrap(), 0);
                grandchild |= line.starts_with("grandchild ");
                if line.trim_end() == "ready" {
                    break;
                }
            }
            assert!(
                grandchild,
                "the fake announced a descendant holding the same pipe"
            );
            // Retain the real pipe as the group-death observer. A pending
            // decoder input prevents EOF or a port parse from ending the work.
            let (input, _writer) = tokio::io::duplex(1);
            let cancel = CancellationToken::new();
            let mut work = Box::pin(bounded(capture::collect(child, input, stderr), &cancel));
            assert!(futures_util::poll!(work.as_mut()).is_pending());
            match stop {
                Stop::Cancel => cancel.cancel(),
                Stop::Deadline => {
                    tokio::time::pause();
                    tokio::time::advance(TIMEOUT).await;
                    tokio::time::resume();
                }
            }
            let error = work.await.unwrap_err();
            match stop {
                Stop::Cancel => assert!(matches!(error, PortScriptError::Cancelled)),
                Stop::Deadline => assert!(matches!(error, PortScriptError::Timeout)),
            }
            // stdin stays open: neither fake could have exited through EOF.
            // EOF here proves both the leader and descendant closed stdout.
            let mut rest = Vec::new();
            let _remaining = observer.read_to_end(&mut rest).await.unwrap();
            drop(stdin);
        }
    })
    .await
    .unwrap();
}
