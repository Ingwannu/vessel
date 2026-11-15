#!/usr/bin/env bash
# A small fleet for trying the app: two real `vesseld` nodes in a scratch
# directory with the testkit's fake harnesses (`fake-acp` as the ACP
# adapters, `fake-tui` as the CLIs), a node book naming them, and a few
# structured sessions with a turn each. No real agent runs and nothing
# leaves the machine unless a person opens the public agent registry:
# every process has a scratch home, the nodes listen
# on loopback ports the system picks. The same setup as the page's
# end-to-end fleet (web/e2e/fleet.ts), in shell.
#
# Usage (from spikes/native):
#   scripts/fleet.sh start ROOT TARGET   # TARGET: the main workspace's
#                                        # build dir with debug binaries
#   scripts/fleet.sh stop ROOT
# Then run the app with VESSEL_CONFIG_DIR=ROOT/config. Every node serves
# pairing (`--public`), and the book holds what a person's pairing grants
# (read, operate, admin), never a node's owner token: each node is paired
# as a person pairs, its single-use link going from `vessel node pair` to
# `vessel nodes add NAME --pair --yes` through a pipe, never printed,
# logged or kept (the app track's rule of 2026-11-14). `ROOT/pair.sh`
# prints a link for laptop, for trying the pairing dialog by hand.
#
# Build the binaries first, in the repository root:
#   cargo build -p vessel-cli -p vessel-node -p vessel-testkit --bins
set -euo pipefail
umask 077
source "$(dirname "${BASH_SOURCE[0]}")/agent-fixtures.sh"

command=${1:?start or stop}
root=${2:?a scratch directory}

if [ "$command" = stop ]; then
  if [ -f "$root/pids" ]; then
    while read -r pid; do kill "$pid" 2>/dev/null || true; done < "$root/pids"
    rm -f "$root/pids"
  fi
  exit 0
fi

target=${3:?the debug binaries directory}
mkdir -p "$root/bin" "$root/config" "$root/work"
for name in claude codex; do ln -sf "$target/fake-tui" "$root/bin/$name"; done
# Claude Code's adapter offers what the real one sends (many models,
# subagents with descriptions of several lines, long and Korean names, an
# on/off option with no name: FAKE_ACP_OPTIONS=real) and writes the real
# one's start-up log before the first prompt (`[session/create] …`,
# FAKE_ACP_STARTUP_LOG); Codex's the fake's small default set.
rm -f "$root/bin/claude-agent-acp"
printf '#!/bin/sh\nFAKE_ACP_OPTIONS=real FAKE_ACP_STARTUP_LOG=1 exec "%s" "$@"\n' "$target/fake-acp" > "$root/bin/claude-agent-acp"
chmod 755 "$root/bin/claude-agent-acp"
ln -sf "$target/fake-acp" "$root/bin/codex-acp"
base_path="$root/bin:/usr/local/bin:/usr/bin:/bin"

free_port() {
  python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()'
}

python3 "$(dirname "$0")/settings-fixtures.py" prepare "$root"
python3 "$(dirname "$0")/wire-gaps-fixtures.py" prepare "$root" "$target"

: > "$root/pids"
for name in laptop build-server; do
  port=$(free_port)
  data="$root/$name"
  home="$root/$name-home"
  mkdir -p "$home"
  agent_fixture_bin "$root" "$target" "$name"
  agent_fixture_config "$data"
  node_path="$root/$name-bin:/usr/bin:/bin"
  env -i PATH="$node_path" HOME="$home" TZ=Asia/Seoul RUST_LOG=info \
    "$target/vesseld" --data-dir "$data" --bind "127.0.0.1:$port" --name "$name" \
    --public "127.0.0.1:$port" \
    > "$root/$name.log" 2>&1 &
  echo $! >> "$root/pids"
  # The token is written before the node listens: wait for both.
  for _ in $(seq 1 300); do
    [ -f "$data/token" ] && (exec 3<>"/dev/tcp/127.0.0.1/$port") 2>/dev/null && break
    sleep 0.1
  done
  echo "$name 127.0.0.1:$port" >> "$root/nodes"
  # The node's own token makes the link (on the node's side, as its owner
  # would); the book only ever receives the paired device's credentials.
  env -i PATH="$base_path" HOME="$home" \
    "$target/vessel" --data-dir "$data" --node "127.0.0.1:$port" node pair --label fleet-app \
    | env -i PATH="$base_path" HOME="$root" VESSEL_CONFIG_DIR="$root/config" \
      "$target/vessel" nodes add "$name" --pair --yes > /dev/null
done

python3 "$(dirname "$0")/settings-fixtures.py" seed "$root"

cli() {
  env -i PATH="$base_path" HOME="$root" VESSEL_CONFIG_DIR="$root/config" "$target/vessel" "$@"
}

# The schedule preview is real node behaviour, not a client calculation.
python3 "$(dirname "$0")/wire-gaps-fixtures.py" check "$root" "$target"
agent_fixture_seed "$root" "$target"

# A few structured sessions; the first prompt names each (a title is the
# first line of the first prompt), one of them in Hangul.
start_session() { # node folder first-prompt
  mkdir -p "$root/work/$2"
  local id
  id=$(cli --node "$1" run claude-code --structured --cwd "$root/work/$2" --detach | tail -1)
  cli --node "$1" send "$id" "$3" > /dev/null
  echo "$1 $id $2" >> "$root/sessions"
}
: > "$root/sessions"
start_session laptop vessel-client "entries 40"
start_session laptop 한글-세션 "plain"
start_session build-server ci-fixes "entries 12"
start_session build-server docs "wait"

# Real conversation shapes (fake-acp's `shapes`; the real agent's options
# are above): a long thought; Markdown with a table, code wider than any
# window and Korean; a read of a long path; an edit with a wide diff; a
# command on one long line asking for permission with four answers, and
# once allowed 2,400 lines of output. Two copies: `shapes` answered (the
# output in the timeline), `shapes-asking` left asking (the permission
# card with the long command). Screens are checked against these, so the
# sessions are checked to really hold them, through what `vessel log`
# prints; a tool's output is not printed there, and its line count is
# held by the verb's own tests (crates/vessel-testkit/src/bin/fake_acp/
# shapes.rs). Older fake-acp binaries echo `shapes` instead: rebuild.
log_of() { # node id
  cli --node "$1" log "$2" 2>&1
}
wait_for() { # node id text
  for _ in $(seq 1 300); do
    if log_of "$1" "$2" | grep -qF -- "$3"; then return 0; fi
    sleep 0.1
  done
  echo "fleet: session $2 never showed \"$3\" (is fake-acp built with \`shapes\`?)" >&2
  return 1
}
check_shapes() { # node id answered|asking
  local log problems=()
  log=$(log_of "$1" "$2")
  # The adapter's start-up log: its ten lines, all before the first
  # prompt (a real Claude Code's; FAKE_ACP_STARTUP_LOG above).
  local startup
  startup=$(sed -n '/^> shapes$/q; p' <<<"$log" | grep -c '^\[harness\] \[session/' || true)
  [ "$startup" -eq 10 ] || problems+=("the start-up log's ten lines before the prompt (got: $startup)")
  # The entries: the thought, the message's table (header, rule, four
  # rows), three tool calls, the permission with four answers.
  # The thought, streamed a few characters at a time (one syllable
  # alone among them) and put back together from `vessel log`, which
  # prints each chunk as a `(thinking) …` line and a chunk's own line
  # breaks as lines without it, up to the message's heading: many chunks,
  # its Korean sentence and English whole, and no Hangul letter loose from
  # its syllable (the owner's "생가ㄱ").
  local thought
  thought=$(python3 -c '
import sys
chunks, text, inside = 0, "", False
for line in sys.stdin.read().split("\n"):
    if line.startswith("(thinking) "):
        inside, chunks, text = True, chunks + 1, text + line[len("(thinking) "):]
    elif inside and line.startswith("## "):
        break
    elif inside:
        text += "\n" + line
loose = any("\u3131" <= c <= "\u318e" or "\u1100" <= c <= "\u11ff" for c in text)
whole = "생각을 정리하면, 묶음의 끝이 메시지 안에 걸릴 때 두 번째 조각이 빠집니다." in text
english = "The failure is in the timeline" in text
print("ok" if chunks > 100 and whole and english and not loose else
      f"chunks={chunks} korean={whole} english={english} loose={loose}")
' <<<"$log")
  [ "$thought" = ok ] || problems+=("the thought streamed whole ($thought)")
  [ "$(grep -c '^| ' <<<"$log")" -eq 6 ] || problems+=("the table's six lines")
  [ "$(grep -c '^\[tool\] ' <<<"$log")" -eq 3 ] || problems+=("three tool calls")
  grep -q '^  4) Never allow commands' <<<"$log" || problems+=("the permission's four answers")
  # The longest line: the code line, 240 characters (ASCII, so bytes).
  local widest
  widest=$(LC_ALL=C awk '/^let a_name_long_enough/ { print length($0) }' <<<"$log")
  [ "$widest" = 240 ] || problems+=("the 240-character code line (got: ${widest:-none})")
  if [ "$3" = answered ]; then
    grep -q '^\[tool shapes-run\] done' <<<"$log" || problems+=("the command done")
    grep -q '테스트를 모두 돌렸습니다' <<<"$log" || problems+=("the closing Korean message")
  else
    ! grep -q '^\[tool shapes-run\] ' <<<"$log" || problems+=("the command still asking")
  fi
  if [ ${#problems[@]} -gt 0 ]; then
    printf 'fleet: session %s lacks:' "$2" >&2
    printf ' %s;' "${problems[@]}" >&2
    echo >&2
    return 1
  fi
}
start_session laptop shapes "shapes"
shapes=$(tail -1 "$root/sessions" | cut -d' ' -f2)
start_session laptop shapes-asking "shapes"
asking=$(tail -1 "$root/sessions" | cut -d' ' -f2)
wait_for laptop "$shapes" "[permission]"
wait_for laptop "$asking" "[permission]"
# `vessel log` does not print a request's id; the one request waiting is
# named by the hex digit its id starts with.
for digit in 0 1 2 3 4 5 6 7 8 9 a b c d e f; do
  cli --node laptop session answer "$shapes" "$digit" allow-once 2> /dev/null && break
done
wait_for laptop "$shapes" "테스트를 모두 돌렸습니다"
check_shapes laptop "$shapes" answered
check_shapes laptop "$asking" asking

# A conversation of several turns (the outline of prompts, read marks):
# five prompts, one long and one Korean. The fake answers a prompt it
# has no verb for with an echo and a tool that asks for permission;
# each is allowed, so every turn ends.
answer_waiting() { # node id: allow the one request waiting
  for digit in 0 1 2 3 4 5 6 7 8 9 a b c d e f; do
    cli --node "$1" session answer "$2" "$digit" allow-once 2> /dev/null && return 0
  done
  return 1
}
start_session laptop turns "plain"
turns=$(tail -1 "$root/sessions" | cut -d' ' -f2)
ended=1
wait_for laptop "$turns" "[turn ended"
for prompt in \
  "Read the timeline's fold and tell me where a batch that ends inside a message loses its last chunk, with the lines that do it and what you would change first" \
  "긴 대화에서 스크롤이 끊기지 않는지 확인해 주세요" \
  "Now run the tests again" \
  "plain"; do
  # The node may still be closing the turn just ended: try again.
  for _ in $(seq 1 50); do
    cli --node laptop send "$turns" "$prompt" > /dev/null 2>&1 && break
    sleep 0.1
  done
  if [ "$prompt" != plain ]; then
    for _ in $(seq 1 300); do
      [ "$(log_of laptop "$turns" | grep -c '^\[permission\]')" -ge "$ended" ] && break
      sleep 0.1
    done
    for _ in $(seq 1 50); do answer_waiting laptop "$turns" && break; sleep 0.1; done
  fi
  # Each prompt after the turn before it ended, so it is a turn of its
  # own and not a message steered into the running one.
  ended=$((ended + 1))
  for _ in $(seq 1 300); do
    [ "$(log_of laptop "$turns" | grep -c '^\[turn ended')" -ge "$ended" ] && break
    sleep 0.1
  done
done
[ "$(log_of laptop "$turns" | grep -c '^> ')" -eq 5 ] || {
  echo "fleet: session $turns lacks its five prompts" >&2
  exit 1
}

# The workspace's shapes (track F): a monorepo on laptop whose paths are
# long, whose working tree has well over a hundred changes (modified,
# staged, untracked, deleted, renamed, a Hangul name) and one diff of
# thousands of lines, with a vessel.toml naming setup, teardown, scripts
# and services (not approved: the approval shows every one); a small
# project whose commands are approved; a worktree on a long branch name
# with uncommitted work (its removal lists what would be lost); and a
# clone that failed. Checked below, so a screen captured against the
# fleet is known to show them.
git_in() { git -C "$1" -c user.name=fleet -c user.email=fleet@example.invalid "${@:2}"; }
mono="$root/work/atlas-design-system-monorepo"
deep="packages/design-system-foundations/src/components/navigation/sidebar-with-collapsible-sections"
mkdir -p "$mono/$deep" "$mono/packages/tokens/generated" "$mono/docs/한글-문서"
git_in "$mono" init -q -b main
cat > "$mono/vessel.toml" <<'TOML'
[worktree]
setup = ["pnpm install --frozen-lockfile --prefer-offline", "pnpm --filter @atlas/design-system-foundations run build:tokens", "cp \"$VESSEL_PROJECT_ROOT/.env.example\" .env"]
teardown = "docker compose --project-name atlas-preview down --volumes --remove-orphans"

[scripts.lint]
command = "pnpm run lint -- --max-warnings=0"

[scripts.typecheck]
command = "pnpm -r run typecheck"

[services.storybook]
command = "pnpm --filter @atlas/design-system-foundations run storybook -- --port 6006 --no-open"
dir = "packages/design-system-foundations"

[services.dev]
command = "pnpm run dev"
TOML
for n in $(seq 1 80); do
  printf 'export const item%s = %s;\n' "$n" "$n" > "$mono/$deep/collapsible-section-item-with-long-name-$n.tsx"
done
seq 1 3000 | sed 's/.*/export const colour& = "#&";/' > "$mono/packages/tokens/generated/generated-colour-scale-for-every-theme.ts"
printf '# 설치 안내\n\n한글로 쓴 문서입니다.\n' > "$mono/docs/한글-문서/설치-안내.md"
printf 'old name\n' > "$mono/docs/renamed-before.md"
mkdir -p "$mono/assets"
head -c 4096 /dev/urandom > "$mono/assets/brand-mark.bin"
git_in "$mono" add -A && git_in "$mono" commit -q -m "The design system, as it was"
# The working tree: 60 modified (20 of them staged), 40 untracked, 5
# deleted, one renamed, the Hangul file edited, 2,500 lines rewritten.
for n in $(seq 1 60); do echo "export const changed$n = true;" >> "$mono/$deep/collapsible-section-item-with-long-name-$n.tsx"; done
for n in $(seq 1 20); do git_in "$mono" add "$deep/collapsible-section-item-with-long-name-$n.tsx"; done
for n in $(seq 1 40); do printf 'new %s\n' "$n" > "$mono/$deep/untracked-generated-snapshot-$n.snap"; done
for n in $(seq 76 80); do rm "$mono/$deep/collapsible-section-item-with-long-name-$n.tsx"; done
git_in "$mono" mv docs/renamed-before.md docs/renamed-after-a-much-longer-name.md
printf '\n바뀐 줄입니다.\n' >> "$mono/docs/한글-문서/설치-안내.md"
# A binary file changed, a folder with nothing in it (the Files tree's
# empty folder), and a file no one may read (a failed read).
head -c 4096 /dev/urandom > "$mono/assets/brand-mark.bin"
mkdir -p "$mono/packages/empty-folder"
printf 'not for reading\n' > "$mono/unreadable-notes.txt"
chmod 000 "$mono/unreadable-notes.txt"
sed -i '1,2500s/"#/"oklch(/' "$mono/packages/tokens/generated/generated-colour-scale-for-every-theme.ts"
cli --node laptop project add "$mono" > /dev/null
small="$root/work/shop"
mkdir -p "$small" && git_in "$small" init -q -b main
printf '[worktree]\nsetup = "npm ci"\n\n[scripts.test]\ncommand = "npm test"\n' > "$small/vessel.toml"
printf 'console.log("shop");\n' > "$small/index.js"
git_in "$small" add -A && git_in "$small" commit -q -m init
cli --node laptop project add "$small" > /dev/null
digest=$(cli --node laptop project commands shop 2> /dev/null | grep -oE '[0-9a-f]{64}' | head -1)
[ -n "$digest" ] && cli --node laptop project approve shop "$digest" > /dev/null 2>&1
cli --node laptop worktree new atlas-design-system-monorepo --skip-setup \
  --branch feature/sidebar-collapsible-sections-keyboard-navigation-and-focus-order > /dev/null 2>&1 || true
tree=$(cli --node laptop worktree list 2> /dev/null | awk '/sidebar-collapsible/ {print $NF; exit}')
[ -n "$tree" ] && [ -d "$tree" ] && printf 'work in progress\n' > "$tree/README-draft.md"
cli --node laptop project clone "https://127.0.0.1:9/team/unreachable-repository.git" "$root/work" > /dev/null 2>&1 || true

problems=()
changes=$(git -C "$mono" status --porcelain | wc -l)
[ "$changes" -ge 100 ] || problems+=("only $changes changes in the monorepo")
big=$(git -C "$mono" diff --numstat -- packages/tokens | awk '{print $1}')
[ "${big:-0}" -ge 2000 ] || problems+=("the large diff has ${big:-0} lines")
listed=$(cli --node laptop project list 2> /dev/null)
grep -q 'atlas-design-system-monorepo' <<<"$listed" || problems+=("the monorepo is not a project")
grep -q 'clone_failed\|clone failed' <<<"$listed" || problems+=("no failed clone listed")
[ -n "$digest" ] || problems+=("shop's commands were not approved")
[ -n "$tree" ] || problems+=("no worktree on the long branch")
git -C "$mono" diff --numstat -- assets | grep -q '^-' || problems+=("no binary change")
[ -d "$mono/packages/empty-folder" ] && [ -z "$(ls -A "$mono/packages/empty-folder")" ] || problems+=("no empty folder")
[ -e "$mono/unreadable-notes.txt" ] && ! [ -r "$mono/unreadable-notes.txt" ] || problems+=("no unreadable file")
if [ ${#problems[@]} -gt 0 ]; then
  printf 'fleet: the workspace shapes are missing: %s\n' "${problems[*]}" >&2
  exit 1
fi

# Questions the agent asks, as Claude's adapter sends its
# `AskUserQuestion` (fake-acp's `ask questions` and `ask one`): three
# questions (one choosing several, one in Korean, each with its "Other"
# box) and one. Both are left waiting, and checked through `vessel log`.
start_session laptop questions "ask questions"
questions=$(tail -1 "$root/sessions" | cut -d' ' -f2)
start_session laptop question "ask one"
question=$(tail -1 "$root/sessions" | cut -d' ' -f2)
wait_for laptop "$questions" "[question "
wait_for laptop "$question" "[question "
asked=$(log_of laptop "$questions")
grep -q '^\[question [^]]*\] Please answer the following questions\.$' <<<"$asked" &&
  [ "$(grep -c '^  question_[0-9]_custom: Other (text)$' <<<"$asked")" -eq 3 ] &&
  grep -q '^  question_1: Checks (choose, several)$' <<<"$asked" &&
  grep -q '^  question_2: 언어 (choose)$' <<<"$asked" || {
  echo "fleet: session $questions lacks Claude's three questions" >&2
  exit 1
}
grep -q '^\[question [^]]*\] Which approach should the fold take' <<<"$(log_of laptop "$question")" || {
  echo "fleet: session $question lacks Claude's one question" >&2
  exit 1
}

# Permission requests in the real adapters' shapes (fake-acp's
# `permit plan`, `permit codex`, `permit two`): Claude Code leaving plan
# mode with its plan and seven answers, Codex's five answers for a
# command, two commands asking at once. All are left waiting, and checked
# through `vessel log` (one `[permission]` line per request).
start_session laptop permit-plan "permit plan"
plan=$(tail -1 "$root/sessions" | cut -d' ' -f2)
start_session laptop permit-codex "permit codex"
codex=$(tail -1 "$root/sessions" | cut -d' ' -f2)
start_session laptop permit-two "permit two"
two=$(tail -1 "$root/sessions" | cut -d' ' -f2)
wait_for laptop "$plan" "[permission]"
wait_for laptop "$codex" "[permission]"
for _ in $(seq 1 300); do
  [ "$(log_of laptop "$two" | grep -c '^\[permission\]')" -ge 2 ] && break
  sleep 0.1
done
grep -q '^  7) No, keep planning' <<<"$(log_of laptop "$plan")" &&
  grep -q "^  5) No, and tell Codex what to do differently" <<<"$(log_of laptop "$codex")" &&
  [ "$(log_of laptop "$two" | grep -c '^\[permission\]')" -eq 2 ] || {
  echo "fleet: the permission sessions lack their real shapes" >&2
  exit 1
}

# How full the agent's context is (fake-acp's `usage <tokens>`, of
# 200,000, at a cost of 0.5 USD): the context meter's room (0.6%), amber
# (75%) and red (95%). The agent offers `/compact`. Checked through
# `vessel log` (the turn's message chunk).
for shape in "context-room usage" "context-low usage 150000" "context-full usage 190000"; do
  start_session laptop "${shape%% *}" "${shape#* }"
  wait_for laptop "$(tail -1 "$root/sessions" | cut -d' ' -f2)" "used"
done
# The same on a session that has ended (stopped once its turn said its
# usage): the meter card's "Compact context" waits, saying why, and the
# ended banner shows over the composer. Checked through `vessel log`
# (the turn's chunk, then the stopped session's last status).
start_session laptop context-ended "usage 120000"
stopped=$(tail -1 "$root/sessions" | cut -d' ' -f2)
wait_for laptop "$stopped" "[turn ended"
cli --node laptop kill "$stopped" > /dev/null
wait_for laptop "$stopped" "[session exited"

# An agent's plan as it changes (fake-acp's `plans`, Claude Code's shape:
# the whole list in each update): created, sent again unchanged, a step
# started, one done as the next starts, one added. Checked through
# `vessel log` (one `[plan]` line per update, six).
start_session laptop plans "plans"
plans=$(tail -1 "$root/sessions" | cut -d' ' -f2)
wait_for laptop "$plans" "plans done"
[ "$(log_of laptop "$plans" | grep -c '^\[plan\]')" -eq 6 ] || {
  echo "fleet: the plans session lacks its six plan updates" >&2
  exit 1
}

# Prompts waiting their turn (the queue above the composer): a turn that
# runs until it is stopped (`wait`, which also takes a steer) and three
# prompts queued behind it, one long, one Korean, one short. Checked
# through `vessel session queue`, one line per entry after its heading.
start_session laptop queued "wait"
queued=$(tail -1 "$root/sessions" | cut -d' ' -f2)
wait_for laptop "$queued" "> wait"
for prompt in \
  "After this, read the fold again and write down every place a batch that ends inside a message can lose a chunk, with the lines that do it" \
  "끝나면 한글 세션의 제목도 같이 바꿔 주세요" \
  "Then run the tests"; do
  for _ in $(seq 1 50); do
    cli --node laptop send "$queued" "$prompt" --queue > /dev/null 2>&1 && break
    sleep 0.1
  done
done
waiting=$(cli --node laptop session queue "$queued" | grep -cE '^ *[0-9]+  ' || true)
[ "$waiting" -eq 3 ] || {
  echo "fleet: session $queued lacks its three queued prompts (got: $waiting)" >&2
  exit 1
}

python3 "$(dirname "$0")/shell-fixtures.py" seed "$root" "$target"

cat > "$root/pair.sh" <<PAIR
#!/usr/bin/env bash
# A pairing link for laptop (single use, ten minutes).
env -i PATH="$base_path" HOME="$root/laptop-home" "$target/vessel" --data-dir "$root/laptop" --node "\$(cut -d' ' -f2 < "$root/nodes" | head -1)" node pair "\$@"
PAIR
chmod +x "$root/pair.sh"
echo "fleet up: VESSEL_CONFIG_DIR=$root/config"
cat "$root/sessions"
