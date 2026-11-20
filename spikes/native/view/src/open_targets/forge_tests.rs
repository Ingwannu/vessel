//! Paseo's `git/forge-url.test.ts` (pinned 0.15.4), each remote as the
//! node sends it (host and path only), and the forge each status names.

use super::*;

fn page_of(
    forge: Forge,
    host: &str,
    path: &str,
    branch: &str,
    file: Option<LinkedFile<'_>>,
) -> Option<String> {
    page(
        forge,
        &host.parse().unwrap(),
        &path.parse().unwrap(),
        branch,
        file.as_ref(),
    )
}

const fn file(path: &str, line_start: Option<u32>, line_end: Option<u32>) -> LinkedFile<'_> {
    LinkedFile {
        path,
        line_start,
        line_end,
    }
}

#[test]
fn branch_pages_follow_each_forge_s_grammar() {
    let cases: [(Forge, &str, &str, &str, &str); 7] = [
        (
            Forge::GitHub,
            "github.com",
            "acme/repo",
            "feature/workspace-button",
            "https://github.com/acme/repo/tree/feature/workspace-button",
        ),
        // Reserved characters are encoded; the branch's slashes are kept.
        (
            Forge::GitHub,
            "github.com",
            "acme/repo",
            "feature/ship #42",
            "https://github.com/acme/repo/tree/feature/ship%20%2342",
        ),
        (
            Forge::GitLab,
            "gitlab.com",
            "group/sub/repo",
            "main",
            "https://gitlab.com/group/sub/repo/-/tree/main",
        ),
        (
            Forge::Gitea,
            "gitea.com",
            "acme/repo",
            "main",
            "https://gitea.com/acme/repo/src/branch/main",
        ),
        (
            Forge::Codeberg,
            "codeberg.org",
            "acme/repo",
            "main",
            "https://codeberg.org/acme/repo/src/branch/main",
        ),
        (
            Forge::Forgejo,
            "home-git.example.com:60443",
            "team/repo",
            "master",
            "https://home-git.example.com:60443/team/repo/src/branch/master",
        ),
        (
            Forge::Forgejo,
            "home-git.example.com",
            "team/repo",
            "master",
            "https://home-git.example.com/team/repo/src/branch/master",
        ),
    ];
    for (forge, host, path, branch, expected) in cases {
        assert_eq!(
            page_of(forge, host, path, branch, None).as_deref(),
            Some(expected),
            "{expected}"
        );
    }
    for detached in ["HEAD", "", "  "] {
        assert_eq!(
            page_of(Forge::GitHub, "github.com", "acme/repo", detached, None),
            None
        );
    }
}

#[test]
fn file_pages_carry_the_path_and_each_forge_s_line_anchor() {
    let index = |start, end| Some(file("src/index.ts", start, end));
    let cases: [(Forge, &str, &str, Option<LinkedFile<'_>>, &str); 11] = [
        (
            Forge::GitHub,
            "github.com",
            "acme/repo",
            index(None, None),
            "https://github.com/acme/repo/blob/main/src/index.ts",
        ),
        (
            Forge::GitHub,
            "github.com",
            "acme/repo",
            index(Some(12), None),
            "https://github.com/acme/repo/blob/main/src/index.ts#L12",
        ),
        (
            Forge::GitHub,
            "github.com",
            "acme/repo",
            index(Some(12), Some(20)),
            "https://github.com/acme/repo/blob/main/src/index.ts#L12-L20",
        ),
        (
            Forge::GitLab,
            "gitlab.com",
            "group/sub/repo",
            index(Some(12), Some(20)),
            "https://gitlab.com/group/sub/repo/-/blob/main/src/index.ts#L12-20",
        ),
        (
            Forge::Forgejo,
            "codeberg.org",
            "acme/repo",
            index(Some(12), Some(20)),
            "https://codeberg.org/acme/repo/src/branch/main/src/index.ts#L12-L20",
        ),
        // A self-hosted GitHub keeps its own host.
        (
            Forge::GitHub,
            "github.acme.internal",
            "team/repo",
            index(None, None),
            "https://github.acme.internal/team/repo/blob/main/src/index.ts",
        ),
        (
            Forge::GitHub,
            "ssh.github.com",
            "acme/repo",
            index(None, None),
            "https://github.com/acme/repo/blob/main/src/index.ts",
        ),
        (
            Forge::GitHub,
            "github.com",
            "acme/repo",
            Some(file("/src/a b/c#d.ts", None, None)),
            "https://github.com/acme/repo/blob/main/src/a%20b/c%23d.ts",
        ),
        (
            Forge::GitHub,
            "github.com",
            "acme/repo",
            Some(file("./src/../index.ts", None, None)),
            "https://github.com/acme/repo/blob/main/index.ts",
        ),
        // An end before the start is a single line.
        (
            Forge::GitHub,
            "github.com",
            "acme/repo",
            index(Some(12), Some(3)),
            "https://github.com/acme/repo/blob/main/src/index.ts#L12",
        ),
        (
            Forge::GitHub,
            "github.com",
            "acme/repo",
            index(Some(0), None),
            "https://github.com/acme/repo/blob/main/src/index.ts",
        ),
    ];
    for (forge, host, path, linked, expected) in cases {
        assert_eq!(
            page_of(forge, host, path, "main", linked).as_deref(),
            Some(expected),
            "{expected}"
        );
    }
    let self_hosted = page_of(
        Forge::Forgejo,
        "home-git.example.com:60443",
        "team/repo",
        "master",
        index(Some(12), Some(20)),
    );
    assert_eq!(
        self_hosted.as_deref(),
        Some("https://home-git.example.com:60443/team/repo/src/branch/master/src/index.ts#L12-L20")
    );
    for outside in ["../outside.ts", ""] {
        assert_eq!(
            page_of(
                Forge::GitHub,
                "github.com",
                "acme/repo",
                "main",
                Some(file(outside, None, None))
            ),
            None,
            "{outside}"
        );
    }
}

#[test]
fn the_forge_is_the_node_s_word_else_the_public_host_s() {
    let of = |host: &str, kind| forge_of(&host.parse().unwrap(), kind);
    assert_eq!(of("github.com", None), Some(Forge::GitHub));
    assert_eq!(of("ssh.github.com", None), Some(Forge::GitHub));
    assert_eq!(of("gitlab.com", None), Some(Forge::GitLab));
    assert_eq!(of("gitea.com", None), Some(Forge::Gitea));
    assert_eq!(of("codeberg.org", None), Some(Forge::Codeberg));
    assert_eq!(of("github.acme.internal", None), None);
    assert_eq!(
        of("github.acme.internal", Some(ForgeKind::Github)),
        Some(Forge::GitHub)
    );
    assert_eq!(of("gitlab.com", Some(ForgeKind::Unknown)), Some(Forge::GitLab));
    assert_eq!(of("bitbucket.org", None), None);
    assert_eq!(of("gitlab.example.com", None), None);
    for forge in Forge::ALL {
        assert_eq!(Forge::from_id(forge.id()), Some(forge));
    }
    assert_eq!(Forge::from_id("bitbucket"), None);
    assert_eq!(Forge::GitLab.label(), "GitLab");
}
