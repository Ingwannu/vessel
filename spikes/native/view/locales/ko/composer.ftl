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

# 새 채팅의 `/` 메뉴에서, 고른 에이전트의 세션이 그 기기에서 아직 한 번도
# 명령을 알리지 않았을 때의 한 줄(노드는 실행 중인 세션에서만 명령을
# 알리고, 메뉴는 마지막으로 저장해 둔 명령을 보인다). 한 문장.
composer-commands-unknown = 이 에이전트의 세션을 한 번 실행하면 여기에 명령이 나타납니다.

## 작성창의 첨부(트랙 P의 c 행). 참고: Paseo의 composer.attachments와
## message.attachments 문구.

# 첨부 버튼의 이름이자 메뉴 제목. Paseo의 addAttachment.
composer-attach = 첨부 추가
# 첨부 메뉴에서 이미지를 고르는 줄. Paseo의 addImage.
composer-attach-image = 이미지 추가
# 첨부 메뉴에서 아무 파일이나 고르는 줄. Paseo의 addFile.
composer-attach-file = 파일 업로드
# 이름에 확장자가 없는 첨부 파일의 둘째 줄. Paseo의 message.attachments.file.
composer-file = 파일
# 첨부의 둘째 줄: 세션의 에이전트가 이미지를 받지 않음.
composer-file-no-images = 이 에이전트는 이미지를 받지 않습니다
# 첨부의 둘째 줄: 세션의 에이전트가 오디오를 받지 않음.
composer-file-no-audio = 이 에이전트는 오디오를 받지 않습니다
# 첨부의 둘째 줄: 기기가 그 종류로 받는 크기보다 큼.
composer-file-too-large = 보내기에 너무 큽니다
# 첨부의 둘째 줄: 빈 파일.
composer-file-empty = 빈 파일입니다
# 첨부의 둘째 줄: 메시지 하나에 담을 수 있는 수보다 많음.
composer-file-too-many = 메시지 하나에 담기에 파일이 너무 많습니다
# 첨부의 둘째 줄: 메시지의 이미지·오디오를 합치면 너무 큼.
composer-file-inline-total = 메시지 하나에 담기에 이미지가 너무 많습니다
# 첨부의 둘째 줄: 이름과 다른 형식의 이미지.
composer-file-not-its-type = 이름과 형식이 다른 이미지입니다
# 첨부의 둘째 줄: 보내는 동안 파일이 바뀜.
composer-file-changed = 보내는 동안 파일이 바뀌었습니다
# 첨부의 둘째 줄: 이 기기에서 파일을 읽지 못함.
composer-file-unreadable = 파일을 읽지 못했습니다
# 첨부의 둘째 줄: 업로드 실패, 누르면 다시 시도.
composer-file-failed = 업로드하지 못했습니다. 누르면 다시 시도합니다
