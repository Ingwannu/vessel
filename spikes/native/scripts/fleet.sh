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
for name in claude-agent-acp codex-acp; do ln -sf "$target/fake-acp" "$root/bin/$name"; done
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

cat > "$root/pair.sh" <<PAIR
#!/usr/bin/env bash
# A pairing link for laptop (single use, ten minutes).
env -i PATH="$base_path" HOME="$root/laptop-home" "$target/vessel" --data-dir "$root/laptop" --node "\$(cut -d' ' -f2 < "$root/nodes" | head -1)" node pair "\$@"
PAIR
chmod +x "$root/pair.sh"
echo "fleet up: VESSEL_CONFIG_DIR=$root/config"
cat "$root/sessions"
