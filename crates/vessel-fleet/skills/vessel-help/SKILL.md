---
name: vessel-help
description: Explain and troubleshoot the affected Vessel installation using its current documentation and nonsecret diagnostics.
---

# Vessel Help

Answer the user's question directly. Use documentation for the installed
version and link the relevant source when available; do not replace an answer
with an instruction to read a manual. Prefer current evidence over memory.
Use the vessel skill when the task is to operate sessions rather than explain
the product.

Identify both the affected node and how the client reaches it. A desktop app
can connect to several nodes, and its local CLI may describe a different one.
Establish whether the node is desktop-managed, standalone or containerized.
Paths, provider binaries, credentials and workspaces belong to that runtime;
client version and client logs belong to the viewing device.

Start with the smallest relevant read-only check: current version, node
identity/status, connection state, provider availability, or a specific
sanitized log. Use reported paths and addresses rather than assuming a home,
port, process owner or container UID. Check the account and environment that
actually run the affected service.

Separate observed facts, source-derived behavior and unverified possibilities.
A missing capability or typed refusal is useful evidence; do not invent an RPC
or diagnose it as a successful workflow. Preserve secrets and customer data
when requesting or reporting diagnostics.

A restart can end the agent doing the diagnosis. Do not restart services, edit
runtime configuration, publish a listener, update software or change access
unless that action is within the user's authorized task. When a change is
needed, prepare a concrete narrow change and verification using the correct
service account. Verify the affected user flow before calling it fixed.
