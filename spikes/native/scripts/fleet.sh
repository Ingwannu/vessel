#!/usr/bin/env bash
# A small fleet for trying the app: two real `vesseld` nodes in a scratch
# directory with the testkit's fake harnesses (`fake-acp` as the ACP
# adapters, `fake-tui` as the CLIs), a node book naming them, and a few
# structured sessions with a turn each. No real agent runs and nothing
# leaves the machine: every process has a scratch home, the nodes listen
# on loopback ports the system picks. The same setup as the page's
# end-to-end fleet (web/e2e/fleet.ts), in shell.
#
# Usage (from spikes/native):
#   scripts/fleet.sh start ROOT TARGET   # TARGET: the main workspace's
#                                        # build dir with debug binaries
#   scripts/fleet.sh stop ROOT
# Then run the app with VESSEL_CONFIG_DIR=ROOT/config. `laptop` also
# serves pairing (`--public`), so `ROOT/pair.sh` prints a pairing link.
#
# Build the binaries first, in the repository root:
#   cargo build -p vessel-cli -p vessel-node -p vessel-testkit --bins
set -euo pipefail

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

: > "$root/pids"
for name in laptop build-server; do
  port=$(free_port)
  data="$root/$name"
  home="$root/$name-home"
  mkdir -p "$home"
  extra=()
  if [ "$name" = laptop ]; then extra=(--public "127.0.0.1:$port"); fi
  env -i PATH="$base_path" HOME="$home" RUST_LOG=info \
    "$target/vesseld" --data-dir "$data" --bind "127.0.0.1:$port" --name "$name" "${extra[@]}" \
    > "$root/$name.log" 2>&1 &
  echo $! >> "$root/pids"
  # The token is written before the node listens: wait for both.
  for _ in $(seq 1 300); do
    [ -f "$data/token" ] && (exec 3<>"/dev/tcp/127.0.0.1/$port") 2>/dev/null && break
    sleep 0.1
  done
  echo "$name 127.0.0.1:$port" >> "$root/nodes"
  env -i PATH="$base_path" HOME="$root" VESSEL_CONFIG_DIR="$root/config" \
    "$target/vessel" nodes add "$name" "ws://127.0.0.1:$port" --token-file "$data/token"
done

cli() {
  env -i PATH="$base_path" HOME="$root" VESSEL_CONFIG_DIR="$root/config" "$target/vessel" "$@"
}

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
  grep -q '^(thinking) The failure is in the timeline' <<<"$log" || problems+=("the thought")
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

cat > "$root/pair.sh" <<PAIR
#!/usr/bin/env bash
# A pairing link for laptop (single use, ten minutes).
env -i PATH="$base_path" HOME="$root/laptop-home" "$target/vessel" --data-dir "$root/laptop" --node "\$(cut -d' ' -f2 < "$root/nodes" | head -1)" node pair "\$@"
PAIR
chmod +x "$root/pair.sh"
echo "fleet up: VESSEL_CONFIG_DIR=$root/config"
cat "$root/sessions"
