#!/usr/bin/env bash
# A node behind a relay, both on this machine's loopback, for trying the
# phone app in the Android emulator as the owner's phone will be used: the
# node is reached only through the relay it registered with, and the app
# pairs with it by the single-use link `vessel node pair` prints.
#
#   test-fleet.sh start <bin-dir> <work-dir> [emulator-serial]
#   test-fleet.sh link  <bin-dir> <work-dir>     # a fresh link, into a file
#   test-fleet.sh stop  <work-dir>
#
# `adb` must be on PATH when an emulator serial is given.
#
# <bin-dir> holds `vessel`, `vesseld`, `fake-tui` and `fake-acp` from the
# main workspace (`cargo build -p vessel-cli -p vessel-node -p
# vessel-testkit --bins`). The node runs the testkit's fake harnesses under
# the real harnesses' names, with a temporary home: no real agent is
# started and nothing of the user's is read. With an emulator serial, the
# relay's port is forwarded into the emulator (`adb reverse`), so the app
# reaches it at the same loopback address the link names.
#
# The pairing link is a credential until used: it is written to
# <work-dir>/pair-link (mode 0600) and never printed.
set -euo pipefail

free_port() {
  python3 -I -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()'
}

start() {
  local bin=$1 work=$2 serial=${3:-}
  mkdir -p "$work" && chmod 700 "$work"
  local fakes="$work/bin" home="$work/home" data="$work/node"
  mkdir -p "$fakes" "$home"
  ln -sf "$bin/fake-tui" "$fakes/claude"
  ln -sf "$bin/fake-acp" "$fakes/claude-agent-acp"
  ln -sf "$bin/fake-tui" "$fakes/codex"
  ln -sf "$bin/fake-acp" "$fakes/codex-acp"
  local relay_port node_port
  relay_port=$(free_port)
  node_port=$(free_port)
  (umask 077; head -c 24 /dev/urandom | base64 | tr -d '/+=\n' > "$work/relay-token")
  local env=(env -i "PATH=$fakes:/usr/local/bin:/usr/bin:/bin" "HOME=$home" "RUST_LOG=info")
  "${env[@]}" "$bin/vesseld" relay --bind "127.0.0.1:$relay_port" \
    --token-file "$work/relay-token" > "$work/relay.log" 2>&1 &
  echo $! > "$work/relay.pid"
  "${env[@]}" "$bin/vesseld" --data-dir "$data" --bind "127.0.0.1:$node_port" \
    --name test-node --relay "ws://127.0.0.1:$relay_port" \
    --relay-token-file "$work/relay-token" > "$work/node.log" 2>&1 &
  echo $! > "$work/node.pid"
  for _ in $(seq 1 100); do [ -f "$data/token" ] && break; sleep 0.2; done
  echo "$relay_port" > "$work/relay-port"
  echo "$node_port" > "$work/node-port"
  if [ -n "$serial" ]; then
    adb -s "$serial" reverse "tcp:$relay_port" "tcp:$relay_port" > /dev/null
  fi
  echo "relay 127.0.0.1:$relay_port, node 127.0.0.1:$node_port (reached by the app only through the relay)"
}

link() {
  local bin=$1 work=$2
  local env=(env -i "PATH=$work/bin:/usr/local/bin:/usr/bin:/bin" "HOME=$work/home")
  (umask 077; "${env[@]}" "$bin/vessel" --data-dir "$work/node" \
    --node "ws://127.0.0.1:$(cat "$work/node-port")" node pair > "$work/pair-out" 2>&1)
  grep -o 'vessel://pair#[A-Za-z0-9_-]*' "$work/pair-out" | head -1 > "$work/pair-link"
  chmod 600 "$work/pair-link"
  rm -f "$work/pair-out"
  [ -s "$work/pair-link" ] && echo "a fresh link is in $work/pair-link"
}

stop() {
  local work=$1
  for name in node relay; do
    [ -f "$work/$name.pid" ] && kill "$(cat "$work/$name.pid")" 2>/dev/null || true
  done
}

case "${1:-}" in
  start) shift; start "$@" ;;
  link) shift; link "$@" ;;
  stop) shift; stop "$@" ;;
  *) echo "usage: $0 start|link|stop ..." >&2; exit 2 ;;
esac
