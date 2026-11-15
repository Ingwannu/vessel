## Updates in the title bar, About and the updated-on-start banner.
## About follows Paseo settings.about, desktop.updates.status and desktop.daemon.logs.
## Title-bar wording keeps the existing Zed-inspired labels. Values are data,
## and every dynamic message is formatted before a frame is drawn.

# Title bar while checking for a build. Keep short.
update-checking = 업데이트 확인 중…

# Title-bar action once a build is ready. Keep short.
update-restart = 다시 시작해 업데이트

# Title-bar status when an attempt fails. Keep short.
update-failed = 업데이트하지 못했습니다

# Title-bar retry action. Keep short.
update-try-again = 다시 시도

# Settings page title. Keep short.
update-about = 정보

# About: running app's version row title. Keep short.
update-app-version = 앱 버전

# About: hint below the running version's row title. Keep short.
update-this-device = 이 기기

# About: update status row title. Keep short.
update-app-updates = 앱 업데이트

# About: check now action. Keep short.
update-check = 확인

# About: check action while waiting. Keep short.
update-checking-short = 확인 중...

# About: install action before a newer build is ready. Keep short.
update-update = 업데이트

# About: a check is running.
update-status-checking = 앱 업데이트 확인 중...

# About: the running build is the newest.
update-status-up-to-date = 앱이 최신 상태입니다.

# About: the attempt failed; the safe explanation is shown below.
update-status-failed = 앱을 업데이트하지 못했습니다.

# About: no check has been made this run.
update-status-idle = 업데이트 상태를 아직 확인하지 않았습니다.

# About: application log row title. Keep short.
update-log-file = 로그 파일

# About: open the log folder. Keep short.
update-open-logs = 로그 열기

# About and General: copy a folder's path. Keep short.
update-copy-path = 경로 복사

# Frame notice after the log folder path was copied.
update-log-path-copied = 로그 경로가 복사되었습니다.

# About: no log folder path is known.
update-log-unavailable = 로그 경로를 사용할 수 없습니다

# About: open this build's release notes. Keep short.
update-whats-new = 새로운 소식

# About: hint below the release-notes action.
update-whats-new-hint = 이 버전의 릴리스 노트

# Install confirmation dialog title. Keep short.
update-install-title = 데스크톱 업데이트 설치

# Install confirmation: this app on this computer will be replaced.
update-install-message = 이 컴퓨터의 Vessel을 업데이트합니다

# Install confirmation action. Keep short.
update-install-confirm = 업데이트 설치

# Close install confirmation without installing. Keep short.
update-cancel = 취소

# About: automatic-update switch label. Keep short.
update-automatic = 자동으로 업데이트

# About: effect of disabling automatic checks.
update-automatic-note = 끄면 확인을 누를 때만 새 빌드를 찾습니다.

# About: the development build has no updater.
update-no-updates = 이 빌드는 스스로 업데이트하지 않습니다(개발 빌드).

# Dismiss the updated-on-start banner. Keep short.
update-dismiss = 닫기

# Title-bar progress. $percent is a numeric percentage. Keep short.
update-downloading = 업데이트 받는 중… { NUMBER($percent) }%

# Banner after restarting into a build. $label is its raw release label.
update-updated-to = 업데이트를 마쳤습니다: { $label }

# Numbered build display. $label is the release label without v; words such as dev stay data.
update-version = v{ $label }

# About install action for a ready numbered build. $label has no v prefix. Keep short.
# $kind selects a numbered release or a build word such as dev.
update-update-to = { $kind ->
    [numbered] 업데이트 설치: v{ $label }
   *[word] 업데이트 설치: { $label }
    }

# About: a numbered build is downloading. $label has no v prefix.
# $kind selects a numbered release or a build word such as dev.
update-status-downloading = { $kind ->
    [numbered] 업데이트 발견: v{ $label }. 다운로드 중...
   *[word] 업데이트 발견: { $label }. 다운로드 중...
    }

# About: a numbered build is ready. $label has no v prefix.
# $kind selects a numbered release or a build word such as dev.
update-status-ready = { $kind ->
    [numbered] 업데이트 준비됨: v{ $label }
   *[word] 업데이트 준비됨: { $label }
    }

# Safe fallback only if message preparation itself fails; diagnostics stay in logs.
update-details-unavailable = 업데이트 정보를 표시할 수 없습니다.

# Update failure recovery: check. Diagnostics stay in logs.
update-problem-check = 업데이트 확인을 마치지 못했습니다. 나중에 다시 시도하세요.

# Update failure recovery: feed too large. Diagnostics stay in logs.
update-problem-feed-too-large = 업데이트 정보가 이 앱이 읽을 수 있는 크기보다 큽니다. 나중에 다시 시도하세요.

# Update failure recovery: feed invalid. Diagnostics stay in logs.
update-problem-feed-invalid = 업데이트 정보를 읽지 못했습니다. 나중에 다시 시도하세요.

# Update failure recovery: feed unverified. Diagnostics stay in logs.
update-problem-feed-unverified = 업데이트 정보를 검증하지 못했습니다. 이 앱은 변경되지 않았습니다.

# Update failure recovery: feed new format. Diagnostics stay in logs.
update-problem-feed-new-format = 이 앱은 새로운 업데이트 형식을 읽을 수 없습니다. 최신 빌드를 직접 설치하세요.

# Update failure recovery: no build. Diagnostics stay in logs.
update-problem-no-build = 이 기기에 맞는 업데이트 빌드가 없습니다.

# Update failure recovery: download. Diagnostics stay in logs.
update-problem-download = 업데이트를 다운로드하지 못했습니다. 연결을 확인하고 다시 시도하세요.

# Update failure recovery: download write. Diagnostics stay in logs.
update-problem-download-write = 이 앱 옆에 업데이트를 저장하지 못했습니다. 앱 폴더에 쓸 수 있는지 확인하고 다시 시도하세요.

# Update failure recovery: download integrity. Diagnostics stay in logs.
update-problem-download-integrity = 다운로드한 파일을 검증하지 못해 설치하지 않았습니다. 다시 시도하세요.

# Update failure recovery: install write. Diagnostics stay in logs.
update-problem-install-write = 업데이트로 이 앱을 교체하지 못했습니다. 앱 폴더에 쓸 수 있는지 확인하고 다시 시도하세요.

# Update failure recovery: unsupported. Diagnostics stay in logs.
update-problem-unsupported = 이 빌드는 스스로 업데이트할 수 없습니다. 최신 빌드를 직접 설치하세요.

# Update failure recovery: interrupted. Diagnostics stay in logs.
update-problem-interrupted = 업데이트가 중단되었습니다. 다시 시도하세요.

# Update failure recovery: restart. Diagnostics stay in logs.
update-problem-restart = 업데이트된 앱을 시작하지 못했습니다. Vessel은 계속 열려 있으니 다시 시작해 보세요.
