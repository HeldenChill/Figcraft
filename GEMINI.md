# Workspace Guidelines (Gemini & Antigravity)

Synced from claude-unity-shared — do not edit here; edit the shared source and pull.

## Project context

- Read `AGENTS.md` at the project root at the start of each session: rule registry, architecture orientation, memory index pointer.
- Project rules live in `.claude/rules/` (one file per rule, listed in the `AGENTS.md` table) and apply to you too.
- **Memory first:** before reading source for a topic, grep the project's memory index (the `## Memory index` table of the canonical architecture memory) and read the matching `.cursor/memory/` file. Code wins over memory when they disagree.

```
grep -i "<keyword>" .cursor/memory/mem-arch-assemblies-canonical.md
```

## Skills & Superpowers Workflow

Skills live in `.agents/skills/`, rules in `.agents/rules/`. Skill-to-task mapping: `.agents/rules/superpowers-setup.md`.

1. **Invoke Before Acting**:
   - **New feature / Refactor**: `brainstorming` (`.agents/skills/brainstorming/SKILL.md`).
   - **Bugs / Exceptions**: `systematic-debugging` (`.agents/skills/systematic-debugging/SKILL.md`).
   - **Writing Plans**: `writing-plans` (`.agents/skills/writing-plans/SKILL.md`).
   - **Plan Execution**: `executing-plans` (`.agents/skills/executing-plans/SKILL.md`).
   - **Test-Driven Work**: `test-driven-development` (`.agents/skills/test-driven-development/SKILL.md`).
   - **Before Claiming Done**: `verification-before-completion` (`.agents/skills/verification-before-completion/SKILL.md`).
   - **Superpowers Entrypoint**: `using-superpowers` (`.agents/skills/using-superpowers/SKILL.md`).
2. **Plans & Specs Storage** (`.agents/rules/plan-files.md`):
   - Save design specs to `.cursor/plans/design/YYYY-MM-DD-<name>.md`.
   - Save implementation plans to `.cursor/plans/implement/YYYY-MM-DD-<name>.md`.

## Caveman Mode

When requested by the user (`caveman`, `/caveman`, or asking for extreme brevity), follow `.agents/skills/caveman/SKILL.md`:
- Cut all filler words, preamble, greetings, and pleasantries.
- Provide direct answers and telegraphic phrasing.
- Keep all code snippets, paths, commands, and compiler logs 100% exact and intact.
