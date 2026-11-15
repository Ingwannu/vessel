#!/usr/bin/env bash
# Sourced after fleet.sh has real fake-harness sessions and projects. Every
# definition and historical run is made through the CLI, never by editing DBs.
schedule_fixture_seed() { # fleet scratch root; cli() is supplied by fleet.sh
  local fleet_root=$1 history_runs=12 settlement_attempts=300
  cli --node laptop schedule create --name '한글 예약 — 1분 30초마다 검사' \
    --every 90s --harness claude-code --project shop --prompt plain >/dev/null
  cli --node laptop schedule create \
    --name 'Weekday review of the shared navigation and keyboard focus in narrow windows' \
    --cron '0 9 * * MON-FRI' --tz Asia/Seoul --harness claude-code \
    --project shop --prompt plain >/dev/null
  cli --node build-server schedule create --name 'Paused build review' \
    --every 2h --harness claude-code --prompt plain --paused >/dev/null
  cli --node laptop schedule create --name 'Failed run' --every 1d \
    --harness ghost-agent --prompt plain --paused >/dev/null
  cli --node laptop schedule run 'Failed run' > "$fleet_root/schedule-failed-run.txt"
  cli --node laptop schedule create --name 'Past runs with sessions' --every 1d \
    --harness claude-code --project shop --prompt plain --keep --paused >/dev/null
  local run attempt
  for ((run=0; run<history_runs; run++)); do
    cli --node laptop schedule run 'Past runs with sessions' >/dev/null
    # Run now returns after admission. The next run waits for the node's actual
    # settlement, rather than relying on a sleep or inventing completed runs.
    for ((attempt=0; attempt<settlement_attempts; attempt++)); do
      cli --node laptop schedule runs 'Past runs with sessions' > "$fleet_root/schedule-history.txt"
      if ! grep -qE 'being made|  started ' "$fleet_root/schedule-history.txt"; then break; fi
      sleep 0.1
    done
    if [ "$attempt" -eq "$settlement_attempts" ]; then
      echo 'fleet: schedule history run did not settle' >&2; return 1
    fi
  done
  local recurring
  recurring=$(awk '$3 == "queued" { print $2; exit }' "$fleet_root/sessions")
  [ -n "$recurring" ] || { echo 'fleet: recurring prompt target is missing' >&2; return 1; }
  cli --node laptop schedule create --name '반복 프롬프트' --every 30m \
    --session "$recurring" --prompt plain --paused >/dev/null
  cli --node laptop schedule list > "$fleet_root/schedules-laptop.txt"
  cli --node build-server schedule list > "$fleet_root/schedules-build.txt"
  local problems=()
  grep -Fq '한글 예약 — 1분 30초마다 검사  every 90s' "$fleet_root/schedules-laptop.txt" || problems+=('exact Korean interval')
  grep -Fq 'Weekday review of the shared navigation and keyboard focus in narrow windows  0 9 * * MON-FRI (Asia/Seoul)' "$fleet_root/schedules-laptop.txt" || problems+=('long cron name')
  grep -Fq 'Paused build review  every 7200s  paused since' "$fleet_root/schedules-build.txt" || problems+=('paused schedule')
  grep -q 'failed:' "$fleet_root/schedule-failed-run.txt" || problems+=('failed run')
  [ "$(grep -c '^#' "$fleet_root/schedule-history.txt")" -eq "$history_runs" ] || problems+=('many historical runs')
  grep -q 'finished (' "$fleet_root/schedule-history.txt" || problems+=('historical session links')
  grep -Fq '반복 프롬프트  every 1800s  paused since' "$fleet_root/schedules-laptop.txt" || problems+=('recurring prompt')
  if [ "${#problems[@]}" -gt 0 ]; then
    printf 'fleet: missing schedule shapes: %s\n' "${problems[*]}" >&2
    return 1
  fi
}
