---
name: vessel-plugin
description: Develop explicitly requested Vessel plugins within the installed plugin contract and trust boundary.
---

Read the installed plugin API and trust policy before designing a plugin.
Use only documented contribution points and capabilities advertised by the
running node. Record lifecycle, permissions, failure handling and ownership
in the repository. Test against synthetic data in an isolated home. Present
the concrete plugin and its checks before requesting any required trust
approval. An unavailable plugin API is a typed limitation, not permission to
modify the host or substitute an undocumented runtime hook.
