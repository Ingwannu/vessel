## 대화 분기. Paseo의 assistant-fork-menu와 use-fork-agent를 참고합니다.
## Vessel은 초안 탭 대신 원본 옆에 새 세션을 만듭니다.

# 답변 아래 분기 버튼의 툴팁. 짧게 씁니다.
fork-tooltip = 여기에서 채팅 분기
# 노드가 지원하는 한 가지 분기 대상. 짧게 씁니다.
fork-in-new-session = 새 세션으로 분기
# 거절된 분기. 이관 중에만 진단 내용을 유지합니다.
fork-failed =
    { $kind ->
        [plain] 채팅을 분기하지 못했습니다
       *[detail] 채팅을 분기하지 못했습니다: { $message }
    }
# 대화가 시작되지 않은 분기의 안내 제목.
fork-empty-heading = 분기가 시작되지 않았습니다
# 에이전트가 고른 지점을 무시하여 이어 갈 수 없습니다.
fork-empty-not-at-point = 에이전트가 고른 지점에서 분기하지 않았습니다. 여기서는 이어 갈 수 없으니 삭제하고 다시 분기하세요.
# 분기를 지원하지 않아 이어 갈 수 없습니다.
fork-empty-cannot-fork = 이 에이전트는 대화를 분기할 수 없습니다. 여기서는 이어 갈 수 없습니다.
# 원본은 남겨 두고 빈 분기만 삭제합니다. 짧게 씁니다.
fork-remove = 삭제
# 원본 읽기 실패. 각 선택지는 복구 안내까지 한 문장으로 씁니다.
fork-origin-unread =
    { $kind ->
        [removed] 분기한 원래 대화는 삭제되었습니다.
        [plain] 분기한 원래 대화를 읽지 못했습니다. 세션을 다시 열면 다시 읽습니다.
       *[detail] 분기한 원래 대화를 읽지 못했습니다: { $message }. 세션을 다시 열면 다시 읽습니다.
    }
# 분기의 첫 줄. title은 원본 세션의 제목입니다.
fork-from-title = “{ $title }”에서 분기함
# 원본 제목을 모르거나 제목이 비어 있을 때의 첫 줄.
fork-from-other = 다른 세션에서 분기함
