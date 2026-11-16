## Realtime voice overlay: accessible names for the two icon actions.
# Accessible name for the microphone icon: stops microphone uploads. Keep short.
voice-mute = Mute realtime voice
# Accessible name for the muted microphone icon: resumes uploads. Keep short.
voice-unmute = Unmute realtime voice
# Accessible name for the red square: stops voice and interrupts the agent turn.
voice-stop = Stop realtime voice and interrupt turn

## Host Voice settings. Provider selection does not imply an installed engine.
voice-settings-dictation-title = Dictation
voice-settings-dictation-switch = Enable dictation
voice-settings-dictation-hint = Turn speech into text in the composer.
voice-settings-conversation-title = Voice mode
voice-settings-conversation-switch = Enable voice mode
voice-settings-conversation-hint = Talk with an agent and hear its replies.
voice-settings-dictation-site = Speech recognition
voice-settings-recognition-site = Conversation speech recognition
voice-settings-synthesis-site = Conversation speech synthesis
voice-settings-offline = Connect to this host to configure speech.
voice-settings-unsupported = Update this host to configure speech.
voice-settings-loading = Loading speech settings…
voice-settings-read-error = Could not read speech settings.
voice-settings-save-error = Could not confirm the settings change. Refresh before trying again.
voice-settings-local = Local
voice-settings-endpoint = OpenAI-compatible endpoint
voice-settings-key-missing = OpenAI-compatible endpoint · API key missing
voice-settings-unknown = Unknown provider
# The model is a validated catalogue or endpoint model identifier, not a key.
voice-settings-local-model = Local · { $model }
voice-settings-endpoint-model = OpenAI-compatible endpoint · { $model }

# Accessible name for a provider row: its title and confirmed summary.
voice-settings-site-label = { $site }, { $summary }
