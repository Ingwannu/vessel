## Settings › Models: configured model routes and explicit profile use.
## Reference: provider-diagnostic-sheet.tsx; native standalone Settings page.

# Heading above configured model rows; short. No values.
models-routes = Configured models

# Search field placeholder, fields are route name/model/provider. No values.
models-search = Search models

# Refresh the selected machine's Models snapshot; short. No values.
models-refresh = Refresh

# Initial failed-read action; short. No values.
models-retry = Retry

# Refresh action while its read is pending; short. No values.
models-refreshing = Refreshing…

# Initial retry action while its read is pending; short. No values.
models-retrying = Retrying…

# Initial model read, beside the shared small spinner. No values.
models-loading = Loading models…

# Successful model list with no configured routes. No values.
models-empty = No models are configured on this machine.

# A nonempty search with no matching configured models. No values.
models-no-matches = No models match your search.

# No selected machine; no read is sent. No values.
models-no-machine = Choose a machine to see its models.

# Explains the scope above the model rows. No values.
models-read-only = Models and configured profile use reported by this machine. Edit their configuration on the machine.

# Label before the profiles explicitly using this route. No values.
models-used-by = Used by profiles


# The model list exists but profile.list is unsupported. No values.
models-usage-unavailable = Update Vessel on this machine to see which profiles use each model.

# Label for the node-reported smaller model identifier; short. No values.
models-small = Small model

# Failed Models read: complete sentence, no machine diagnostic. Refresh reference names the real control.
models-problem-offline = The machine is not connected. Connect it, then select “{ models-refresh }”.

# Failed Models read: complete sentence, no machine diagnostic. Refresh reference names the real control.
models-problem-busy = Vessel is busy. Wait a moment, then select “{ models-refresh }”.

# Failed Models read: complete sentence, no machine diagnostic. Refresh reference names the real control.
models-problem-timeout = The machine did not finish reading models in time. Select “{ models-refresh }” to try again.

# Failed Models read: complete sentence, no machine diagnostic. Refresh reference names the real control.
models-problem-no-transport = This browser cannot connect to a machine yet.

# Failed Models read: complete sentence, no machine diagnostic. Refresh reference names the real control.
models-problem-denied = This device cannot read models on this machine. Ask its administrator for access.

# Failed Models read: complete sentence, no machine diagnostic. Refresh reference names the real control.
models-problem-unsupported = Update Vessel on this machine to read its configured models.

# Failed Models read: complete sentence, no machine diagnostic. Refresh reference names the real control.
models-problem-failed = The machine could not read its models. Select “{ models-refresh }” to try again.

# Models read refused before dispatch: no selected machine. Complete sentence.
models-start-no-machine = Choose a machine to see its models.

# Models read refused because its previous request is pending. Complete sentence.
models-start-busy = The current model read is still running. Wait for it to finish.

# Models request identities cannot wrap. Complete restart instruction.
models-start-exhausted = Model requests have reached their limit. Close and reopen Vessel.

# Public API product name for the machine-reported schema. Keep short.
models-api-anthropic = Anthropic Messages

# Public API product name for the machine-reported schema. Keep short.
models-api-responses = OpenAI Responses

# Public API product name for the machine-reported schema. Keep short.
models-api-chat = OpenAI Chat Completions
