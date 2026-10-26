---
description: FigCraft project context, Rust tooling, and applicability of imported shared AI guidance.
---

# FigCraft project context

FigCraft is a Rust 2024 workspace with an egui frontend, not a Unity project.
Read the root AGENTS.md, ROADMAP.md and docs/gaps.md for project requirements.
The command engine is in crates/engine; desktop, CLI, server and web entry points
are under apps/. Cargo.toml and source code establish the current architecture.

Shared guidance about Unity, C# assemblies, com.hung packages, Locator services,
or the character-core refactoring playbook applies only to projects containing
those systems. Do not create Unity directories or apply their layer names here.
The Unity-specific identity keys in project.config.json are compatibility inputs
for the shared sync script, not evidence that those systems exist in FigCraft.

Project knowledge is in docs/, ROADMAP.md and the optional gitignored plan/.
Session knowledge is indexed in .cursor/memory/mem-arch-assemblies-canonical.md;
the shared hook's filename is a compatibility convention, not an assembly map.
Read .claude/rules/memory-first-workflow.md and .cursor/playbooks/README.md.
Some architecture notes lag source code, particularly .fig import support.

Use cargo for builds and tests; cargo xtask ci is the repository's commit gate.
Native Windows builds use the MSVC Rust target. Browser builds use the
wasm32-unknown-unknown target and Trunk. Fonts stay in ../craft-fonts and must
never be copied into this repository. Shared standards belong in ../../craftrules
when that checkout is available.

Sync-owned files are listed in the external shared repository's manifest.json.
Keep FigCraft-specific additions in project-owned files such as this rule.
