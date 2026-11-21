## Review comments on the Changes diff (ADR 0124). Reference: Paseo's
## review.comment words (review/surface.tsx, review/comment-sheet.tsx).

# The gutter button on a diff line. Keep short.
review-add = 리뷰 댓글 추가
# A comment's pencil button.
review-edit = 리뷰 댓글 편집
# A comment's bin button.
review-delete = 리뷰 댓글 삭제
# The editor's name and the narrow sheet's title.
review-label = 리뷰 댓글
# The empty editor.
review-placeholder = 댓글을 남기세요
# The editor's Cancel button. Keep short.
review-cancel = 취소
# Cancel's name for assistive technology.
review-cancel-accessibility = 리뷰 댓글 취소
# The inline editor's save button. Keep short.
review-save = 댓글
# The save button's name for assistive technology.
review-save-accessibility = 리뷰 댓글 저장
# The narrow sheet's save button. Keep short.
review-sheet-save = 저장
# Under the editor when the comment is past its bound; the save button is off.
review-too-long = 댓글은 16 KiB까지 쓸 수 있습니다.
# The button over the Changes list; count is the comments the review holds.
review-feedback-send = 피드백 보내기 ({ NUMBER($count) })
# The button while the review is on its way.
review-feedback-sending = 피드백 전송 중 ({ NUMBER($count) })
# The chooser's title when several agents could take the review.
review-feedback-choose = 에이전트 선택
# The toast once the review was sent.
review-feedback-sent = { $recipient }에게 피드백을 보냈습니다
# Why the button is off: no agent in this workspace.
review-feedback-no-agents = 피드백을 보내려면 이 작업 공간에서 에이전트 탭을 여세요.
# Why the button is off: the machine is not connected.
review-feedback-disconnected = 피드백을 보내려면 호스트에 연결하세요.
# Why the button is off: no comment's line is in the diff.
review-feedback-no-context = 저장된 댓글이 더 이상 이 변경 사항과 일치하지 않습니다.
# The send failed.
review-feedback-failed = 피드백 전송에 실패했습니다. 다시 시도하세요.
# The words sent to the agent with the review.
review-feedback-prompt = 이 코드 리뷰를 반영해 주세요.
