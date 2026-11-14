//! Repositories made in temporary folders for this crate's tests, with the
//! real `git`, isolated from the machine's own configuration (no global or
//! system file, a fixed identity, no signing), so a developer's settings
//! never change a result.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::git::{Git, GitSetup};

/// A temporary folder with git's configuration of its own.
pub(crate) struct Sandbox {
    pub(crate) temp: tempfile::TempDir,
    env: Vec<(String, OsString)>,
}

impl Sandbox {
    pub(crate) fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let config = temp.path().join("gitconfig");
        std::fs::write(
            &config,
            "[user]\n\tname = Test\n\temail = test@example.invalid\n\
             [init]\n\tdefaultBranch = main\n\
             [commit]\n\tgpgsign = false\n\
             [core]\n\tautocrlf = false\n",
        )
        .unwrap();
        std::fs::create_dir(temp.path().join("no-hooks")).unwrap();
        let env = vec![
            ("GIT_CONFIG_GLOBAL".to_owned(), config.into_os_string()),
            ("GIT_CONFIG_NOSYSTEM".to_owned(), OsString::from("1")),
        ];
        Self { temp, env }
    }

    pub(crate) fn path(&self) -> &Path {
        self.temp.path()
    }

    /// The node's `Git`, set up as a node sets it up, isolated.
    pub(crate) async fn git(&self) -> Git {
        let program = which::which("git").expect("the tests need git on PATH");
        Git::probe(Some(GitSetup {
            program,
            no_hooks: self.temp.path().join("no-hooks"),
            env: self.env.clone(),
        }))
        .await
        .git
        .expect("git is usable")
    }

    /// The node's `Git` as [`git`](Self::git) makes it, running `program`
    /// instead of the `git` on `PATH` (a test's wrapper around it).
    pub(crate) async fn git_through(&self, program: PathBuf) -> Git {
        Git::probe(Some(GitSetup {
            program,
            no_hooks: self.temp.path().join("no-hooks"),
            env: self.env.clone(),
        }))
        .await
        .git
        .expect("git is usable through the wrapper")
    }

    /// A new repository `name` with one commit of `README.md`.
    pub(crate) fn repo(&self, name: &str) -> PathBuf {
        let dir = self.temp.path().join(name);
        std::fs::create_dir_all(&dir).unwrap();
        self.run(&dir, &["init", "-q"]);
        std::fs::write(dir.join("README.md"), "hello\n").unwrap();
        self.run(&dir, &["add", "README.md"]);
        self.run(&dir, &["commit", "-q", "-m", "first"]);
        dir
    }

    /// Run git in `dir` (the test's own setup steps), panicking on failure.
    pub(crate) fn run(&self, dir: &Path, args: &[&str]) {
        let output = Command::new("git")
            .current_dir(dir)
            .envs(self.env.iter().map(|(name, value)| (name, value)))
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// Write `text` to `relative` in `dir`, making its folders.
    pub(crate) fn write(dir: &Path, relative: &str, text: &str) {
        let path = dir.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    /// Write an executable script at `path`, one this crate's code then runs.
    ///
    /// Not with `std::fs::write`: while this process holds the file open
    /// for writing, a test on another thread may fork, and until that child
    /// execs it holds the descriptor too; running the script then fails
    /// with `ETXTBSY` ("Text file busy"), which a probe reports as no git.
    /// So the text goes to a side file and `cp`, a process of its own,
    /// creates the script: no descriptor for it is ever open here. (The
    /// same helper as the remote and CLI tests'.)
    #[cfg(unix)]
    pub(crate) fn executable(path: &Path, text: &str) {
        use std::os::unix::fs::PermissionsExt as _;

        let mut source = path.as_os_str().to_owned();
        source.push(".source");
        std::fs::write(&source, text).unwrap();
        let copied = Command::new("cp").arg(&source).arg(path).status().unwrap();
        assert!(copied.success());
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}
