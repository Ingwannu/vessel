## Caller authority for the selected admitted connection. No peer text.

# Neutral loading label while the selected connection’s own grant is requested.
device-authority-loading = 이 연결의 권한 확인 중

# Short button: explicitly read this connection’s effective grant again.
device-authority-retry = 권한 확인

# Known non-admin grant: one hint under Device management; never shown for a failed read.
device-authority-denied = 이 연결에는 페어링된 기기를 관리할 권한이 없습니다. 관리자 연결을 사용하세요.


# Authority read failed because the selected admission was replaced.
device-authority-changed = 머신의 연결이 바뀌어 권한을 확인하지 못했습니다. 권한을 다시 확인하세요.

# Authority read failed because its original transport closed.
device-authority-offline = 머신과의 연결이 끊어져 권한을 확인하지 못했습니다. 다시 연결한 뒤 권한을 확인하세요.

# The monotonic request source cannot safely allocate another identity.
device-authority-exhausted = 권한을 안전하게 확인할 수 없습니다. 앱을 다시 열고 시도하세요.

# An invalid self report is refused; no reported grant is retained.
device-authority-invalid = 머신의 응답이 올바르지 않아 권한을 확인하지 못했습니다. 다시 시도하세요.

# A management control was refused locally because no admin report is current.
device-authority-not-confirmed = 이 연결의 관리 권한이 확인되지 않았습니다. 권한을 확인한 뒤 다시 시도하세요.

# Old/unsupported node recovery; no compatibility admin grant is invented.
device-authority-unsupported = 이 머신은 연결의 권한을 알려 줄 수 없습니다. 머신의 Vessel을 업데이트한 뒤 다시 연결하세요.

# Authenticated/authorization refusal: this query establishes no grant.
device-authority-refused = 이 연결의 권한을 확인하지 못했습니다. 머신에 다시 연결한 뒤 시도하세요.

# Temporary failure of this query; never a statement of denied access.
device-authority-unavailable = 지금은 이 연결의 권한을 확인할 수 없습니다. 다시 시도하세요.

# Short settings row title above the known non-admin explanation.
device-authority-management = 기기 관리

# A sent write lost its admission; no success or definite failure is inferred.
device-authority-uncertain-title = 결과 확인 필요
device-authority-uncertain-invitation = 페어링 링크를 만드는 동안 연결이 바뀌었습니다. 링크가 만들어졌을 수 있지만 안전하게 표시할 수 없습니다. 다시 연결한 뒤 필요하면 새 링크를 만드세요.
device-authority-uncertain-revocation = 접근을 해제하는 동안 연결이 바뀌었습니다. 이미 해제되었을 수 있습니다. 다시 연결해 페어링된 기기를 확인한 뒤 재시도하세요.
device-authority-uncertain-both = 페어링 링크 생성과 접근 해제 중에 연결이 바뀌었습니다. 두 작업 모두 이미 처리되었을 수 있습니다. 다시 연결해 페어링된 기기를 확인한 뒤 재시도하고, 필요하면 새 링크를 만드세요.
