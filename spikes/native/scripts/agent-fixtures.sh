#!/usr/bin/env bash
# Sourced by fleet.sh. Each node gets its own fake program directory so
# one machine can lack an adapter without changing the other's state.
# Registry runners and npm never download or execute external packages.

agent_fixture_bin() { # fleet-root binaries machine
  local fleet_root=$1 binaries=$2 machine=$3
  local bin="$fleet_root/$machine-bin"
  mkdir -p "$bin"
  ln -sf "$binaries/fake-tui" "$bin/claude"
  ln -sf "$fleet_root/bin/claude-agent-acp" "$bin/claude-agent-acp"
  # No session uses Codex in the fleet, so its sign-out check is not
  # suppressed by the node's credential race guard.
  printf '#!/bin/sh\nFAKE_TUI_LOGGED_OUT=1 exec "%s" "$@"\n' "$binaries/fake-tui" > "$bin/codex"
  chmod 755 "$bin/codex"
  if [ "$machine" = laptop ]; then ln -sf "$binaries/fake-acp" "$bin/codex-acp"; fi
  for runner in npx uvx; do ln -sf "$binaries/fake-acp" "$bin/$runner"; done
  # The installer reads its completion from a separate FIFO (stdin is
  # closed by the node): captures can hold progress, then release success
  # or failure deterministically without a timed fake install.
  cat > "$bin/npm" <<'SH'
#!/bin/sh
printf 'Fake installer: preparing the requested package\n'
if [ -p "$HOME/install-gate" ]; then
  IFS= read -r result < "$HOME/install-gate"
else
  result=success
fi
if [ "$result" = fail ]; then
  printf 'Fake installer: requested failure\n' >&2
  exit 7
fi
printf 'Fake installer: completed without downloading a package\n'
SH
  chmod 755 "$bin/npm"
}

agent_fixture_config() { # data-directory
  local data=$1
  mkdir -p "$data"
  cat > "$data/models.toml" <<'TOML'
[providers.fixture]
api = "openai-chat-completions"
base_url = "http://127.0.0.1:9/v1"

[models.example-coder-with-a-long-route-name]
provider = "fixture"
model = "example/very-long-model-name-for-checking-settings-layout-in-small-windows"
small_model = "example/small"
TOML
  cat > "$data/profiles.toml" <<'TOML'
[profiles.claude-example]
harness = "claude"
model = "example-coder-with-a-long-route-name"

[profiles.codex-example]
harness = "codex"
model = "example-coder-with-a-long-route-name"
TOML
  chmod 600 "$data/models.toml" "$data/profiles.toml"
}

agent_fixture_seed() { # fleet-root binaries; cli() supplied by fleet.sh
  local fleet_root=$1 binaries=$2
  local long_arg
  long_arg=$(printf 'long-command-argument-for-a-small-settings-window-%.0s' {1..8})
  cli --node laptop agents add fake-goose --label '추가한 에이전트의 긴 이름 — ACP conversation fixture' -- "$binaries/fake-acp" "$long_arg" >/dev/null
  cli --node laptop agents add ghost-agent --label 'Program no longer present' -- "$fleet_root/missing-agent-program" >/dev/null
  cli --node laptop agents add check-failed --label 'Agent check fails' --env FAKE_ACP_EXIT_EARLY=1 -- "$binaries/fake-acp" >/dev/null
  cli --node laptop agents add needs-sign-in --label 'Agent with sign-in steps' --env FAKE_ACP_SIGN_IN=1 -- "$binaries/fake-acp" >/dev/null
  cli --node laptop agents status check-failed --refresh > "$fleet_root/failed-check.txt"
  cli --node laptop agents status needs-sign-in --refresh > "$fleet_root/help-check.txt"
  cli --node laptop agents status codex --refresh > "$fleet_root/signed-out-check.txt"
  cli --node laptop agents > "$fleet_root/agents-check.txt"
  cli --node laptop agents status gemini > "$fleet_root/missing-known-check.txt"
  cli --node laptop agents status ghost-agent > "$fleet_root/ghost-check.txt"
  cli --node build-server harnesses > "$fleet_root/adapter-check.txt"
  cli --node laptop models > "$fleet_root/models-check.txt"
  grep -q 'failed' "$fleet_root/failed-check.txt" || { echo 'fleet: failed check is missing' >&2; return 1; }
  grep -q 'it offers Fake login' "$fleet_root/help-check.txt" || { echo 'fleet: sign-in steps are missing' >&2; return 1; }
  grep -q 'not signed in' "$fleet_root/signed-out-check.txt" || { echo 'fleet: signed-out agent is missing' >&2; return 1; }
  grep -q 'fake-goose' "$fleet_root/agents-check.txt" || { echo 'fleet: added agent is missing' >&2; return 1; }
  grep -q "$long_arg" "$fleet_root/agents-check.txt" || { echo 'fleet: long command is missing' >&2; return 1; }
  grep -q 'not installed' "$fleet_root/missing-known-check.txt" || { echo 'fleet: missing built-in program is missing' >&2; return 1; }
  grep -q 'not installed' "$fleet_root/ghost-check.txt" || { echo 'fleet: missing added program is missing' >&2; return 1; }
  grep -q 'codex-acp' "$fleet_root/adapter-check.txt" || { echo 'fleet: missing adapter case is missing' >&2; return 1; }
  grep -q 'example-coder-with-a-long-route-name' "$fleet_root/models-check.txt" || { echo 'fleet: model route is missing' >&2; return 1; }
}
