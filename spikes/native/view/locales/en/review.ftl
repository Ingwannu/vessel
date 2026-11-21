## Review comments on the Changes diff (ADR 0124). Reference: Paseo's
## review.comment words (review/surface.tsx, review/comment-sheet.tsx).

# The gutter button on a diff line. Keep short.
review-add = Add review comment
# A comment's pencil button.
review-edit = Edit review comment
# A comment's bin button.
review-delete = Delete review comment
# The editor's name and the narrow sheet's title.
review-label = Review comment
# The empty editor.
review-placeholder = Leave a comment
# The editor's Cancel button. Keep short.
review-cancel = Cancel
# Cancel's name for assistive technology.
review-cancel-accessibility = Cancel review comment
# The inline editor's save button. Keep short.
review-save = Comment
# The save button's name for assistive technology.
review-save-accessibility = Save review comment
# The narrow sheet's save button. Keep short.
review-sheet-save = Save
# Under the editor when the comment is past its bound; the save button is off.
review-too-long = A comment can hold at most 16 KiB.
# The button over the Changes list; count is the comments the review holds.
review-feedback-send = Send feedback ({ NUMBER($count) })
# The button while the review is on its way.
review-feedback-sending = Sending feedback ({ NUMBER($count) })
# The chooser's title when several agents could take the review.
review-feedback-choose = Choose an agent
# The toast once the review was sent.
review-feedback-sent = Feedback sent to { $recipient }
# Why the button is off: no agent in this workspace.
review-feedback-no-agents = Open an agent tab in this workspace to send feedback.
# Why the button is off: the machine is not connected.
review-feedback-disconnected = Connect to the host to send feedback.
# Why the button is off: no comment's line is in the diff.
review-feedback-no-context = Saved comments no longer match this diff.
# The send failed.
review-feedback-failed = Failed to send feedback. Try again.
# The words sent to the agent with the review.
review-feedback-prompt = Please address this code review.
