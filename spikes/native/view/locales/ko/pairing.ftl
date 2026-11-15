## 페어링 링크와 머신 확인 대화 상자.
pairing-title = 페어링 링크 붙여넣기
pairing-helper = 머신이 출력한 페어링 링크를 붙여넣으세요.
pairing-label = 페어링 링크
pairing-security-warning = 이 페어링 링크는 비밀번호처럼 취급하세요. 링크를 가진 사람은 누구나 이 머신에 접근할 수 있습니다.
pairing-cancel = 취소
pairing-pair = 페어링
pairing-busy = 페어링 중...
pairing-confirm-title = 이 머신에 연결할까요?
pairing-confirm-description = 이 머신은 이 앱에서 코드를 실행하고 연결된 다른 머신에 접근할 수 있게 됩니다. 알고 있는 머신인 경우에만 연결하세요.
pairing-machine = 머신
pairing-fingerprint = 키 지문
pairing-relay = 릴레이
pairing-connect = 연결

# Link input placeholder; keep the pairing URI spelling unchanged.
pairing-link-placeholder = vessel://pair#…

# Persistent label above the local machine name field. Keep short.
pairing-name-label = 이 머신의 이름

# Help beneath the local name field; the example is not a default value.
pairing-name-hint = 알아보기 쉬운 짧은 이름 (예: home-pc)

# Name field error when a fingerprint was entered instead of a name.
pairing-name-is-key = 이것은 키입니다. 대신 짧은 이름을 적으세요 (예: home-pc).

# Name field error when a pairing link was entered instead of a name.
pairing-name-is-link = 이것은 페어링 링크입니다. 여기에는 짧은 이름을 적거나, 머신 자신의 이름을 쓰려면 비워 두세요.

# Name field error when nothing was typed.
pairing-name-empty = 짧은 이름을 적으세요 (예: home-pc).

# Name field error for the node book naming rules.
pairing-name-invalid = 영문자, 숫자, '.', '_', '-'만 쓸 수 있고 영문자나 숫자로 시작해야 합니다.

# Name field error when another machine already has that name.
pairing-name-taken = 이미 같은 이름의 머신이 있습니다.

# Link field error if connecting was attempted without a decoded offer.
pairing-paste-first = 먼저 페어링 링크를 붙여 넣으세요.

# Link field error from the typed decoder; no input or diagnostic is echoed.
pairing-link-wrong-kind = 페어링 링크가 아닙니다. 머신에서 페어링 링크를 복사해 붙여넣으세요.

# Link field error from the typed decoder; no input or diagnostic is echoed.
pairing-link-encoding = 페어링 링크가 손상되었습니다. 머신에서 페어링 링크 전체를 다시 복사하세요.

# Link field error from the typed decoder; no input or diagnostic is echoed.
pairing-link-offer = 페어링 링크를 읽을 수 없습니다. 머신에서 새 페어링 링크를 만든 뒤 다시 시도하세요.


# Connect request failure beside the confirmation form; no host diagnostic is echoed.
pairing-failed-name = 이 이름을 더 이상 사용할 수 없습니다. 다른 이름으로 다시 시도하세요.

# Connect request failure beside the confirmation form; no host diagnostic is echoed.
pairing-failed-storage = 머신을 저장할 수 없습니다. 앱이 설정 폴더에 쓸 수 있는지 확인한 뒤, 새 페어링 링크를 만들어 다시 시도하세요.

# Connect request failure beside the confirmation form; no host diagnostic is echoed.
pairing-failed-connection = 머신에 연결할 수 없습니다. 머신이 실행 중인지 확인한 뒤, 새 페어링 링크를 만들어 다시 시도하세요.

# Connect request failure beside the confirmation form; no host diagnostic is echoed.
pairing-failed-browser = 이 브라우저에서는 페어링할 수 없습니다. 데스크톱 앱에서 이 머신을 페어링하세요.

pairing-failed-unsent = 보내지 못했습니다. 다시 시도하세요.

# Footer after a successful pairing. $name is the machine name in the sidebar;
# $via is direct or relay. The address is a separate tooltip, never part of this sentence.
pairing-paired = { $via ->
    [direct] { $name } 머신에 직접 연결됨
   *[relay] { $name } 머신에 릴레이로 연결됨
    }

# Empty local-name input hint. Short; an empty value asks for the machine's own name.
pairing-name-placeholder = 비워 두면 머신 자신의 이름을 씁니다
