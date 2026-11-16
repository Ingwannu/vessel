## 호스트 오케스트레이션 설정. 초안이 아닌 실제 호스트 상태를 표시합니다.
# 섹션과 행 제목. 짧게 유지합니다.
# Host settings heading above the tools switch.
orchestration-section = 오케스트레이션
# Switch label for the host-controlled agent tools server.
orchestration-tools-title = Vessel 도구 사용
# Supporting text below the tools switch; own means authority stays scoped.
orchestration-tools-hint = 에이전트가 자신이 만든 세션, 작업 트리와 일정을 관리할 수 있습니다.
# Host settings heading above skill management.
orchestration-skills-section = 스킬
# Switch label for installed orchestration skill bundles.
orchestration-skills-title = 오케스트레이션 스킬
# Supporting text below the skills switch.
orchestration-skills-hint = 에이전트를 위한 기본 스킬을 설치합니다.
# Status hint when bundled managed files need maintenance.
orchestration-update-available = 기본 스킬 업데이트가 있습니다.
# 완전한 복구 안내. 서버의 진단 문장은 표시하지 않습니다.
# Status shown while reading the chosen host; no mutation is implied.
orchestration-loading = 오케스트레이션 설정을 읽는 중…
# Recovery hint when the chosen machine has no authenticated connection.
orchestration-unavailable = 오케스트레이션 설정을 관리하려면 이 머신에 연결하세요.
# Shown for an older or feature-unavailable host; no request is invoked.
orchestration-unsupported = 이 머신은 아직 오케스트레이션 설정을 지원하지 않습니다.
# Generic operation failure; private server diagnostics are never appended.
orchestration-failed = 작업을 완료하지 못했습니다. 현재 상태를 읽고 다시 시도하세요.
# Permission refusal for host orchestration settings, not an agent permission prompt.
orchestration-access = 이 기기는 오케스트레이션 설정을 변경할 권한이 없습니다.
# A stale target or changed removal plan requires a fresh read.
orchestration-changed = 대상이나 삭제 계획이 바뀌었습니다. 현재 상태를 읽고 다시 시도하세요.
# Bounded operation queue refused admission; the user may retry later.
orchestration-busy = 다른 오케스트레이션 작업이 진행 중입니다. 잠시 후 다시 시도하세요.
# Deadline expired; the operation may still finish, so refresh before another write.
orchestration-unconfirmed = 변경 결과를 확인하지 못했습니다. 새로고침 후 다시 시도하세요.
# Request identities cannot be allocated safely until the app restarts.
orchestration-exhausted = 다른 오케스트레이션 요청을 보내려면 Vessel을 다시 시작하세요.
# 짧은 동작 이름 및 확인 목록에서 사용하는 독립된 작업 레이블.
# Button to read actual host state again without changing the picker draft.
orchestration-refresh = 새로고침
# Button that opens confirmation for bundled additions and updates.
orchestration-update = 업데이트
# Gear action and title of the skill selection dialog.
orchestration-choose = 스킬 선택
# Button committing the selection draft; removal may require separate confirmation.
orchestration-save = 저장
# Disabled loading button while the selection acknowledgement is outstanding.
orchestration-saving = 저장 중…
# Dismiss without writing; disabled during an outstanding selection save.
orchestration-cancel = 취소
# Destructive confirmation button for freshly listed deselected folders.
orchestration-remove = 삭제
# Destructive confirmation button removing all installed managed skill folders.
orchestration-uninstall = 설치 제거
# Loading label during a host skill operation.
orchestration-working = 작업 중…
# Standalone operation verb beside a skill directory name in a confirmation list.
orchestration-add = 추가
# Standalone maintenance verb beside a skill directory name.
orchestration-operation-update = 업데이트
# Button opening the trusted public skill documentation on an explicit click.
orchestration-docs = 스킬 안내 읽기
# 선택 창. 열 때만 초안을 초기화합니다.
# Selection dialog switch: use all current and future bundled skills.
orchestration-all-title = 모든 스킬
# Explains that all is dynamic across future release catalogues.
orchestration-all-hint = 이후 업데이트에서 추가되는 스킬도 포함합니다.
# Heading above per-skill selection switches.
orchestration-available = 사용 가능한 스킬
# Selection dialog empty state when the host advertises no bundled skill names.
orchestration-empty = 사용 가능한 기본 스킬이 없습니다.
# 명시적 확인. 이름은 문장 아래의 별도 데이터 목록에 표시합니다.
# Confirmation dialog title before removing all managed skill folders.
orchestration-uninstall-title = 스킬을 제거할까요?
# Warns that whole folders include user additions; saved selection survives.
orchestration-uninstall-message = 추가한 파일을 포함해 스킬 폴더 전체를 삭제합니다. 다시 설치할 수 있도록 선택 내역은 유지합니다.
# Confirmation dialog title for freshly computed deselected skill folders.
orchestration-remove-title = 선택 해제한 스킬을 삭제할까요?
# Deletion warning; exact folder names follow as separate data rows.
orchestration-remove-message = 머신에서 다음 폴더의 삭제가 필요하다고 응답했습니다. 폴더 안의 모든 파일을 삭제합니다.
# Confirmation title for non-destructive bundled skill maintenance.
orchestration-update-title = 스킬을 업데이트할까요?
# Maintenance explanation; ordered addition/update rows follow separately.
orchestration-update-message = 다음 기본 스킬의 추가 및 업데이트를 적용합니다.
