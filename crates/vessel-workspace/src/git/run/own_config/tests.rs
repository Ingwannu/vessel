//! ADR 0030 held: each way a repository's own configuration names a
//! program, planted in a real repository, and shown not to run (or the
//! run refused, naming the key). The judgement itself is tested on
//! listings first.

use super::{OwnConfigRefusal, Refused, Uses, has_worktree_config, judge};

const READ: Uses = Uses {
    network: false,
    hooks: false,
};
const FETCH: Uses = Uses {
    network: true,
    hooks: false,
};
const COMMIT: Uses = Uses {
    network: false,
    hooks: true,
};

fn listing(entries: &[&str]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for entry in entries {
        bytes.extend_from_slice(entry.as_bytes());
        bytes.push(0);
    }
    bytes
}

fn refused(key: &str, why: OwnConfigRefusal) -> Result<Vec<String>, Refused> {
    Err(Refused {
        key: key.to_owned(),
        why,
    })
}

/// A diff driver is left to the flags every diff passes (it cannot be
/// emptied); filters and merge drivers are turned off by name.
#[test]
fn every_driver_the_repository_defines_is_turned_off_by_name() {
    let settings = judge(
        &listing(&[
            "core.repositoryformatversion\n0",
            "filter.Evil.clean\nrun-me",
            "filter.Evil.smudge\nrun-me",
            "filter.lfs.process\ngit-lfs filter-process",
            "diff.pics.textconv\nexif",
            "diff.pics.command\nexif",
            "merge.ours.driver\ntrue",
            "user.name\nSomeone",
        ]),
        READ,
    )
    .unwrap();
    assert_eq!(
        settings,
        [
            "filter.Evil.clean=",
            "filter.Evil.smudge=",
            "filter.Evil.process=",
            "filter.Evil.required=false",
            "filter.lfs.clean=",
            "filter.lfs.smudge=",
            "filter.lfs.process=",
            "filter.lfs.required=false",
            "merge.ours.driver=",
        ]
    );
}

#[test]
fn a_program_is_refused_only_where_the_run_would_start_it() {
    for key in [
        "core.sshcommand",
        "core.gitproxy",
        "core.alternaterefscommand",
        "credential.helper",
        "credential.https://example.com.helper",
        "remote.origin.uploadpack",
        "remote.origin.receivepack",
        "remote.origin.vcs",
    ] {
        let config = listing(&[&format!("{key}\nrun-me")]);
        assert_eq!(judge(&config, READ), Ok(Vec::new()), "{key} on a read");
        assert_eq!(
            judge(&config, FETCH),
            refused(key, OwnConfigRefusal::Program),
            "{key} on a fetch"
        );
    }
    for key in [
        "gpg.program",
        "gpg.ssh.program",
        "gpg.x509.program",
        "gpg.ssh.defaultkeycommand",
    ] {
        let config = listing(&[&format!("{key}\nrun-me")]);
        assert_eq!(judge(&config, READ), Ok(Vec::new()), "{key} on a read");
        assert_eq!(
            judge(&config, COMMIT),
            refused(key, OwnConfigRefusal::Program),
            "{key} on a commit"
        );
    }
}

#[test]
fn what_could_send_the_credential_elsewhere_refuses_a_network_run() {
    for key in [
        "http.proxy",
        "http.https://example.com.proxy",
        "http.sslverify",
        "http.sslcainfo",
        "http.curloptresolve",
        "http.extraheader",
        "remote.origin.proxy",
    ] {
        let config = listing(&[&format!("{key}\nsomething")]);
        assert_eq!(judge(&config, READ), Ok(Vec::new()), "{key} on a read");
        assert_eq!(
            judge(&config, FETCH),
            refused(key, OwnConfigRefusal::Credential),
            "{key} on a fetch"
        );
    }
    // A buffer size is not one of them.
    assert_eq!(
        judge(&listing(&["http.postbuffer\n524288000"]), FETCH),
        Ok(Vec::new())
    );
}

#[test]
fn a_moved_working_tree_refuses_every_run_and_bare_does_not() {
    assert_eq!(
        judge(&listing(&["core.worktree\n/elsewhere"]), READ),
        refused("core.worktree", OwnConfigRefusal::Layout)
    );
    // A bare repository's linked worktrees share its `core.bare = true`.
    assert_eq!(judge(&listing(&["core.bare\ntrue"]), READ), Ok(Vec::new()));
}

#[test]
fn a_driver_that_cannot_be_named_on_the_command_line_refuses() {
    assert_eq!(
        judge(&listing(&["filter.a=b.clean\nrun-me"]), READ),
        refused("filter.a=b.clean", OwnConfigRefusal::Unnamable)
    );
    let many: Vec<String> = (0..=super::MAX_DRIVERS)
        .map(|index| format!("filter.d{index}.clean\nrun-me"))
        .collect();
    let many: Vec<&str> = many.iter().map(String::as_str).collect();
    assert_eq!(
        judge(&listing(&many), READ),
        refused(
            &format!("filter.d{}.clean", super::MAX_DRIVERS),
            OwnConfigRefusal::TooMany
        )
    );
}

#[test]
fn the_worktree_configuration_is_read_when_it_is_on() {
    assert!(has_worktree_config(&listing(&[
        "extensions.worktreeconfig\ntrue"
    ])));
    assert!(!has_worktree_config(&listing(&[
        "extensions.worktreeconfig\nfalse"
    ])));
    assert!(!has_worktree_config(&listing(&["core.bare\nfalse"])));
}

/// The planted vectors, run through the node's own git.
#[cfg(unix)]
mod planted {
    use std::path::{Path, PathBuf};

    use crate::git::{Git, GitError, OwnConfigRefusal};
    use crate::testing::Sandbox;

    /// A repository whose own configuration names `marker`'s program for
    /// every planted key, and the marker each one would touch.
    struct Planted {
        sandbox: Sandbox,
        repo: PathBuf,
        git: Git,
        script: PathBuf,
    }

    impl Planted {
        async fn new() -> Self {
            let sandbox = Sandbox::new();
            let repo = sandbox.repo("r");
            let git = sandbox.git().await;
            let script = sandbox.path().join("planted.sh");
            // Touches `ran-<what>`, then passes its input through, as a
            // filter or a pager would.
            Sandbox::executable(
                &script,
                &format!(
                    "#!/bin/sh\ntouch '{}/ran-'\"$1\"\ncat\n",
                    sandbox.path().display()
                ),
            );
            Self {
                sandbox,
                repo,
                git,
                script,
            }
        }

        /// `git config <key> "<script> <what>"` in the repository.
        fn plant(&self, key: &str, what: &str) {
            let value = format!("{} {what}", self.script.display());
            self.sandbox.run(&self.repo, &["config", key, &value]);
        }

        /// Every `ran-*` marker there is now.
        fn ran(&self) -> Vec<String> {
            let mut ran: Vec<String> = std::fs::read_dir(self.sandbox.path())
                .unwrap()
                .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
                .filter(|name| name.starts_with("ran-"))
                .collect();
            ran.sort();
            ran
        }

        fn write(&self, path: &str, text: &str) {
            Sandbox::write(&self.repo, path, text);
        }

        fn commit_all(&self, message: &str) {
            self.sandbox.run(&self.repo, &["add", "--all"]);
            self.sandbox
                .run(&self.repo, &["commit", "-q", "-m", message]);
        }
    }

    fn assert_refused(result: Result<impl std::fmt::Debug, GitError>, key: &str) {
        match result {
            Err(GitError::OwnConfig { key: refused, why }) => {
                assert_eq!(refused, key);
                assert_eq!(why, OwnConfigRefusal::Program);
            }
            other => panic!("expected `{key}` refused, got {other:?}"),
        }
    }

    /// Filters, text conversion and diff commands that the repository
    /// defines, chosen by its attributes, on a status, a diff, staging,
    /// a commit and a worktree's checkout.
    #[tokio::test]
    async fn drivers_the_repository_defines_never_run() {
        let planted = Planted::new().await;
        planted.write(".gitattributes", "*.txt filter=evil diff=evil\n");
        planted.write("a.txt", "one\n");
        planted.commit_all("attributes");
        planted.plant("filter.evil.clean", "clean");
        planted.plant("filter.evil.smudge", "smudge");
        planted.plant("diff.evil.textconv", "textconv");
        planted.plant("diff.evil.command", "diffcmd");
        planted.write("a.txt", "two\n");
        let repo = &planted.repo;
        let git = &planted.git;

        let _status = git.status(repo).await.unwrap();
        let _changes = git
            .command(repo)
            .args(["diff", "--stat"])
            .output()
            .await
            .unwrap()
            .checked()
            .unwrap();
        // Every diff the node makes passes `--no-ext-diff --no-textconv`.
        let scope = git
            .resolve_scope(repo, &vessel_proto::workspace::DiffScope::Uncommitted, None)
            .await
            .unwrap();
        let summary = git.changes(repo, &scope, false).await.unwrap();
        assert_eq!(summary.files.len(), 1);
        let path: vessel_proto::workspace::RelPath = "a.txt".parse().unwrap();
        let _patch = git.file_diff(repo, &scope, &path, false, 0).await.unwrap();
        git.command(repo)
            .args(["add", "--all"])
            .run()
            .await
            .unwrap();
        git.command(repo)
            .args(["commit", "-q", "-m", "through the node"])
            .with_hooks()
            .run()
            .await
            .unwrap();
        let worktree = planted.sandbox.path().join("w");
        git.command(repo)
            .args(["worktree", "add", "--quiet", "--detach"])
            .arg(&worktree)
            .run()
            .await
            .unwrap();
        assert_eq!(planted.ran(), Vec::<String>::new());
        // The same keys in the user's own git would have run: the plant
        // works, the node's git is what does not run it.
        planted.write("a.txt", "three\n");
        planted.sandbox.run(repo, &["diff", "--stat"]);
        assert!(!planted.ran().is_empty(), "the planted filter is real");
    }

    /// A merge driver: a stash applied onto a change of the same file
    /// merges it, and the repository's driver does not run (git's merge of
    /// that file fails instead).
    #[tokio::test]
    async fn a_merge_driver_never_runs() {
        let planted = Planted::new().await;
        planted.write(".gitattributes", "*.txt merge=evil\n");
        planted.write("a.txt", "base\n");
        planted.commit_all("base");
        planted.plant("merge.evil.driver", "merge");
        planted.write("a.txt", "mine\n");
        planted.sandbox.run(&planted.repo, &["stash", "-q"]);
        planted.write("a.txt", "theirs\n");
        planted.commit_all("theirs");
        let applied = planted
            .git
            .command(&planted.repo)
            .args(["stash", "apply"])
            .output()
            .await
            .unwrap();
        assert!(!applied.succeeded());
        assert_eq!(planted.ran(), Vec::<String>::new());
    }

    /// `fsmonitor`, a pager (`core.pager` and `pager.<command>`), an
    /// editor, an alias shadowing a command, and a hooks folder: none runs
    /// on a status, a log or a worktree's creation.
    #[tokio::test]
    async fn monitors_pagers_editors_aliases_and_hooks_folders_never_run() {
        let planted = Planted::new().await;
        planted.plant("core.fsmonitor", "fsmonitor");
        planted.plant("core.pager", "pager");
        planted.plant("pager.log", "pagerlog");
        planted.plant("core.editor", "editor");
        planted.plant("sequence.editor", "sequence");
        let alias = format!("!{} alias", planted.script.display());
        planted
            .sandbox
            .run(&planted.repo, &["config", "alias.status", &alias]);
        let hooks = planted.sandbox.path().join("planted-hooks");
        std::fs::create_dir(&hooks).unwrap();
        let hook = hooks.join("post-checkout");
        Sandbox::executable(
            &hook,
            &format!("#!/bin/sh\n{} hook\n", planted.script.display()),
        );
        planted.sandbox.run(
            &planted.repo,
            &["config", "core.hooksPath", hooks.to_str().unwrap()],
        );
        planted.write("README.md", "changed\n");
        let repo = &planted.repo;
        let git = &planted.git;
        let _status = git.status(repo).await.unwrap();
        git.command(repo).args(["log", "-1"]).run().await.unwrap();
        git.command(repo)
            .args(["worktree", "add", "--quiet", "--detach"])
            .arg(planted.sandbox.path().join("w"))
            .run()
            .await
            .unwrap();
        assert_eq!(planted.ran(), Vec::<String>::new());
    }

    /// A key put in a file the repository's configuration includes, by
    /// `include` and by `includeIf`, is the repository's own too.
    #[tokio::test]
    async fn included_files_are_the_repository_s_own() {
        let planted = Planted::new().await;
        planted.write(".gitattributes", "*.txt filter=hidden\n");
        planted.write("a.txt", "one\n");
        planted.commit_all("attributes");
        let included = planted.sandbox.path().join("included.cfg");
        std::fs::write(
            &included,
            format!(
                "[filter \"hidden\"]\n\tclean = {} included\n[core]\n\tfsmonitor = {} fsmon\n",
                planted.script.display(),
                planted.script.display()
            ),
        )
        .unwrap();
        planted.sandbox.run(
            &planted.repo,
            &["config", "include.path", included.to_str().unwrap()],
        );
        let conditional = planted.sandbox.path().join("conditional.cfg");
        std::fs::write(
            &conditional,
            format!(
                "[core]\n\tsshCommand = {} conditional\n",
                planted.script.display()
            ),
        )
        .unwrap();
        planted.sandbox.run(
            &planted.repo,
            &[
                "config",
                "includeIf.gitdir:/.path",
                conditional.to_str().unwrap(),
            ],
        );
        planted.write("a.txt", "two\n");
        let _status = planted.git.status(&planted.repo).await.unwrap();
        planted
            .git
            .command(&planted.repo)
            .args(["diff", "--stat"])
            .run()
            .await
            .unwrap();
        assert_eq!(planted.ran(), Vec::<String>::new());
        assert_refused(
            planted
                .git
                .command(&planted.repo)
                .args(["fetch", "origin"])
                .network()
                .output()
                .await,
            "core.sshcommand",
        );
    }

    /// What a network run would start: refused before it runs, naming the
    /// key; a transport that runs a program is not allowed at all.
    #[tokio::test]
    async fn network_programs_refuse_the_run() {
        for (key, remote) in [
            ("core.sshCommand", "ssh://git@127.0.0.1:9/r.git"),
            ("core.gitProxy", "git://127.0.0.1:9/r.git"),
            ("credential.helper", "http://127.0.0.1:9/r.git"),
            ("remote.origin.uploadpack", "/nowhere/r.git"),
            ("remote.origin.vcs", "http://127.0.0.1:9/r.git"),
        ] {
            let planted = Planted::new().await;
            planted
                .sandbox
                .run(&planted.repo, &["remote", "add", "origin", remote]);
            planted.plant(key, "network");
            let fetched = planted
                .git
                .command(&planted.repo)
                .args(["fetch", "origin"])
                .network()
                .output()
                .await;
            assert_refused(fetched, &key.to_ascii_lowercase());
            assert_eq!(planted.ran(), Vec::<String>::new(), "{key}");
        }
        // A remote rewritten to `ext::` by the repository: git refuses the
        // transport; nothing runs.
        let planted = Planted::new().await;
        let ext = format!("ext::{} ext", planted.script.display());
        planted.sandbox.run(
            &planted.repo,
            &["remote", "add", "origin", "https://example.invalid/r"],
        );
        planted.sandbox.run(
            &planted.repo,
            &["config", "url.ext::.insteadOf", "https://example.invalid/"],
        );
        planted
            .sandbox
            .run(&planted.repo, &["config", "protocol.allow", "always"]);
        planted
            .sandbox
            .run(&planted.repo, &["remote", "set-url", "origin", &ext]);
        let fetched = planted
            .git
            .command(&planted.repo)
            .args(["fetch", "origin"])
            .network()
            .output()
            .await
            .unwrap();
        assert!(!fetched.succeeded());
        assert_eq!(planted.ran(), Vec::<String>::new());
    }

    /// A signing program the repository names refuses a commit; the user's
    /// own hooks still run on it (the one thing kept, ADR 0030).
    #[tokio::test]
    async fn a_signing_program_refuses_a_commit() {
        let planted = Planted::new().await;
        planted
            .sandbox
            .run(&planted.repo, &["config", "commit.gpgSign", "true"]);
        planted.plant("gpg.program", "gpg");
        let committed = planted
            .git
            .command(&planted.repo)
            .args(["commit", "-q", "--allow-empty", "-m", "signed?"])
            .with_hooks()
            .output()
            .await;
        assert_refused(committed, "gpg.program");
        assert_eq!(planted.ran(), Vec::<String>::new());
    }

    /// A working tree moved out of the folder refuses every run.
    #[tokio::test]
    async fn a_moved_working_tree_refuses_every_run() {
        let planted = Planted::new().await;
        let elsewhere = planted.sandbox.path().join("elsewhere");
        std::fs::create_dir(&elsewhere).unwrap();
        planted.sandbox.run(
            &planted.repo,
            &["config", "core.worktree", elsewhere.to_str().unwrap()],
        );
        match planted.git.status(&planted.repo).await {
            Err(GitError::OwnConfig { key, why }) => {
                assert_eq!(key, "core.worktree");
                assert_eq!(why, OwnConfigRefusal::Layout);
            }
            other => panic!("{other:?}"),
        }
    }

    /// A worktree's own `config.worktree` is listed too.
    #[tokio::test]
    async fn a_worktree_s_own_configuration_is_read() {
        let planted = Planted::new().await;
        planted.write(".gitattributes", "*.txt filter=wt\n");
        planted.write("a.txt", "one\n");
        planted.commit_all("attributes");
        planted.sandbox.run(
            &planted.repo,
            &["config", "extensions.worktreeConfig", "true"],
        );
        let value = format!("{} worktree", planted.script.display());
        planted.sandbox.run(
            &planted.repo,
            &["config", "--worktree", "filter.wt.clean", &value],
        );
        planted.write("a.txt", "two\n");
        planted
            .git
            .command(&planted.repo)
            .args(["diff", "--stat"])
            .run()
            .await
            .unwrap();
        assert_eq!(planted.ran(), Vec::<String>::new());
    }

    /// Outside a repository nothing is listed, and the run goes ahead.
    #[tokio::test]
    async fn outside_a_repository_the_run_goes_ahead() {
        let planted = Planted::new().await;
        let plain: &Path = planted.sandbox.path();
        let output = planted
            .git
            .command(plain)
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .await
            .unwrap();
        assert_ne!(output.code, Some(0), "not a repository");
    }
}
