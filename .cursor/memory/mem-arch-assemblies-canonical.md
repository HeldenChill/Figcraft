# FigCraft architecture and memory index

Last reviewed: 2026-10-26. Project-owned; shared sync must preserve this file.
The filename is the shared hook's compatibility convention. This project has Rust
crates, not Unity assemblies. Source and root AGENTS.md govern architecture.

FigCraft is a clean-room interface design and prototyping application using Rust
and egui. Apps under apps/ provide desktop, CLI, collaboration server and browser
entry points. All editing surfaces dispatch engine commands. Read ROADMAP.md and
docs/gaps.md at session start; plan/ holds optional local evidence and notes.

Use the index to find a topic before broad source exploration. Memories record
dated observations, not current runtime guarantees. Recheck tools, Git state,
running processes and upstream versions when needed. Do not record credentials
or treat historical process IDs as durable identifiers.

## Memory index

| Topic | File |
|---|---|
<!-- LOCAL-MEMORY-INDEX:BEGIN -->
| HUB: Project architecture, features, status and source authority | `mem-figcraft-project.md` |
| HUB: Windows environment, setup evidence and unresolved access | `mem-figcraft-windows-setup.md` |
| HUB: MCP automation, plugin installation and Swatch Grid usage | `mem-figcraft-mcp-plugins.md` |
| HUB: Figma import, renderer crash workaround, four plugins and Gemini handoff | `mem-figcraft-import-render-and-plugin-handoff.md` |
| HUB: Confirmed bugs and lifecycle evidence by domain | `bugs/README.md` |
| Session record: 2026-10-26 requests, decisions and lessons | `mem-session-2026-10-26.md` |
| Shared AI config ownership, sync mechanics and generic policies | `mem-shared-ai-config.md` |
<!-- LOCAL-MEMORY-INDEX:END -->

## Operating references

- Project instructions: AGENTS.md and CLAUDE.md at the repository root.
- Knowledge: docs/, ROADMAP.md, docs/gaps.md and Cargo.toml.
- Procedures: .cursor/playbooks/README.md.
- Import diagnosis and plugin execution handoff: .cursor/playbooks/figcraft-import-and-plugin-handoff.md.
- Workstation-specific evidence: plan/SETUP.md and plan/setup/ (gitignored).
- Memory workflow: .claude/rules/memory-first-workflow.md.
