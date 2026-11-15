## The session's composer: what its controls say on a running session.
## Reference: Paseo composer/agent-controls (AgentControls): a change the
## agent does not take is an error toast; nothing is blocked during a turn.

# Error toast after a running session did not take a change of its model,
# effort, permissions or another option. $reason picks the sentence, by the
# machine's answer. The agent's own words are not shown here (they are the
# machine's, often English); a whole sentence each.
composer-option-refused = { $reason ->
    [busy] The agent can't make this change now.
    [not-offered] The agent no longer offers this choice. Pick another one.
    [read-only] This device can only view this machine, so it can't change a session.
    [gone] This session is no longer running on its machine.
    [unsupported] This agent can't change its options while the session runs.
    [unavailable] The change couldn't be sent now. Try again in a moment.
   *[other] The change didn't go through. Try again.
    }

## What an agent offers, kept and refreshed behind the list (owner word 28).
## Reference: Paseo's provider snapshot keeps the last list; it shows no
## words when a refresh fails. Ours says so once, quietly, in the list.

# A muted line at the top of the model, effort or permissions list when the
# list shown is the last one kept and asking the machine again failed. One
# sentence; the list still works.
composer-offers-stale = This list couldn't be refreshed, so it may be out of date.

# The `/` menu's one line when no command answers what is typed, or the
# agent has offered none (yet). Paseo's agentAutocomplete.noCommands.
composer-commands-empty = No commands found

# The new chat's `/` menu's one line when no session of the chosen agent on
# that machine has said its commands yet (the node reports commands only
# from a running session; the menu shows the last ones kept). One sentence.
composer-commands-unknown = This agent's commands show here once one of its sessions has run.

## The composer's attachments (track P's row c). Reference: Paseo's
## composer.attachments and message.attachments words.

# The attach button's name and its menu's title. Paseo's addAttachment.
composer-attach = Add attachment
# The attach menu's row that picks an image. Paseo's addImage.
composer-attach-image = Add image
# The attach menu's row that picks any file. Paseo's addFile.
composer-attach-file = Upload file
# An attached file's second line when its name has no extension. Paseo's
# message.attachments.file.
composer-file = File
# A pill's second line: the session's agent takes no images.
composer-file-no-images = This agent doesn't take images
# A pill's second line: the session's agent takes no audio.
composer-file-no-audio = This agent doesn't take audio
# A pill's second line: larger than the machine takes for its kind.
composer-file-too-large = Too large to send
# A pill's second line: an empty file.
composer-file-empty = The file is empty
# A pill's second line: more files than one message may carry.
composer-file-too-many = Too many files for one message
# A pill's second line: the message's images and audio are too large together.
composer-file-inline-total = Too many images for one message
# A pill's second line: an image whose bytes are not its type's.
composer-file-not-its-type = Not the image its name says
# A pill's second line: the file changed while it was sent.
composer-file-changed = The file changed while it was sent
# A pill's second line: the file could not be read on this device.
composer-file-unreadable = The file couldn't be read
# A pill's second line: the upload failed; a click tries it again.
composer-file-failed = Upload failed. Click to try again
