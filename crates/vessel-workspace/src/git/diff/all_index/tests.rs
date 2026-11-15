//! The real-index regression from checkout-git tests, using production git
//! commands through the protected runner. No network or installed agent.

use vessel_proto::workspace::{
    ChangeStatus, DiffContent, GitComparison, GitHunkOffset, GitWhitespace, WorktreeBase,
};

use super::*;
use crate::testing::Sandbox;

fn base() -> WorktreeBase {
    WorktreeBase::Ref {
        name: "main".parse().unwrap(),
    }
}

#[tokio::test]
async fn recreated_base_file_is_compared_once_and_the_real_index_is_unchanged() {
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("all");
    let git = sandbox.git().await;
    sandbox.run(&repo, &["checkout", "-q", "-b", "topic"]);
    sandbox.run(&repo, &["rm", "-q", "README.md"]);
    sandbox.run(&repo, &["commit", "-q", "-m", "remove"]);
    Sandbox::write(&repo, "README.md", "hello\nchanged\n");
    // Git's lock-file replacement otherwise widens the private copy's mode.
    sandbox.run(&repo, &["config", "core.sharedRepository", "group"]);
    let before_mode = std::fs::metadata(repo.join(".git/index"))
        .unwrap()
        .permissions();
    let before = std::fs::read(repo.join(".git/index")).unwrap();
    let prepared = git
        .prepare_comparison(
            &repo,
            &GitComparison::All { base: None },
            Some(&base()),
            sandbox.path(),
        )
        .await
        .unwrap()
        .unwrap();
    let summary = prepared
        .changes(&git, &repo, GitWhitespace::Show)
        .await
        .unwrap();
    assert_eq!(summary.files.len(), 1);
    let changed = summary.files.first().unwrap();
    assert_eq!(changed.path.as_str(), "README.md");
    assert_eq!(changed.status, ChangeStatus::Modified);
    assert_eq!((changed.additions, changed.deletions), (Some(1), Some(0)));
    let file = prepared
        .file(
            &git,
            &repo,
            &"README.md".parse().unwrap(),
            GitWhitespace::Show,
            GitHunkOffset::default(),
        )
        .await
        .unwrap();
    assert_eq!(file.status, ChangeStatus::Modified);
    assert!(matches!(file.content, DiffContent::Hunks { .. }));
    let private = prepared.directory.as_ref().unwrap().path().to_owned();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        assert_eq!(
            std::fs::metadata(private.join("index"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::metadata(&private).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            std::fs::metadata(repo.join(".git/index"))
                .unwrap()
                .permissions()
                .mode(),
            before_mode.mode()
        );
    }
    #[cfg(not(unix))]
    let _before_mode = before_mode;
    prepared.close().await.unwrap();
    assert!(!private.exists());
    assert_eq!(std::fs::read(repo.join(".git/index")).unwrap(), before);
}

#[tokio::test]
async fn identically_recreated_file_is_unchanged_in_summary_and_body() {
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("identical");
    let git = sandbox.git().await;
    sandbox.run(&repo, &["checkout", "-q", "-b", "topic"]);
    sandbox.run(&repo, &["rm", "-q", "README.md"]);
    sandbox.run(&repo, &["commit", "-q", "-m", "remove"]);
    Sandbox::write(&repo, "README.md", "hello\n");
    let prepared = git
        .prepare_comparison(
            &repo,
            &GitComparison::All { base: None },
            Some(&base()),
            sandbox.path(),
        )
        .await
        .unwrap()
        .unwrap();
    assert!(
        prepared
            .changes(&git, &repo, GitWhitespace::Show)
            .await
            .unwrap()
            .files
            .is_empty()
    );
    let file = prepared
        .file(
            &git,
            &repo,
            &"README.md".parse().unwrap(),
            GitWhitespace::Show,
            GitHunkOffset::default(),
        )
        .await
        .unwrap();
    assert_eq!(file.content, DiffContent::Unchanged);
    prepared.close().await.unwrap();
}

#[tokio::test]
async fn restored_paths_are_literal_and_preserve_other_staged_renames() {
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("literal");
    let git = sandbox.git().await;
    let name = "odd [name],*.txt";
    Sandbox::write(&repo, name, "original\n");
    sandbox.run(&repo, &["add", "--", name]);
    sandbox.run(&repo, &["commit", "-q", "-m", "base file"]);
    sandbox.run(&repo, &["checkout", "-q", "-b", "topic"]);
    sandbox.run(&repo, &["rm", "-q", "--", name]);
    sandbox.run(&repo, &["commit", "-q", "-m", "remove"]);
    Sandbox::write(&repo, name, "original\nmodified\n");
    sandbox.run(&repo, &["mv", "README.md", "renamed.md"]);
    let before = std::fs::read(repo.join(".git/index")).unwrap();
    let prepared = git
        .prepare_comparison(
            &repo,
            &GitComparison::All { base: None },
            Some(&base()),
            sandbox.path(),
        )
        .await
        .unwrap()
        .unwrap();
    let summary = prepared
        .changes(&git, &repo, GitWhitespace::Show)
        .await
        .unwrap();
    assert_eq!(summary.files.len(), 2);
    let literal = summary
        .files
        .iter()
        .find(|file| file.path.as_str() == name)
        .unwrap();
    assert_eq!(literal.status, ChangeStatus::Modified);
    let rename = summary
        .files
        .iter()
        .find(|file| file.path.as_str() == "renamed.md")
        .unwrap();
    assert_eq!(rename.status, ChangeStatus::Renamed);
    assert_eq!(rename.old_path.as_ref().unwrap().as_str(), "README.md");
    prepared.close().await.unwrap();
    assert_eq!(std::fs::read(repo.join(".git/index")).unwrap(), before);
}

#[test]
fn batches_bound_path_count_and_conservatively_escaped_argv() {
    let short: Vec<RelPath> = (0..1000)
        .map(|index| format!("file-{index}").parse().unwrap())
        .collect();
    let long: Vec<RelPath> = (0..8)
        .map(|index| {
            format!("{}/{index}", "a/".repeat(1900))
                .replace("//", "/")
                .parse()
                .unwrap()
        })
        .collect();
    for paths in [short, long] {
        let batches = batches(&paths);
        assert_eq!(
            batches.iter().map(|batch| batch.len()).sum::<usize>(),
            paths.len()
        );
        for batch in batches {
            assert!(batch.len() <= MAX_BATCH_PATHS);
            let cost = ARG_FIXED_BYTES
                + batch
                    .iter()
                    .map(|path| path.as_str().len() * 2 + CACHEINFO_ENTRY_BYTES)
                    .sum::<usize>();
            assert!(cost <= MAX_BATCH_ARG_BYTES);
        }
    }
}

#[test]
fn malformed_tree_entries_are_errors_instead_of_dropped_paths() {
    let id = "a".repeat(40);
    assert!(matches!(
        parse_entries(format!("100644 blob {id}\t../outside\0").as_bytes()),
        Err(IndexError::Path(InvalidRelPath::Part))
    ));
    for record in [
        format!("100644 commit {id}\tfile\0"),
        format!("777777 blob {id}\tfile\0"),
        format!("100644 blob {id}\t\0"),
    ] {
        assert!(matches!(
            parse_entries(record.as_bytes()),
            Err(IndexError::Git(GitError::Unexpected {
                what: "base index entry"
            }))
        ));
    }
}

#[test]
fn entry_decode_failures_retain_the_original_error_source() {
    use std::error::Error as _;

    let utf8 = parse_entries(&[0xff, 0]).unwrap_err();
    assert!(matches!(utf8, IndexError::Utf8(_)));
    assert!(utf8.source().is_some());
    let object = parse_entries(b"100644 blob invalid\tfile\0").unwrap_err();
    assert!(matches!(object, IndexError::Object(_)));
    assert!(object.source().is_some());
    let path = parse_entries(format!("100644 blob {}\t../outside\0", "a".repeat(40)).as_bytes())
        .unwrap_err();
    assert!(matches!(path, IndexError::Path(_)));
    assert!(path.source().is_some());
}

#[test]
fn private_index_copy_enforces_its_byte_budget_and_private_mode() {
    let sandbox = Sandbox::new();
    let source = sandbox.path().join("index-source");
    std::fs::write(&source, b"0123456789abcdefg").unwrap();
    let refused = copy_index(&source, IndexCopyLimit(16), sandbox.path()).unwrap_err();
    assert_eq!(refused.kind(), std::io::ErrorKind::FileTooLarge);
    let private = copy_index(&source, IndexCopyLimit(32), sandbox.path()).unwrap();
    assert_eq!(
        std::fs::read(private.path().join("index")).unwrap(),
        b"0123456789abcdefg"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        assert_eq!(
            std::fs::metadata(private.path())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        assert_eq!(
            std::fs::metadata(private.path().join("index"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    private.close().unwrap();
}

#[cfg(unix)]
#[test]
fn private_index_copy_does_not_follow_a_source_symlink() {
    let sandbox = Sandbox::new();
    let source = sandbox.path().join("index-source");
    std::fs::write(&source, b"index bytes").unwrap();
    let link = sandbox.path().join("index-link");
    std::os::unix::fs::symlink(&source, &link).unwrap();
    assert!(copy_index(&link, IndexCopyLimit(32), sandbox.path()).is_err());
    assert_eq!(std::fs::read(source).unwrap(), b"index bytes");
}

#[tokio::test]
async fn a_relative_scratch_parent_is_refused_before_reading_a_comparison() {
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("scratch");
    let git = sandbox.git().await;
    let result = git
        .prepare_comparison(
            &repo,
            &GitComparison::Uncommitted,
            None,
            Path::new("relative"),
        )
        .await;
    let Err(IndexError::Io(error)) = result else {
        panic!("relative scratch parent was accepted");
    };
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
}
