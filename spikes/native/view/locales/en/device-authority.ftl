## Caller authority for the selected admitted connection. No peer text.

# Neutral loading label while the selected connection’s own grant is requested.
device-authority-loading = Checking this connection’s access

# Short button: explicitly read this connection’s effective grant again.
device-authority-retry = Check access

# Known non-admin grant: one hint under Device management; never shown for a failed read.
device-authority-denied = This connection cannot manage paired devices. Use an administrator connection.


# Authority read failed because the selected admission was replaced.
device-authority-changed = Could not check access because this machine’s connection changed. Check access again.

# Authority read failed because its original transport closed.
device-authority-offline = Could not check access because this machine disconnected. Reconnect and check access again.

# The monotonic request source cannot safely allocate another identity.
device-authority-exhausted = Could not check access safely. Reopen the app and try again.

# An invalid self report is refused; no reported grant is retained.
device-authority-invalid = Could not check access because the machine returned an invalid response. Try again.

# A management control was refused locally because no admin report is current.
device-authority-not-confirmed = Management access is not confirmed for this connection. Check access before trying again.

# Old/unsupported node recovery; no compatibility admin grant is invented.
device-authority-unsupported = This machine cannot report connection permissions. Update Vessel on that machine and reconnect.

# Authenticated/authorization refusal: this query establishes no grant.
device-authority-refused = Could not check this connection’s access. Reconnect to this machine and try again.

# Temporary failure of this query; never a statement of denied access.
device-authority-unavailable = Could not check this connection’s access right now. Try again.

# Short settings row title above the known non-admin explanation.
device-authority-management = Device management

# A sent write lost its admission; no success or definite failure is inferred.
device-authority-uncertain-title = Result unconfirmed
device-authority-uncertain-invitation = Could not confirm whether a pairing link was created. It cannot be shown safely. Create a fresh link when needed.
device-authority-uncertain-revocation = Could not confirm whether access was revoked. Check the paired devices before trying again; reconnect first if needed.
device-authority-uncertain-both = Could not confirm whether the pairing link was created or access was revoked. Check the paired devices before trying again; reconnect first if needed, and create a fresh link when needed.
