## 세션 작성란: 실행 중인 세션에서 컨트롤이 보여 주는 문구.

# 실행 중인 세션이 모델, 노력, 권한 등의 변경을 받아들이지 않았을 때의 오류 토스트.
composer-option-refused = { $reason ->
    [busy] 에이전트가 지금은 이 변경을 할 수 없습니다.
    [not-offered] 에이전트가 더 이상 이 선택지를 제공하지 않습니다. 다른 것을 고르세요.
    [read-only] 이 기기는 이 머신을 보기만 할 수 있어 세션을 바꿀 수 없습니다.
    [gone] 이 세션은 머신에서 더 이상 실행되고 있지 않습니다.
    [unsupported] 이 에이전트는 세션이 실행되는 동안 옵션을 바꿀 수 없습니다.
    [unavailable] 지금은 변경을 보낼 수 없습니다. 잠시 후 다시 시도하세요.
   *[other] 변경이 적용되지 않았습니다. 다시 시도하세요.
    }

## What an agent offers, kept and refreshed behind the list (owner word 28).

# 목록이 저장해 둔 마지막 목록이고, 기기에 다시 묻지 못했을 때 목록 맨 위에
# 흐리게 보이는 한 줄. 목록은 그대로 쓸 수 있다.
composer-offers-stale = 이 목록을 새로 받지 못해 최신이 아닐 수 있습니다.

# `/` 메뉴에서 입력한 글자에 맞는 명령이 없거나, 에이전트가 아직 명령을
# 알리지 않았을 때의 한 줄. Paseo의 agentAutocomplete.noCommands.
composer-commands-empty = 명령을 찾을 수 없습니다
