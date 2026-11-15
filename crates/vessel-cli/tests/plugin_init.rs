//! Actual local CLI output/effects, using no node or application configuration.
#[cfg(test)]
mod tests {
    use std::{path::Path, time::Duration};
    use tokio::process::Command;
    use vessel_proto::plugins::ClientDocument;

    const CLI_START_BUDGET: Duration = Duration::from_secs(30);

    async fn init(root: &Path, args: &[&str]) -> std::process::Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_vessel"));
        command
            .env_clear()
            .env("HOME", root)
            .env("TMPDIR", root)
            .env("TEMP", root)
            .env("TMP", root)
            .current_dir(root)
            .kill_on_drop(true)
            .args(["--node", "ws://[", "plugin", "init"])
            .args(args);
        #[cfg(windows)]
        if let Some(system_root) = std::env::var_os("SystemRoot") {
            command.env("SystemRoot", system_root);
        }
        tokio::time::timeout(CLI_START_BUDGET, command.output())
            .await
            .expect("bounded local CLI")
            .expect("actual CLI output")
    }

    #[tokio::test]
    async fn plugin_init_prints_each_admitted_mode_without_attempting_a_node_connection() {
        for mode in [
            vec![],
            vec!["--json"],
            vec!["-o", "yaml"],
            vec!["--quiet"],
            vec!["--no-headers"],
        ] {
            let root = tempfile::tempdir().expect("isolated CLI working directory");
            let mut args = vec!["sample"];
            args.extend(&mode);
            let output = init(root.path(), &args).await;
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(output.stderr.is_empty());
            let text = String::from_utf8(output.stdout).expect("UTF-8 output");
            match mode.as_slice() {
                ["--json"] => {
                    let rows: serde_json::Value =
                        serde_json::from_str(&text).expect("structured rows");
                    assert_eq!(rows[0]["id"], "sample");
                    assert_eq!(
                        rows[0]["directory"],
                        root.path()
                            .join("sample")
                            .canonicalize()
                            .expect("path")
                            .to_str()
                            .expect("UTF-8 test path")
                    );
                }
                ["-o", "yaml"] => {
                    let rows: serde_json::Value = serde_json::from_str(
                        text.strip_prefix("---\n").expect("YAML1.2 flow output"),
                    )
                    .expect("flow rows");
                    assert_eq!(rows[0]["id"], "sample");
                }
                ["--quiet"] => assert_eq!(text, "sample\n"),
                ["--no-headers"] => assert!(!text.contains("PLUGIN")),
                [] => assert!(text.starts_with("PLUGIN  DIRECTORY\n")),
                _ => panic!("fixed output fixture"),
            }
            let bytes =
                std::fs::read(root.path().join("sample/client.json")).expect("generated document");
            assert!(ClientDocument::decode(&bytes).is_ok());
        }
    }

    #[tokio::test]
    async fn invalid_identity_and_nonempty_targets_fail_without_overwriting_files() {
        let root = tempfile::tempdir().expect("isolated CLI working directory");
        let output = init(root.path(), &["INVALID-PRIVATE-CANARY"]).await;
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("PRIVATE-CANARY"));
        assert!(!root.path().join("INVALID-PRIVATE-CANARY").exists());
        let directory = root.path().join("sample");
        std::fs::create_dir(&directory).expect("existing project");
        std::fs::write(directory.join("original"), b"keep original").expect("existing content");
        let output = init(root.path(), &["sample", "--json"]).await;
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert_eq!(
            std::fs::read(directory.join("original")).expect("original"),
            b"keep original"
        );
        assert_eq!(
            std::fs::read_dir(directory)
                .expect("unchanged project")
                .count(),
            1
        );
    }
}
