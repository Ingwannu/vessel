---
name: vessel-plugin
description: Build or manage an explicitly requested Vessel plugin using the installed API, compatibility and trust contract.
---

# Vessel Plugin

Identify the requested contribution and read the plugin documentation for the
actual app and node versions. Confirm which supported contribution point owns
it: a user-interface surface, command, attachment, timeline contribution,
configuration/launch hook, lifecycle observer or plugin-specific backend work.
Use only APIs and capabilities that the installation actually offers.

Before writing code, inspect existing plugin modules and examples. Reuse the
supported SDK and shared typed request/result contracts. Keep client, node and
shared responsibilities separate; client code must not obtain node filesystem
or credential access through an undocumented import or fallback.

Record the plugin identity, compatible versions, entry points, requested
capabilities and lifecycle behavior in its repository. Include the purpose,
trust boundary, cancellation/shutdown behavior, failure handling and how to
verify the contribution. Keep secrets out of manifests, logs and source.

Use an isolated project and synthetic data for development. Check the actual
supported install/build/reload commands rather than guessing a CLI or RPC.
Do not turn a missing contribution point into permission to patch the running
host or inject an undocumented runtime hook. Report that limitation and the
work needed from the owning API.

Test only the contribution the task authorizes. Preserve existing plugin
configuration and dirty work. Present the concrete artifact, capability needs,
checks and remaining limitations. Apply any required trust approval at the
normal installation boundary; an agent cannot grant itself capabilities.
After installation or reload, verify the actual contribution and cleanup,
including which process/account owns backend effects. A successful build
alone is not proof that a user interface or lifecycle hook ran.
