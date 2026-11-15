## Inline permission/question response failures. Keep recovery sentences whole.
## No provider diagnostic, request content or secret is interpolated here.

# No connection existed before sending; offer reconnection.
responses-offline = Connect to this machine before answering.
# Outcome may have taken effect; require reconciliation before manual retry.
responses-unconfirmed = The answer could not be confirmed. Check whether the request still waits before trying again.
# Another answer is still pending; do not suggest another submission.
responses-busy = This request is already being answered. Wait for its result.
# The window ticket counter exhausted; reopening creates a fresh source.
responses-exhausted = Reopen this window before answering again.
# Version or capability does not support the requested answer.
responses-unsupported = This machine cannot accept this answer. Update it before trying again.
# A broken or mismatched request identity; reload the conversation.
responses-invalid = The answer did not match this request. Reload the conversation.
# Local queue refused before node work; explicit retry is safe.
responses-unsent = Could not send the answer. Try again.
# The node refused; no opaque provider diagnostic is interpolated.
responses-refused = The machine refused the answer. Check the request and try again.
