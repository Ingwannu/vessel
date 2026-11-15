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
  ln -sf "$binaries/fake-tui" "$bin/fake-codex-cli"
  # A capture can hold initial discovery with a FIFO in this scratch home;
  # the node's own bounded version probe still ends it after five seconds.
  cat > "$bin/codex" <<'SH'
#!/bin/sh
if [ "$1" = --version ] && [ -p "$HOME/discovery-gate" ]; then
  printf 'Fake version check: waiting for discovery gate\n' >&2
  IFS= read -r fixture_release < "$HOME/discovery-gate"
fi
FAKE_TUI_LOGGED_OUT=1 exec "$(dirname "$0")/fake-codex-cli" "$@"
SH
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

# One named fixture count, shared by generated configuration and its wire check.
AGENT_FIXTURE_MODEL_COUNT=24
AGENT_FIXTURE_UNBROKEN_MODEL='unbrokenmodelidentifierwithnoslashorhyphenforcheckingthatreadablesettingsvaluesfitinsmallwindowsandarestillcopiedasonewholeidentifier'
AGENT_FIXTURE_LONG_MODEL='fixture/a-model-identifier-with-a-long-namespace-and-version-that-remains-readable-in-a-small-window/한글-모델'

agent_fixture_config() { # data-directory machine
  local data=$1 machine=$2
  mkdir -p "$data"
  cat > "$data/models.toml" <<'TOML'
[providers.fixture]
api = "openai-chat-completions"
base_url = "http://127.0.0.1:9/v1"

[providers.alternate]
api = "anthropic-messages"
base_url = "http://127.0.0.1:9/v1"
TOML
  if [ "$machine" = laptop ]; then
    cat >> "$data/models.toml" <<'TOML'

[models.example-coder-with-a-long-route-name]
provider = "fixture"
model = "example/very-long-model-name-for-checking-settings-layout-in-small-windows"
small_model = "example/small"
TOML
    local number
    for number in $(seq 1 "$((AGENT_FIXTURE_MODEL_COUNT - 1))"); do
      local model
      if [ "$number" -eq 1 ]; then model="$AGENT_FIXTURE_LONG_MODEL";
      elif [ "$number" -eq 2 ]; then model="$AGENT_FIXTURE_UNBROKEN_MODEL";
      else model="fixture/model-$number"; fi
      printf '\n[models.fixture-model-%02d]\nprovider = "alternate"\nmodel = "%s"\n' \
        "$number" "$model" >> "$data/models.toml"
    done
  fi
  cat > "$data/profiles.toml" <<'TOML'
[profiles.claude-example]
harness = "claude"

[profiles.codex-example]
harness = "codex"
TOML
  if [ "$machine" = laptop ]; then
    # Append the route inside a profile table, not an inferred compatibility
    # claim. Two accounts use the same agent and must remain distinct.
    cat > "$data/profiles.toml" <<'TOML'
[profiles.claude-example]
harness = "claude"
model = "example-coder-with-a-long-route-name"
label = "Work account"

[profiles.codex-example]
harness = "codex"
model = "example-coder-with-a-long-route-name"

[profiles.codex-personal]
harness = "codex"
model = "example-coder-with-a-long-route-name"
label = "같은 에이전트의 별도 프로필 — 긴 이름과 한글을 확인하는 개인 계정"
TOML
  fi
  chmod 600 "$data/models.toml" "$data/profiles.toml"
}

agent_fixture_seed() { # fleet-root binaries; cli() supplied by fleet.sh
  local fleet_root=$1 binaries=$2
  agent_fixture_check_discovery "$fleet_root"
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
  [ "$(wc -l < "$fleet_root/models-check.txt")" -eq "$AGENT_FIXTURE_MODEL_COUNT" ] || { echo 'fleet: many model routes are missing' >&2; return 1; }
  grep -qF "$AGENT_FIXTURE_LONG_MODEL" "$fleet_root/models-check.txt" || { echo 'fleet: long Korean model identifier is missing' >&2; return 1; }
  grep -qF "$AGENT_FIXTURE_UNBROKEN_MODEL" "$fleet_root/models-check.txt" || { echo 'fleet: unbroken model identifier is missing' >&2; return 1; }
  grep -q 'anthropic-messages' "$fleet_root/models-check.txt" || { echo 'fleet: alternate provider API is missing' >&2; return 1; }
  cli --node laptop profiles > "$fleet_root/profiles-check.txt"
  grep -q 'codex-personal' "$fleet_root/profiles-check.txt" || { echo 'fleet: separate model-use account is missing' >&2; return 1; }
  grep -q '같은 에이전트의 별도 프로필' "$fleet_root/profiles-check.txt" || { echo 'fleet: Korean profile label is missing' >&2; return 1; }
  cli --node build-server models > "$fleet_root/empty-models-check.txt"
  grep -Fxq "no model routes (the node's models.toml defines them)" "$fleet_root/empty-models-check.txt" || { echo 'fleet: empty model list is missing' >&2; return 1; }
}

# Exercise the optional gate in a separate scratch home, so checking the
# fixture never delays a node that is already serving requests.
agent_fixture_check_discovery() { # fleet-root
  local fleet_root=$1 check_home result=0
  check_home=$(mktemp -d "$fleet_root/discovery-check.XXXXXX")
  mkfifo "$check_home/discovery-gate"
  timeout 1 env HOME="$check_home" "$fleet_root/laptop-bin/codex" --version > "$fleet_root/discovery-check.txt" 2>&1 || result=$?
  rm "$check_home/discovery-gate"
  rmdir "$check_home"
  if [ "$result" -ne 124 ] || ! grep -Fxq 'Fake version check: waiting for discovery gate' "$fleet_root/discovery-check.txt"; then
    echo 'fleet: held discovery fixture is missing' >&2
    return 1
  fi
}
