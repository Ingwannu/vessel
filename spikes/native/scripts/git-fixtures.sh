#!/usr/bin/env bash
# The git toolbar's shapes on a running fake fleet (track G1): one project
# per state the toolbar draws differently, each with a structured session
# in it so the code panel opens on its repository, and a device paired as
# a person pairs one (not with the node's owner token).
#
# Usage (from spikes/native, after `scripts/fleet.sh start ROOT TARGET`):
#   bash scripts/git-fixtures.sh ROOT TARGET
# Then run the app with HOME=ROOT and VESSEL_CONFIG_DIR=ROOT/person-config
# (operate scope) or ROOT/readonly-config (read only, made by the fleet
# itself): their books name the credential files relative to that home.
#
# Every remote is a bare repository under ROOT/remotes: nothing leaves the
# machine. Git runs with no global or system configuration. The pairing
# link goes from the node straight into the CLI's standard input and is
# never printed or written anywhere.
set -euo pipefail
umask 077

root=${1:?the fleet scratch directory}
target=${2:?the debug binaries directory}
base_path="$root/bin:/usr/local/bin:/usr/bin:/bin"

cli() {
  env -i PATH="$base_path" HOME="$root" VESSEL_CONFIG_DIR="$root/config" "$target/vessel" "$@"
}
g() {
  env -i PATH=/usr/bin:/bin HOME="$root" GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1 \
    git -C "$1" -c user.name="Fixture Author" -c user.email=fixture@example.invalid \
    -c init.defaultBranch=main "${@:2}"
}

work="$root/work/git"
remotes="$root/remotes"
mkdir -p "$work" "$remotes"

# A project and a session in it; the session's title names the state.
project_session() { # folder title
  cli --node laptop project add "$1" > /dev/null
  local id
  id=$(cli --node laptop run claude-code --structured --project "$(basename "$1")" --detach | tail -1)
  cli --node laptop send "$id" "$2" > /dev/null
  echo "laptop $id $2" >> "$root/git-sessions"
}

# A repository whose origin has `main` (its default, as `origin/HEAD`
# names it) and, when `branch` is another, that branch made from main and
# pushed with an upstream; one commit each.
with_origin() { # name branch
  local repo="$work/$1" bare="$remotes/$1.git"
  g "$remotes" init -q --bare "$bare"
  mkdir -p "$repo"
  g "$repo" init -q -b main
  printf '# %s\n' "$1" > "$repo/README.md"
  g "$repo" add -A
  g "$repo" commit -q -m "First commit"
  g "$repo" remote add origin "$bare"
  g "$repo" push -q -u origin main
  g "$repo" remote set-head origin main
  if [ "$2" != main ]; then
    g "$repo" switch -q -c "$2"
    g "$repo" push -q -u origin "$2"
  fi
  # The node's git runs in a scratch home with no identity: commits there
  # take this repository's own. `no-commits-yet` keeps none, so its first
  # commit shows the node's refusal.
  g "$repo" config user.name "Fixture Author"
  g "$repo" config user.email fixture@example.invalid
}

: > "$root/git-sessions"
long_branch="feature/keyboard-navigation-for-collapsible-sections-and-focus-order-레이아웃"

# 1. Commit is what makes sense: changes of every kind, Korean and long
# paths, staged and not, with an upstream one commit behind the branch;
# two saved entries (one long, one Korean).
with_origin commit-ready-checkout-for-the-design-system "$long_branch"
repo="$work/commit-ready-checkout-for-the-design-system"
deep="packages/design-system-foundations/src/components/navigation/sidebar-with-collapsible-sections"
mkdir -p "$repo/$deep" "$repo/docs/한글-문서"
for n in $(seq 1 40); do printf 'export const item%s = %s;\n' "$n" "$n" > "$repo/$deep/section-item-with-a-long-name-$n.tsx"; done
printf '# 설치 안내\n' > "$repo/docs/한글-문서/설치-안내.md"
g "$repo" add -A
g "$repo" commit -q -m "Sidebar sections"
printf 'saved edit\n' >> "$repo/README.md"
g "$repo" stash push -q -m "Half-done keyboard navigation for the sidebar's collapsible sections, kept while reviewing the focus order on narrow windows"
printf '저장한 수정\n' >> "$repo/README.md"
g "$repo" stash push -q -m "한글 메모: 접기 단추의 초점 순서 확인 중"
for n in $(seq 1 30); do echo "export const changed$n = true;" >> "$repo/$deep/section-item-with-a-long-name-$n.tsx"; done
for n in $(seq 1 10); do g "$repo" add "$deep/section-item-with-a-long-name-$n.tsx"; done
for n in $(seq 1 12); do printf 'new %s\n' "$n" > "$repo/$deep/untracked-snapshot-$n.snap"; done
rm "$repo/$deep/section-item-with-a-long-name-40.tsx"
printf '\n바뀐 줄입니다.\n' >> "$repo/docs/한글-문서/설치-안내.md"
project_session "$repo" "git commit ready"

# 2. Push: clean, two commits ahead of its upstream.
with_origin push-ahead-of-origin main
repo="$work/push-ahead-of-origin"
for n in 1 2; do printf 'line %s\n' "$n" >> "$repo/README.md"; g "$repo" commit -q -am "Local commit $n"; done
project_session "$repo" "git push ready"

# 3. Pull: clean, three commits behind (pushed from another clone).
with_origin pull-behind-origin main
other="$work/.pull-behind-other-clone"
g "$work" clone -q "$remotes/pull-behind-origin.git" "$other"
for n in 1 2 3; do printf 'theirs %s\n' "$n" >> "$other/README.md"; g "$other" commit -q -am "Their commit $n"; done
g "$other" push -q origin main
g "$work/pull-behind-origin" fetch -q origin
project_session "$work/pull-behind-origin" "git pull ready"

# 4. A branch never pushed: "Push and set upstream".
with_origin no-upstream-yet main
g "$work/no-upstream-yet" switch -q -c topic/never-pushed
printf 'topic\n' >> "$work/no-upstream-yet/README.md"
g "$work/no-upstream-yet" commit -q -am "Topic work"
project_session "$work/no-upstream-yet" "git no upstream"

# 5. In step with its upstream and clean: no primary action.
with_origin in-step-and-clean main
project_session "$work/in-step-and-clean" "git in step"

# 6. The remote refuses the push (a pre-receive hook, as a protected
# branch does).
with_origin push-refused-by-remote main
printf '#!/bin/sh\necho "protected branch: pushes to main are not allowed" >&2\nexit 1\n' \
  > "$remotes/push-refused-by-remote.git/hooks/pre-receive"
chmod 755 "$remotes/push-refused-by-remote.git/hooks/pre-receive"
printf 'refused\n' >> "$work/push-refused-by-remote/README.md"
g "$work/push-refused-by-remote" commit -q -am "A commit the remote refuses"
project_session "$work/push-refused-by-remote" "git push refused"

# 7a. A change to commit on the default branch itself (the commit form
# warns there, and only there).
with_origin commit-on-the-default-branch main
printf 'edited on main\n' >> "$work/commit-on-the-default-branch/README.md"
project_session "$work/commit-on-the-default-branch" "git commit on main"

# 7. No commit yet, one new file.
mkdir -p "$work/no-commits-yet"
g "$work/no-commits-yet" init -q -b main
printf 'first\n' > "$work/no-commits-yet/first.txt"
project_session "$work/no-commits-yet" "git no commits"

# 8. Detached at a commit.
with_origin detached-head main
printf 'two\n' >> "$work/detached-head/README.md"
g "$work/detached-head" commit -q -am "Second"
g "$work/detached-head" switch -q --detach HEAD~1
project_session "$work/detached-head" "git detached"

# A device paired as a person pairs one, with the default scopes.
rm -rf "$root/person-config"
mkdir -p "$root/person-config"
env -i PATH="$base_path" HOME="$root/laptop-home" "$target/vessel" --data-dir "$root/laptop" \
  --node "$(cut -d' ' -f2 < "$root/nodes" | head -1)" node pair --label "Person's laptop app" 2> /dev/null \
  | env -i PATH="$base_path" HOME="$root" VESSEL_CONFIG_DIR="$root/person-config" \
    "$target/vessel" nodes add laptop --pair --yes > /dev/null 2>&1

problems=()
status_of() { g "$work/$1" status --porcelain --branch | head -1; }
[ "$(g "$work/commit-ready-checkout-for-the-design-system" status --porcelain | wc -l)" -ge 40 ] \
  || problems+=("commit-ready has too few changes")
[ "$(g "$work/commit-ready-checkout-for-the-design-system" stash list | wc -l)" -eq 2 ] \
  || problems+=("commit-ready lacks its two saved entries")
grep -q 'ahead 2' <<<"$(status_of push-ahead-of-origin)" || problems+=("push is not two ahead")
grep -q 'behind 3' <<<"$(status_of pull-behind-origin)" || problems+=("pull is not three behind")
! grep -q '\.\.\.' <<<"$(status_of no-upstream-yet)" || problems+=("no-upstream has an upstream")
[ "$(status_of in-step-and-clean)" = "## main...origin/main" ] || problems+=("in-step is not in step")
grep -q 'No commits yet' <<<"$(status_of no-commits-yet)" || problems+=("no-commits has a commit")
grep -q 'HEAD (no branch)' <<<"$(status_of detached-head)" || problems+=("detached is on a branch")
[ "$(g "$work/commit-ready-checkout-for-the-design-system" symbolic-ref --short refs/remotes/origin/HEAD)" = origin/main ] \
  || problems+=("commit-ready's default is not main")
[ "$(wc -l < "$root/git-sessions")" -eq 9 ] || problems+=("not every state has its session")
env -i PATH="$base_path" HOME="$root" VESSEL_CONFIG_DIR="$root/person-config" "$target/vessel" \
  --node laptop status > /dev/null || problems+=("the person's device was not paired")
[ -d "$root/readonly-config" ] || problems+=("the fleet's read-only device is missing")
if [ ${#problems[@]} -gt 0 ]; then
  printf 'git fixtures: missing: %s\n' "${problems[*]}" >&2
  exit 1
fi
echo "git fixtures up: VESSEL_CONFIG_DIR=$root/person-config or $root/readonly-config"
cat "$root/git-sessions"
