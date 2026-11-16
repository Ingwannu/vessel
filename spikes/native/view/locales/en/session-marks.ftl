# Directory-only feedback; no panel stop or automatic replay is implied.
# Another directory update is still pending; do not imply an automatic replay.
session-marks-busy = Another session update is pending. Wait for its result before continuing.
# The original connection admission changed; refresh before further action.
session-marks-changed = The session connection changed. Refresh to check the result before continuing.
# A dispatched mutation may already have completed; never promise definite failure.
session-marks-uncertain = The update may have completed. Refresh to check its result before trying again.
# Definite local rejection, such as missing transport or an invalid original read boundary.
session-marks-unavailable = Session updates are unavailable on this connection. Refresh before trying again.
