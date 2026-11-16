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

# Lightbox status and accessible actions (Paseo message.attachments).
# Image viewer action or status; a complete label.
composer-image-loading = Loading image…
# Image viewer action or status; a complete label.
composer-image-failed = Failed to load image
# Image viewer action or status; a complete label.
composer-image-close = Close image
# Image viewer action or status; a complete label.
composer-image-zoom-in = Zoom in
# Image viewer action or status; a complete label.
composer-image-zoom-out = Zoom out
# Image viewer action or status; a complete label.
composer-image-reset = Reset zoom

# Toast when the platform file dialog is unavailable or fails.
composer-file-pick-failed = The file picker couldn't be opened

# Resume failed before upload; the composer retains its files and restores text.
composer-file-wake-failed = The session couldn't resume. Your attachments are still here.

# File and directory autocomplete status; a complete line above the composer.
composer-files-loading = Loading files…

# File and directory autocomplete status; a complete line above the composer.
composer-files-empty = No files found

# File and directory autocomplete status; a complete line above the composer.
composer-files-failed = Files could not be loaded. Try editing the mention to retry.

# Attachment pill subtitle identifying the source of a context snapshot.
composer-context-browser = Browser element

# Attachment pill subtitle identifying the source of a context snapshot.
composer-context-review = Review comments

# Attachment pill subtitle identifying the source of a context snapshot.
composer-context-history = Chat history

# Attachment pill subtitle identifying the source of a context snapshot.
composer-context-pull = Pull request context

# Attachment pill subtitle identifying the source of a context snapshot.
composer-context-issue = Issue

# Attachment pill subtitle identifying the source of a context snapshot.
composer-context-plugin = Plugin resource

# A new or background session did not provide attachment capabilities in time.
composer-file-capabilities-failed = The agent's attachment support couldn't be read. Click to retry.

# Native attachment menu action or clipboard acquisition failure; a complete label.
composer-paste-image = Paste image

# Native attachment menu action or clipboard acquisition failure; a complete label.
composer-image-clipboard-empty = There is no image on the clipboard

# Native attachment menu action or clipboard acquisition failure; a complete label.
composer-image-format-unsupported = This image format cannot be pasted yet. Choose a PNG, JPEG, GIF or WebP image.


# Context intake rejection, shared by file/browser/review/history/plugin producers.
composer-context-busy = An attachment submission is already pending


# Context intake rejection, shared by file/browser/review/history/plugin producers.
composer-context-capacity = There are too many context attachments


# Context intake rejection, shared by file/browser/review/history/plugin producers.
composer-context-invalid = This context attachment is invalid or too large


# Context intake rejection, shared by file/browser/review/history/plugin producers.
composer-context-scope = This attachment belongs to a different workspace


# Context intake rejection, shared by file/browser/review/history/plugin producers.
composer-context-unavailable = Context attachments are unavailable right now


# Failed local immutable-byte acquisition or durable manifest write.
composer-file-local-failed = The attachment could not be saved on this device. Its source was not changed.


# Recovered pending-byte retention never authorizes automatic replay.
composer-file-local-recovered = These retained attachments may belong to a message sent before restart. Sending is blocked until its delivery can be reconciled.


# Attachment delivery status or explicit retry of a request never sent.
composer-file-submission-not-sent = The message was not sent. Its prepared attachments are retained.


# Attachment delivery status or explicit retry of a request never sent.
composer-file-submission-preparing = Preparing attachments…


# Retained attachment delivery status; uncertainty never authorizes automatic retry.
composer-file-submission-retained = The attachment message is retained because delivery is unconfirmed. It will not be sent again automatically.


# Attachment delivery status or explicit retry of a request never sent.
composer-file-submission-retry = Retry this message


# Attachment delivery status or explicit retry of a request never sent.
composer-file-submission-sending = Sending the retained attachment message…
