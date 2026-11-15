#!/usr/bin/env bash
# Extra real-shape fixtures for the official agent mark seam. Fake nodes only.
set -euo pipefail
root=${1:?fake fleet scratch directory}
target=${2:?debug binaries directory}
[ -f "$root/pids" ] && [ -f "$root/sessions" ] || { echo 'start the fake fleet first' >&2; exit 1; }
cli() {
  env -i PATH="$root/bin:/usr/local/bin:/usr/bin:/bin" HOME="$root" VESSEL_CONFIG_DIR="$root/config" "$target/vessel" "$@"
}
cli --node laptop agents add mark-custom-en --replace --label 'Claude Code — a custom team agent with a deliberately long display name' -- "$target/fake-acp" > /dev/null
cli --node laptop agents add mark-custom-ko --replace --label 'Codex와 이름이 비슷하지만 사용자 명령으로 추가한 긴 한글 에이전트 이름' -- "$target/fake-acp" > /dev/null
cli --node laptop agents > "$root/mark-agents.txt"
# Native ACP harnesses use the same fake transport; their official typed
# identities remain intact so the model browser can exercise every mark.
for agent in gemini opencode; do ln -sf "$target/fake-acp" "$root/bin/$agent"; done
: > "$root/mark-sessions"
for entry in english korean gemini opencode; do
  agent=claude-code
  case "$entry" in
    english) title='Review the authentication lifecycle and recover unsaved editor changes across reconnects';;
    korean) title='연결이 끊긴 뒤 저장하지 않은 편집 내용과 인증 상태를 안전하게 복구하는 흐름 검토';;
    gemini) agent=gemini; title='Check the original Google Gemini artwork in a long-running session card';;
    opencode) agent=opencode; title='공식 OpenCode 최소 크기와 여백을 확인하는 긴 이름의 세션 카드';;
  esac
  id=$(cli --node laptop run "$agent" --structured --cwd "$root/work/vessel-client" --detach | tail -1)
  cli --node laptop send "$id" "$title" > /dev/null
  printf '%s %s\n' "$entry" "$id" >> "$root/mark-sessions"
done
python3 - "$root" "$target" <<'PY_CHECK'
import json,pathlib,sqlite3,sys
root=pathlib.Path(sys.argv[1])
target=pathlib.Path(sys.argv[2])
lines=set((root/'mark-agents.txt').read_text().splitlines())
for agent,label in [('mark-custom-en','Claude Code — a custom team agent with a deliberately long display name'),('mark-custom-ko','Codex와 이름이 비슷하지만 사용자 명령으로 추가한 긴 한글 에이전트 이름')]:
    assert f"{agent} ({label}): {target / 'fake-acp'}" in lines
con=sqlite3.connect(f"file:{root / 'laptop/vessel.db'}?mode=ro",uri=True)
for entry in (root/'mark-sessions').read_text().splitlines():
    language,session=entry.split()
    title,spec=con.execute('SELECT title,spec FROM sessions WHERE id=?',(session,)).fetchone()
    expected_agent={'english':'claude-code','korean':'claude-code','gemini':'gemini','opencode':'open-code'}[language]
    assert json.loads(spec)['harness']==expected_agent
    expected={'english':'Review the authentication lifecycle and recover unsaved editor changes across reconnects','korean':'연결이 끊긴 뒤 저장하지 않은 편집 내용과 인증 상태를 안전하게 복구하는 흐름 검토','gemini':'Check the original Google Gemini artwork in a long-running session card','opencode':'공식 OpenCode 최소 크기와 여백을 확인하는 긴 이름의 세션 카드'}[language]
    assert title==expected[:80],(language,title)
print('verified four session titles and two long bilingual custom agent labels')
PY_CHECK
