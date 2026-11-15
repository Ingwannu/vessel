#!/usr/bin/env bash
# Show that a built Linux app reads the feed address written into its slot
# (ADR 0099), by running it.
#
#   tools/feed-slot-proof.sh test APP CHANNEL
#   tools/feed-slot-proof.sh real APP CHANNEL ADDRESS_FILE
#
# test  APP is the compile step's executable (slot unwritten). A copy gets
#       the address of a feed served here, on 127.0.0.1, for this run only;
#       the copy runs, and passes when it asks that feed for CHANNEL.json.
#       Run before the real address is written anywhere.
# real  APP holds the real address. A copy runs and passes when its log
#       says the check of the real feed ended: the manifest was fetched and
#       its signature checked with the key compiled in ("up to date": the
#       feed's newest is the label before this one). ADDRESS_FILE is read
#       only to make sure the log never holds the address's path, whose
#       last part is the token; neither is printed.
#
# Why run the app and not read its bytes: the slot's marker is an immutable
# static, and only running the optimised build shows the read was not
# folded into the code (updater/src/config.rs, FEED_SLOT_LEN). Why Linux
# only: the Windows app stops under wine at its first text (DirectWrite),
# and both are compiled from the same source by the same compiler.
#
# Everything runs from a private scratch directory: a copy of APP (so an
# update could never touch the set being packaged), an X server of its own
# with an auth cookie, the app's config and state, and the test feed; the
# processes started here are stopped by their PIDs and the directory is
# removed. Needs Xvfb, xauth, mcookie and python3.
set -euo pipefail

usage() {
    sed -n '5,6p' "$0" >&2
    exit 2
}
[ $# -ge 3 ] || usage
mode="$1" app="$2" channel="$3"
case "$mode" in
    test) [ $# -eq 3 ] || usage ;;
    real) [ $# -eq 4 ] || usage; address_file="$4" ;;
    *) usage ;;
esac
[[ "$channel" =~ ^[a-z][a-z0-9-]{0,31}$ ]] || { echo "not a channel: $channel" >&2; exit 2; }
[ -f "$app" ] || { echo "no $app" >&2; exit 2; }
here="$(cd "$(dirname "$0")" && pwd)"

umask 077
scratch="$(mktemp -d)"
app_pid="" xvfb_pid="" server_pid=""
stop() {
    # Only what this script started: the app's own process group (setsid),
    # the X server, the test feed.
    [ -z "$app_pid" ] || kill -- "-$app_pid" 2>/dev/null || true
    [ -z "$xvfb_pid" ] || kill "$xvfb_pid" 2>/dev/null || true
    [ -z "$server_pid" ] || kill "$server_pid" 2>/dev/null || true
    wait 2>/dev/null || true
    rm -rf "$scratch"
}
trap stop EXIT

mkdir -p "$scratch/app" "$scratch/config" "$scratch/state" "$scratch/data" "$scratch/cache"
copy="$scratch/app/vessel-app"

if [ "$mode" = test ]; then
    # The test feed: answers every request with 404 and writes its path to
    # a log; the port is the system's choice, written to a file.
    python3 -I -c '
import http.server, sys
port_file, log_file = sys.argv[1], sys.argv[2]
class Feed(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        with open(log_file, "a") as log:
            log.write(f"GET {self.path}\n")
        self.send_error(404)
    def log_message(self, *args):
        pass
server = http.server.HTTPServer(("127.0.0.1", 0), Feed)
with open(port_file + ".part", "w") as out:
    out.write(str(server.server_port))
import os
os.rename(port_file + ".part", port_file)
server.serve_forever()
' "$scratch/port" "$scratch/requests" >"$scratch/server.log" 2>&1 </dev/null &
    server_pid=$!
    for _ in $(seq 1 50); do
        [ -s "$scratch/port" ] && break
        sleep 0.1
    done
    [ -s "$scratch/port" ] || { echo "the test feed did not start" >&2; exit 1; }
    nonce="proof-$(od -An -N8 -tx1 /dev/urandom | tr -d ' \n')"
    printf 'http://127.0.0.1:%s/%s/\n' "$(cat "$scratch/port")" "$nonce" >"$scratch/address"
    python3 -I "$here/patch-feed.py" write --address-file "$scratch/address" "$app" "$copy" >/dev/null
    chmod 0755 "$copy"
else
    install -m 0755 "$app" "$copy"
fi

# An X server of our own: the first display number nobody holds.
display=""
for number in $(seq 140 199); do
    if [ ! -e "/tmp/.X$number-lock" ] && [ ! -e "/tmp/.X11-unix/X$number" ]; then
        display=":$number"
        break
    fi
done
[ -n "$display" ] || { echo "no free X display number in 140-199" >&2; exit 1; }
xauth -f "$scratch/xauth" add "$display" . "$(mcookie)" 2>/dev/null
setsid Xvfb "$display" -auth "$scratch/xauth" -screen 0 1280x800x24 -nolisten tcp >"$scratch/xvfb.log" 2>&1 </dev/null &
xvfb_pid=$!
for _ in $(seq 1 50); do
    [ -e "/tmp/.X11-unix/X${display#:}" ] && break
    sleep 0.1
done

env -u VESSEL_APP_NO_UPDATE DISPLAY="$display" XAUTHORITY="$scratch/xauth" \
    XDG_CONFIG_HOME="$scratch/config" XDG_STATE_HOME="$scratch/state" \
    XDG_DATA_HOME="$scratch/data" XDG_CACHE_HOME="$scratch/cache" \
    setsid "$copy" >"$scratch/run.log" 2>&1 </dev/null &
app_pid=$!
started=$SECONDS

log="$scratch/state/vessel/logs/vessel.log"
proven=0
while [ $((SECONDS - started)) -lt 60 ]; do
    sleep 1
    kill -0 "$app_pid" 2>/dev/null || { echo "the app stopped after $((SECONDS - started)) s" >&2; break; }
    if [ "$mode" = test ]; then
        if [ -f "$scratch/requests" ] && grep -qxF "GET /$nonce/$channel.json" "$scratch/requests"; then
            echo "feed slot: the compiled app asked a test feed written into its slot for $channel.json after $((SECONDS - started)) s"
            proven=1
            break
        fi
    elif [ -f "$log" ] && grep -q "up to date: the feed's newest is not newer" "$log"; then
        echo "feed slot: the shipped app checked the real feed (signed manifest read, nothing newer) after $((SECONDS - started)) s"
        proven=1
        break
    fi
done

if [ "$mode" = real ] && [ -f "$log" ]; then
    # The address and every long part of its path (the token is one) must
    # not be in what the app wrote; searched for, never printed.
    if ! python3 -I -c '
import sys, urllib.parse
address = open(sys.argv[1], "rb").readline().strip().decode("ascii")
parts = [part for part in urllib.parse.urlsplit(address).path.split("/") if len(part) >= 12]
data = b"".join(open(path, "rb").read() for path in sys.argv[2:])
sys.exit(1 if any(text.encode() in data for text in [address, *parts]) else 0)
' "$address_file" "$log" "$scratch/run.log"; then
        echo "the app's log holds the feed's address or token" >&2
        proven=0
    fi
fi
if [ "$proven" != 1 ]; then
    echo "feed slot: NOT proven ($mode) within 60 s" >&2
    if [ -f "$log" ]; then
        # The update lines only, without anything that looks like an address.
        grep -i 'update\|feed' "$log" | grep -v -i 'https\?://' | cut -c1-200 | tail -n 8 >&2 || true
    fi
    exit 1
fi
