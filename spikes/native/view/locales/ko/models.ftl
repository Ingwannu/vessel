## Settings › Models: configured model routes and explicit profile use.
## Reference: provider-diagnostic-sheet.tsx; native standalone Settings page.

# Heading above configured model rows; short. No values.
models-routes = 구성된 모델

# Search field placeholder, fields are route name/model/provider. No values.
models-search = 모델 검색

# Refresh the selected machine's Models snapshot; short. No values.
models-refresh = 새로 고침

# Initial failed-read action; short. No values.
models-retry = 다시 시도

# Refresh action while its read is pending; short. No values.
models-refreshing = 새로 고치는 중…

# Initial retry action while its read is pending; short. No values.
models-retrying = 다시 시도하는 중…

# Initial model read, beside the shared small spinner. No values.
models-loading = 모델을 읽는 중…

# Successful model list with no configured routes. No values.
models-empty = 이 머신에 구성된 모델이 없습니다.

# A nonempty search with no matching configured models. No values.
models-no-matches = 검색과 일치하는 모델이 없습니다.

# No selected machine; no read is sent. No values.
models-no-machine = 모델을 볼 머신을 선택하세요.

# Explains the scope above the model rows. No values.
models-read-only = 이 머신이 알려준 모델과 프로필 사용 정보입니다. 구성은 머신에서 변경하세요.

# Label before the profiles explicitly using this route. No values.
models-used-by = 사용 중인 프로필


# The model list exists but profile.list is unsupported. No values.
models-usage-unavailable = 모델을 사용하는 프로필을 보려면 이 머신의 Vessel을 업데이트하세요.

# Label for the node-reported smaller model identifier; short. No values.
models-small = 작은 모델

# Failed Models read: complete sentence, no machine diagnostic. Refresh reference names the real control.
models-problem-offline = 머신에 연결되지 않았습니다. 연결한 뒤 “{ models-refresh }” 버튼을 누르세요.

# Failed Models read: complete sentence, no machine diagnostic. Refresh reference names the real control.
models-problem-busy = Vessel이 요청을 처리하는 중입니다. 잠시 후 “{ models-refresh }” 버튼을 누르세요.

# Failed Models read: complete sentence, no machine diagnostic. Refresh reference names the real control.
models-problem-timeout = 머신이 제한 시간 안에 모델을 읽지 못했습니다. 다시 시도하려면 “{ models-refresh }” 버튼을 누르세요.

# Failed Models read: complete sentence, no machine diagnostic. Refresh reference names the real control.
models-problem-no-transport = 이 브라우저에서는 아직 머신에 연결할 수 없습니다.

# Failed Models read: complete sentence, no machine diagnostic. Refresh reference names the real control.
models-problem-denied = 이 기기에는 이 머신의 모델을 읽을 권한이 없습니다. 관리자에게 접근 권한을 요청하세요.

# Failed Models read: complete sentence, no machine diagnostic. Refresh reference names the real control.
models-problem-unsupported = 구성된 모델을 읽으려면 이 머신의 Vessel을 업데이트하세요.

# Failed Models read: complete sentence, no machine diagnostic. Refresh reference names the real control.
models-problem-failed = 머신이 모델을 읽지 못했습니다. 다시 시도하려면 “{ models-refresh }” 버튼을 누르세요.

# Models read refused before dispatch: no selected machine. Complete sentence.
models-start-no-machine = 모델을 볼 머신을 선택하세요.

# Models read refused because its previous request is pending. Complete sentence.
models-start-busy = 현재 모델을 읽는 중입니다. 읽기가 끝날 때까지 기다리세요.

# Models request identities cannot wrap. Complete restart instruction.
models-start-exhausted = 모델 요청 한도에 도달했습니다. Vessel을 닫았다가 다시 여세요.

# Public API product name for the machine-reported schema. Keep short.
models-api-anthropic = Anthropic Messages

# Public API product name for the machine-reported schema. Keep short.
models-api-responses = OpenAI Responses

# Public API product name for the machine-reported schema. Keep short.
models-api-chat = OpenAI Chat Completions
