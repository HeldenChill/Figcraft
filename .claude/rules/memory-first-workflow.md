---
description: Discover and maintain dated FigCraft project memories and operating playbooks.
---

# FigCraft memory workflow

Before broad source exploration, search the Memory index in
.cursor/memory/mem-arch-assemblies-canonical.md and read the matching topic.
Use rg for topic searches. Consult .cursor/playbooks/README.md for procedures.
The canonical filename is required by the shared hook; FigCraft has Rust crates.

Source code, current project instructions and fresh observations resolve stale
memory. Record date, scope, source/evidence and unverified limits. Refresh machine
state before relying on tools, versions, processes, file paths or authentication.
Do not claim that a historical passing check is a new verification result.

Keep knowledge in project-owned mem-*.md files. Register each exactly once in
the canonical two-column Memory index between LOCAL-MEMORY-INDEX sentinels.
The canonical file itself has no row. HUB rows expose topic routing through the
Claude prompt hook. Keep procedures in project-owned playbooks, indexed by their
README. Do not edit sync-owned memories or playbooks to add local experience.

After edits, run the shared AI index checker and exercise the memory hook.
Keep credentials and unstable process IDs out of durable memory. Local evidence
under plan/ is gitignored and can be missing on another checkout.
