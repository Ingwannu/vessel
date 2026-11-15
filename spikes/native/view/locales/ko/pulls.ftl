pulls-create = PR 생성
pulls-creating = PR 생성 중...
pulls-created = PR 생성됨
pulls-existing = 이 브랜치의 PR이 이미 열려 있습니다
pulls-view = PR 보기
pulls-label = 풀 리퀘스트
pulls-failed = PR을 생성하지 못했습니다

pulls-no-commits = 이 브랜치에 아직 새 커밋이 없어 PR 생성을 사용할 수 없습니다
pulls-no-branch = 브랜치에 있지 않아 PR 생성을 사용할 수 없습니다
pulls-no-remote = 이 브랜치에 GitHub 원격이 없어 PR 생성을 사용할 수 없습니다
pulls-no-forge = { $host }{ PARTICLE($host, "subject") } 연결되어 있지 않기 때문에 지금은 PR 생성을 사용할 수 없습니다
pulls-signed-out = 이 머신이 { $host }에 로그인되어 있지 않아 PR 생성을 사용할 수 없습니다. 머신에서 gh auth login을 실행하세요.
pulls-refused = { $host }{ PARTICLE($host, "subject") } 이 머신의 로그인을 거부해 PR 생성을 사용할 수 없습니다
pulls-rate-limited = { $host }{ PARTICLE($host, "subject") } 요청을 제한하고 있어 지금은 PR 생성을 사용할 수 없습니다. 나중에 다시 시도하세요.
pulls-unreachable = { $host }에 연결할 수 없어 지금은 PR 생성을 사용할 수 없습니다
pulls-limit = 이 머신이 지켜볼 수 있는 풀 리퀘스트 수가 다 차서 PR 생성을 사용할 수 없습니다
pulls-ended = 이 세션이 끝나서 PR 생성을 사용할 수 없습니다
pulls-unavailable = 지금은 PR 생성을 사용할 수 없습니다

pulls-form-title = 풀 리퀘스트 생성
pulls-field-title = 제목
pulls-field-body = 설명
pulls-field-base = 기준 브랜치
pulls-default-base = 기본 브랜치
pulls-field-draft = 초안으로 생성
pulls-cancel = 취소
pulls-title-blank = 제목을 입력하세요
pulls-title-long = 제목은 256자 이내로 입력하세요
pulls-body-long = 설명은 64KB 이내로 입력하세요
pulls-not-accepted = 이 제목이나 설명은 보낼 수 없습니다

pulls-not-pushed = PR을 생성하기 전에 이 브랜치를 푸시하세요
pulls-push-refused = 이 브랜치를 푸시하지 못했습니다
pulls-rejected = 풀 리퀘스트가 거부되었습니다
pulls-refused-unknown = 풀 리퀘스트를 생성하지 못했습니다

pulls-state-open = 열림
pulls-state-draft = 초안
pulls-state-merged = 병합됨
pulls-state-closed = 닫힘
pulls-state-unknown = 알 수 없음

pulls-checks = 검사
pulls-activity = 활동
pulls-checks-failure = 실패한 검사가 있습니다
pulls-checks-pending = 아직 완료되지 않은 검사가 있습니다
pulls-checks-success = 모든 검사를 통과했습니다
pulls-checks-none = 검사 없음
pulls-checks-unknown = 검사를 아직 읽지 않았습니다
pulls-count-failing = { NUMBER($count) }개 실패
pulls-count-pending = { NUMBER($count) }개 진행 중
pulls-count-passed = { NUMBER($count) }개 성공
pulls-checks-more = 목록에 없는 실패한 검사가 더 있습니다

pulls-approved = 승인됨
pulls-changes-requested = 변경 요청됨
pulls-review-required = 리뷰 필요
pulls-unresolved = 해결되지 않은 스레드 { NUMBER($count) }개

pulls-merge-ready = 병합 가능
pulls-merge-conflicts = 기준 브랜치와 충돌
pulls-merge-blocked = 규칙에 막힘
pulls-merge-unstable = 통과하지 않은 검사 있음
pulls-merge-behind = 기준 브랜치보다 뒤처짐
pulls-merge-unknown = 아직 모름

pulls-open-on = { $host }에서 열기
pulls-refresh = 새로 고침
pulls-empty-title = 아직 풀 리퀘스트가 없습니다
pulls-empty-description = 이 체크아웃에 풀 리퀘스트를 만들면 세부 정보가 여기에 표시됩니다.

pulls-card-passed = 통과
pulls-card-failed = 실패
pulls-card-running = 진행 중

pulls-merge-squash = PR 병합(스쿼시)
pulls-merge-merge = PR 병합(병합)
pulls-merge-rebase = PR 병합(리베이스)
pulls-merging = PR 병합 중...
pulls-merged = PR 병합됨
pulls-merge-failed = PR을 병합하지 못했습니다
pulls-merge-missing = 아직 풀 리퀘스트가 없어 PR 병합을 사용할 수 없습니다
pulls-merge-other = 이 세션의 풀 리퀘스트가 이제 #{ $number }(이)라 PR 병합을 사용할 수 없습니다
pulls-merge-already = 풀 리퀘스트가 이미 병합되어 PR 병합을 사용할 수 없습니다
pulls-merge-closed = 풀 리퀘스트가 닫혀 있어 PR 병합을 사용할 수 없습니다
pulls-merge-draft = 풀 리퀘스트가 아직 초안이라 PR 병합을 사용할 수 없습니다
pulls-merge-moved = 확인한 뒤에 브랜치에 새 커밋이 생겨 PR 병합을 사용할 수 없습니다. 다음 새로 고침 뒤에 다시 시도하세요.
pulls-merge-not-ready = GitHub에서 PR의 병합 준비가 확인될 때까지 PR을 병합할 수 없습니다
pulls-merge-method = 이 저장소가 이 병합 방식을 허용하지 않아 PR 병합을 사용할 수 없습니다
pulls-merge-queue = 이 저장소는 병합 대기열을 사용하므로 여기서 PR 병합을 사용할 수 없습니다
pulls-merge-pin = GitHub가 이 방식으로는 확인한 커밋 그대로 병합할 수 없어 PR 병합을 사용할 수 없습니다
pulls-merge-no-github = GitHub가 연결되어 있지 않아 지금은 PR 병합을 사용할 수 없습니다
pulls-merge-rejected = 병합이 거부되었습니다
pulls-merge-refused-unknown = 풀 리퀘스트를 병합하지 못했습니다
pulls-feedback = 실패한 검사와 리뷰를 세션에 보내기
