## Realtime voice overlay: accessible names for the two icon actions.
# Accessible name for the microphone icon: stops microphone uploads. Keep short.
voice-mute = Mute realtime voice
# Accessible name for the muted microphone icon: resumes uploads. Keep short.
voice-unmute = Unmute realtime voice
# Accessible name for the red square: stops voice and interrupts the agent turn.
voice-stop = Stop realtime voice and interrupt turn

## Host Voice settings. Provider selection does not imply an installed engine.
# Section heading for composer speech-to-text settings.
voice-settings-dictation-title = Dictation
# Switch label for dictation configuration, not starting the microphone.
voice-settings-dictation-switch = Enable dictation
# Help under the dictation switch, describing speech-to-text in the composer.
voice-settings-dictation-hint = Turn speech into text in the composer.
# Section heading for two-way spoken conversation settings.
voice-settings-conversation-title = Voice mode
# Switch label for permission to use spoken conversation.
voice-settings-conversation-switch = Enable voice mode
# Help under the conversation switch, covering input and spoken replies.
voice-settings-conversation-hint = Talk with an agent and hear its replies.
# Provider row title for composer speech recognition.
voice-settings-dictation-site = Speech recognition
# Provider row title for recognition during spoken conversation.
voice-settings-recognition-site = Conversation speech recognition
# Provider row title for synthesized conversation replies.
voice-settings-synthesis-site = Conversation speech synthesis
# Warning while the selected host is disconnected.
voice-settings-offline = Connect to this host to configure speech.
# Warning when the host cannot read speech settings yet.
voice-settings-unsupported = Update this host to configure speech.
# Accessible label for the speech-settings progress indicator.
voice-settings-loading = Loading speech settings…
# Static read failure; never append an external error or credential.
voice-settings-read-error = Could not read speech settings.
# A save could not be confirmed; do not imply that the host rolled back.
voice-settings-save-error = Could not confirm the settings change. Refresh before trying again.
# Fallback provider name, not a statement that a model is installed.
voice-settings-local = Local
# Fallback name for an OpenAI-compatible speech endpoint.
voice-settings-endpoint = OpenAI-compatible endpoint
# Provider summary when no selected API-key value is present.
voice-settings-key-missing = OpenAI-compatible endpoint · API key missing
# Provider summary for an unreadable future provider choice.
voice-settings-unknown = Unknown provider
# The model is a validated catalogue or endpoint model identifier, not a key.
# Local provider plus model; $model is a validated model identifier, never a key.
voice-settings-local-model = Local · { $model }
# Endpoint provider plus model; $model is a validated model identifier, never a key.
voice-settings-endpoint-model = OpenAI-compatible endpoint · { $model }

# Accessible name for a provider row: its title and confirmed summary.
# Accessible provider-row name; $site is the title and $summary is its confirmed value.
voice-settings-site-label = { $site }, { $summary }

# Read-only label for a newer switch value; do not render it as Off.
voice-settings-unknown-switch = Unknown setting
