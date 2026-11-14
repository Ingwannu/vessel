#!/usr/bin/env python3
"""Real FakeForge facts through disposable nodes; never an HTTP/account mock."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import uuid

HOST = "forge.example.invalid"
WHEN = "2026-01-05T09:15:00Z"
CASES = ["ready", "failing", "pending", "changes-requested", "conflicting",
         "draft", "unknown", "merge-refused", "no-access", "rate-limited", "empty"]


def prepare(root: Path, _target: Path) -> None:
    home = root / "forge-git-home"
    home.mkdir()
    env = {"PATH": os.environ["PATH"], "HOME": str(home), "GIT_CONFIG_NOSYSTEM": "1",
           "GIT_CONFIG_GLOBAL": os.devnull, "GIT_TERMINAL_PROMPT": "0"}
    git = shutil.which("git", path=env["PATH"])
    assert git, "git is required for fixture repositories"

    def run(folder: Path, *args: str) -> str:
        result = subprocess.run([git, "-C", str(folder), "-c", "core.hooksPath=/dev/null",
                                 "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid",
                                 *args], env=env, capture_output=True, text=True,
                                timeout=30, check=False)
        assert result.returncode == 0, "could not prepare a local fixture repository"
        return result.stdout.strip()

    repositories, manifest = [], []
    for index, case in enumerate(CASES, 301):
        name = f"forge-{case}"
        folder = root / "work" / name
        bare = root / "forge-remotes" / f"{name}.git"
        folder.mkdir(parents=True)
        bare.mkdir(parents=True)
        run(bare, "init", "--bare", "-b", "main")
        run(folder, "init", "-b", "main")
        (folder / "README.md").write_text(f"# {name}\n", encoding="utf-8")
        run(folder, "add", "README.md")
        run(folder, "commit", "-m", "Fixture base")
        base = run(folder, "rev-parse", "HEAD")
        branch = "feature/long-branch-for-checks-reviews-and-merge-state"
        run(folder, "switch", "-c", branch)
        (folder / "change.txt").write_text("A disposable pull request change.\n", encoding="utf-8")
        run(folder, "add", "change.txt")
        run(folder, "commit", "-m", "Fixture change")
        head = run(folder, "rev-parse", "HEAD")
        run(folder, "remote", "add", "origin", f"https://{HOST}/fixture/{name}.git")
        # An explicit PR-open action can only push to this scratch bare repo.
        run(folder, "remote", "set-url", "--push", "origin", str(bare))
        run(folder, "push", "origin", "main", branch)
        run(folder, "update-ref", "refs/remotes/origin/main", base)
        run(folder, "update-ref", f"refs/remotes/origin/{branch}", head)
        run(folder, "symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main")
        run(folder, "config", f"branch.{branch}.remote", "origin")
        run(folder, "config", f"branch.{branch}.merge", f"refs/heads/{branch}")
        repo = {"kind": "github", "host": HOST, "path": f"fixture/{name}"}
        checks = "failing" if case == "failing" else "pending" if case == "pending" else "passing"
        review = "changes_requested" if case == "changes-requested" else "approved"
        mergeable = {"conflicting": "conflicting", "unknown": "unknown",
                     "changes-requested": "blocked", "draft": "blocked"}.get(case, "mergeable")
        title = ("검사와 리뷰를 확인하는 긴 변경 사항 — "
                 "Keep every pull request fact visible when the title wraps across several lines")
        summary = {"repo": repo, "number": index, "url": f"https://{HOST}/fixture/{name}/pull/{index}",
                   "title": title, "state": "open", "draft": case == "draft", "head_commit": head,
                   "checks": checks, "review": review, "mergeable": mergeable, "updated_at": WHEN}
        pull = {"summary": summary, "author": "fixture-author", "head_branch": branch, "base": "main",
                "checks": {"state": checks, "passed": 1240, "failed": 2 if case == "failing" else 0,
                           "pending": 3 if case == "pending" else 0, "truncated": case == "failing",
                           "failing": ([{"name": "한글 검사 / integration with a long failure name",
                                         "state": "failed"}, {"name": "Linux / workspace", "state": "failed"}]
                                       if case == "failing" else [])},
                "reviews": {"approved": review == "approved", "changes_requested": review == "changes_requested",
                            "commented": case == "changes-requested", "unresolved": 23 if case == "changes-requested" else 0},
                "created_at": WHEN, "read_at": WHEN, "merge_methods": ["merge", "squash", "rebase"]}
        entry = {"repo": repo, "pulls": [] if case == "empty" else [
            {"pull": pull, "merge": "refused" if case == "merge-refused" else "normal"}]}
        if case in ["no-access", "rate-limited"]:
            entry["failure"] = case.replace("-", "_")
        repositories.append(entry)
        manifest.append({"case": case, "project": name, "path": str(folder), "head": head, "number": index})
    (root / "forge-seed.json").write_text(json.dumps({"version": 1, "repositories": repositories},
                                                     ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    (root / "forge-fixtures.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


def check(root: Path, target: Path) -> None:
    env = {"PATH": os.environ["PATH"], "HOME": str(root), "VESSEL_CONFIG_DIR": str(root / "config")}

    def call(*args: str, readonly: bool = False) -> subprocess.CompletedProcess[str]:
        selected = {**env, "VESSEL_CONFIG_DIR": str(root / "readonly-config")} if readonly else env
        return subprocess.run([str(target / "vessel"), "--node", "laptop", *args],
                              env=selected, capture_output=True, text=True, timeout=45, check=False)

    def success(*args: str, readonly: bool = False) -> str:
        result = call(*args, readonly=readonly)
        assert result.returncode == 0, f"CLI refused fixture {args[0]}"
        return result.stdout

    manifest = json.loads((root / "forge-fixtures.json").read_text())
    sessions = []
    for item in manifest:
        case = item["case"]
        success("project", "add", item["path"], "--name", item["project"])
        created = success("run", "claude-code", "--structured", "--project", item["project"],
                          "--title", f"Pull request: {case}", "--detach")
        session = created.strip().splitlines()[-1]
        uuid.UUID(session)
        item["session"] = session
        sessions.append(f"laptop {session} {item['project']}")
        # Explicit refresh runs folder -> provider -> node projection. No
        # turn or background-timer polling is needed for these facts.
        shown = success("pr", "show", session, "--refresh")
        assert "feedback: off" in shown, f"feedback unexpectedly on: {case}"
        if case == "no-access":
            assert "NoAccess" in shown and f"#{item['number']} " not in shown, "no-access projection missing"
        elif case == "rate-limited":
            assert "limits requests until" in shown and f"#{item['number']} " not in shown, "rate-limit projection missing"
        elif case == "empty":
            assert "signed in as vessel-test" in shown and "#" not in shown, "empty repository projection missing"
        else:
            assert f"#{item['number']} Open" in shown, f"open pull missing: {case}"
            assert f"head: feature/long-branch-for-checks-reviews-and-merge-state {item['head']}" in shown, "head does not match local git"
            assert "merge methods: merge, squash, rebase" in shown, "merge methods missing"
            assert "검사와 리뷰" in shown, "long Korean title missing"
            expected = {"failing": "checks Failing", "pending": "checks Pending", "conflicting": "Conflicting",
                        "changes-requested": "review ChangesRequested", "draft": "Open draft", "unknown": "Unknown"}
            assert expected.get(case, "checks Passing") in shown, f"PR state missing: {case}"
            if case == "failing":
                assert "1240 passed, 2 failed, 0 pending (more not read)" in shown and "한글 검사" in shown, "check facts missing"
            if case == "changes-requested":
                assert "approved false, changes requested true, 23 unresolved" in shown, "review facts missing"
            reader = success("pr", "show", session, "--refresh", readonly=True)
            assert f"#{item['number']} Open" in reader, "read-only PR read refused"
            refused = call("pr", "merge", session, "--number", str(item["number"]),
                           "--head", item["head"], "--method", "squash", readonly=True)
            assert refused.returncode != 0 and "Forbidden" in refused.stderr, "read-only merge allowed"
        if case == "merge-refused":
            refused = call("pr", "merge", session, "--number", str(item["number"]),
                           "--head", item["head"], "--method", "squash")
            assert refused.returncode != 0 and "the repository refused this merge" in refused.stderr, "configured merge refusal missing"
            assert f"#{item['number']} Open" in success("pr", "show", session, "--refresh"), "refused merge changed facts"
    (root / "forge-fixtures.json").write_text(json.dumps(manifest, indent=2) + "\n")
    with (root / "sessions").open("a") as out:
        out.write("\n".join(sessions) + "\n")
    listed = success("pr", "list")
    assert all(item["session"] in listed for item in manifest), "watched PR list lacks fixture sessions"
    (root / "forge-shapes.txt").write_text(
        "File-seeded FakeForge: ready/failing/pending/reviews/conflict/draft/unknown/empty checked\n"
        "No access, rate limit, refused merge, unchanged facts and read-only reads/refusals checked\n"
        "Real placed sessions, local git heads and CLI projection; no HTTP or real account used\n")


if __name__ == "__main__":
    try:
        action, location, binary_dir = sys.argv[1:]
        {"prepare": prepare, "check": check}[action](Path(location).resolve(), Path(binary_dir).resolve())
    except AssertionError as error:
        print(f"forge fixture check failed: {error}", file=sys.stderr)
        sys.exit(1)
    except subprocess.TimeoutExpired:
        print("forge fixture check failed: command timed out", file=sys.stderr)
        sys.exit(1)
    except (OSError, ValueError, KeyError, IndexError):
        print("forge fixture check failed: fixture I/O or format invalid", file=sys.stderr)
        sys.exit(1)
