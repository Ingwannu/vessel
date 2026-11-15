## New session/workspace creation, shown inline while the draft is pending.
# Provisioning progress. Keep the whole sentence together.
creation-preparing-workspace = Preparing the workspace…
# Agent or shell registration progress.
creation-creating-resource = Starting the session…
# Registration succeeded; first-message delivery is a separate stage.
creation-sending-first-message = Sending the first message…
# No command was sent; retry is safe.
creation-queue-full = Could not send yet. Try again when the app is ready.
# The node has no required creation capability.
creation-unsupported = This machine does not support this creation flow yet.
# No side effect was sent to an offline node.
creation-offline = Connect to the selected machine and try again.
# An explicit node refusal; raw diagnostics are not interpolated here.
creation-refused = The machine refused this step. Your draft has been kept.
# Side effect may have committed; never invite blind resubmission.
creation-outcome-unknown = The connection ended before the result was confirmed. Check the existing session before starting another.

# Registration is retained after a known first-message refusal.
creation-registered-retry = The session was created. Retry the first message in that session.
# Retries delivery to the already-created session.
creation-retry-first-message = Retry first message

# Workspace launch and primary action; empty Chat creates only a workspace.
creation-create-workspace = Create workspace

# Launch target: a chat, or only a workspace when there is no payload.
creation-launch-chat = Chat

# Launch target: the node configured shell, distinct from a stored profile.
creation-launch-shell = Blank terminal
