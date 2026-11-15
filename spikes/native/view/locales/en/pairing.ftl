## Pairing's two dialogs, following Paseo's pair-link modal and host confirmation.
## Reference: Paseo i18n/resources/{en,ko}.ts: pairing.link,
## pairing.hostConfirmation and pairing.securityWarning.
## Paseo calls the same thing a server, daemon and host across the two steps.
## Vessel uses machine in both steps and languages, matching the sidebar,
## Add a machine and the local machine-name field.
# Paste-link dialog title. Keep short.
pairing-title = Paste pairing link
# Paste-link dialog help, above the link input.
pairing-helper = Paste the pairing link your machine printed.
# Link input label. Keep short.
pairing-label = Pairing link
# Safety explanation beneath the link input. Keep the whole warning together.
pairing-security-warning = Treat this pairing link like a password. Anyone with it can access this machine.
# Cancel either pairing dialog. Keep short.
pairing-cancel = Cancel
# Primary action after entering the link. Keep short.
pairing-pair = Pair
# Primary action while the request is running. Keep short.
pairing-busy = Pairing...
# Confirmation dialog title. Keep short.
pairing-confirm-title = Connect to this machine?
# Safety explanation above the machine's identity. Keep the whole explanation together.
pairing-confirm-description = This machine will be able to run code in this app and reach your other connected machines. Only connect if you recognize it.
# Confirmation's machine identity row label. Keep short.
pairing-machine = Machine
# Confirmation's key fingerprint row label. Keep short.
pairing-fingerprint = Key fingerprint
# Confirmation's relay row label. Keep short.
pairing-relay = Relay
# Primary action after confirming the machine. Keep short.
pairing-connect = Connect

# Link input placeholder; keep the pairing URI spelling unchanged.
pairing-link-placeholder = vessel://pair#…

# Persistent label above the local machine name field. Keep short.
pairing-name-label = Name for this machine

# Help beneath the local name field; the example is not a default value.
pairing-name-hint = A short name you will recognise, for example home-pc.

# Name field error when a fingerprint was entered instead of a name.
pairing-name-is-key = That is the key. Type a short name instead, for example home-pc.

# Name field error when a pairing link was entered instead of a name.
pairing-name-is-link = That is the pairing link. Type a short name here, or leave it empty to use the machine's own name.

# Name field error when nothing was typed.
pairing-name-empty = Type a short name, for example home-pc.

# Name field error for the node book naming rules.
pairing-name-invalid = Use letters, digits, '.', '_' and '-', starting with a letter or digit.

# Name field error when another machine already has that name.
pairing-name-taken = Another machine already has this name.

# Link field error if connecting was attempted without a decoded offer.
pairing-paste-first = Paste a pairing link first.

# Link field error from the typed decoder; no input or diagnostic is echoed.
pairing-link-wrong-kind = This is not a pairing link. Copy a pairing link from your machine and paste it here.

# Link field error from the typed decoder; no input or diagnostic is echoed.
pairing-link-encoding = This pairing link is damaged. Copy the complete pairing link from your machine again.

# Link field error from the typed decoder; no input or diagnostic is echoed.
pairing-link-offer = This pairing link could not be read. Create a new pairing link on your machine and try again.


# Connect request failure beside the confirmation form; no host diagnostic is echoed.
pairing-failed-name = This name can no longer be used. Choose another name and try again.

# Connect request failure beside the confirmation form; no host diagnostic is echoed.
pairing-failed-storage = Could not save this machine. Check that the app can write to its configuration folder, then create a new pairing link and try again.

# Connect request failure beside the confirmation form; no host diagnostic is echoed.
pairing-failed-connection = Could not connect to this machine. Check that it is running, then create a new pairing link and try again.

# Connect request failure beside the confirmation form; no host diagnostic is echoed.
pairing-failed-browser = Pairing is unavailable in this browser. Pair this machine from the desktop app.

# Connect request failure beside the confirmation form: the app did not send the
# request (its link queue was full or stopped), so the same pairing link still works.
pairing-failed-unsent = Could not send. Try again.

# Footer after a successful pairing. $name is the machine name in the sidebar;
# $via is direct or relay. The address is a separate tooltip, never part of this sentence.
pairing-paired = { $via ->
    [direct] Paired with { $name }, connected directly
   *[relay] Paired with { $name }, connected through a relay
    }

# Empty local-name input hint. Short; an empty value asks for the machine's own name.
pairing-name-placeholder = Leave empty to use its own name
