## 세션 가져오기 시트. 참조: Paseo i18n importSession (resources/ko.ts 405-442)
## 와 호스트 선택 창(hosts/host-chooser.tsx).

# The sheet's title, and the new-session page's button that opens it.
history-title = 세션 가져오기
# The sheet's first page when no machine is in context: choose one.
history-choose-host = 호스트에서 가져오기
# Placeholder of the first page's search over machines.
history-search-hosts = 호스트 검색...
# The first page when no machine matches its search.
history-no-hosts = 일치하는 호스트가 없습니다
# Shown when no machine is connected.
history-connect-host = 세션을 가져오려면 호스트에 연결하세요
# Placeholder of the sheet's search over the listed sessions.
history-search = 세션 검색...
# The line under the title when the list covers the whole machine.
history-scope-host = { $host }의 세션
# The line under the title when the list covers the current folder only.
history-scope-workspace = 이 워크스페이스
# The small button beside "This workspace": list the whole machine. Keep short.
history-show-all = 전체 표시
# The provider filter's choice that shows every agent. Keep short.
history-filter-all = 전체
# Title of the provider filter's list on a phone.
history-filter-title = 프로바이더로 필터
# Accessible name of the provider filter; $provider is the choice in effect.
history-filter-label = 필터: { $provider }
# The header's refresh button: its tooltip and accessible name.
history-refresh = 세션 새로고침
# Shown while the first sessions are read.
history-loading = 최근 세션을 불러오는 중...
# The machine has no agent that can list its sessions.
history-no-providers = 가져올 수 있는 프로바이더가 활성화되어 있지 않습니다.
# One agent's sessions could not be read; a Retry button follows.
history-failed-provider = { $provider } 세션을 불러올 수 없습니다
# The button that reads one failed agent again. Keep short.
history-retry = 다시 시도
# The pressed session could not be imported.
history-failed-import = 선택한 세션을 가져올 수 없습니다.
# The button after the list that shows more sessions. Keep short.
history-more = 더 보기
# A listed session without a title.
history-untitled = 제목 없는 세션
# The pressed row while its session is imported. Keep short.
history-importing = 가져오는 중...
# Empty list: nothing on the machine to import.
history-empty-recent = 가져올 최근 세션이 없습니다.
# Empty list: the search matches nothing.
history-empty-matches = 검색과 일치하는 세션이 없습니다.
# Empty list: every listed session is already in the app.
history-empty-imported = 최근 세션이 모두 이미 가져와졌습니다.
# Empty list: the filtered agent has no sessions.
history-empty-provider = { $provider } 세션을 찾을 수 없습니다.

# Import failure: the machine is not connected.
history-offline = 이 머신에 연결하여 대화 내역을 불러오세요.
# Import failure: the request could not be sent.
history-unsent = 요청을 보내지 못했습니다. 다시 시도하세요.
# Import failure: the machine is too old to list sessions (Paseo updateHost).
history-unsupported = 세션을 가져오려면 호스트를 업데이트하세요.
# Import failure: no answer in time.
history-timeout = 대화 목록 응답 시간이 초과되었습니다. 새로고침하여 다시 시도하세요.
# Import failure: the import may have happened.
history-uncertain = 가져오기가 완료되었을 수 있습니다. 다시 시도하기 전에 대화 목록을 새로고침하세요.
# Import failure: the machine refused.
history-refused = 머신에서 요청을 거부했습니다. 새로고침하여 다시 시도하세요.
# Import failure: not allowed on this connection.
history-forbidden = 이 연결에는 대화를 가져올 권한이 없습니다.
# Import failure: already open elsewhere.
history-conflict = 이 대화는 이미 다른 세션에서 열려 있습니다. 새로고침하여 확인하세요.
# Import failure: the machine answered nonsense.
history-invalid-reply = 머신에서 잘못된 응답을 보냈습니다. 다시 시도하기 전에 새로고침하세요.
# Import failure: the app ran out of request numbers.
history-exhausted = 요청 식별자를 모두 사용했습니다. 앱을 다시 여세요.
# Import guard when a saved machine name points to a different node.
history-identity-changed = 머신의 식별 정보가 바뀌었습니다. 대화 목록을 새로고침한 뒤 가져오세요.
# A finite retained-history or batch admission limit.
history-capacity = 폴더 범위를 좁히거나 한 번에 가져올 대화를 줄여 주세요.
