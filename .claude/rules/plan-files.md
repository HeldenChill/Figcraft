---
description: Write specs to `.cursor/plans/design/` and step-by-step plans to `.cursor/plans/implement/` as `YYYY-MM-DD-<name>.md`, never to `docs/superpowers/` or the default Claude plans folder
---
# Rule: Plan Files

**Scope:** Universal — update the PROJECT CONFIG block when moving to a new project.

---

## PROJECT CONFIG
<!-- Update this value when copying to a new project -->

```
PLAN_DIR = .cursor/plans
```

---

## Policy (universal — do not change)

Plans live in two subfolders of `<project-root>/<PLAN_DIR>/`:

| Subfolder | What goes there | Typical source |
|---|---|---|
| `design/` | specs, discussion and design documents, decision records | `superpowers:brainstorming`, design reviews |
| `implement/` | step-by-step implementation plans an executor follows | `superpowers:writing-plans` |

- Name every new file `YYYY-MM-DD-<descriptive-name>.md` (kebab-case, today's date). Do not add a `-design` or `-implementation` suffix: the subfolder already says it.
- This **overrides the Superpowers defaults** `docs/superpowers/specs/` and `docs/superpowers/plans/`: when a skill says to save there, save to `design/` or `implement/` instead.
- Do NOT write plans to the default Claude plans folder (`~/.claude/plans/`; on Windows `%USERPROFILE%\.claude\plans\`), and do not write new files flat in `<PLAN_DIR>/`.

## Naming convention

Use a short descriptive kebab-case name that describes the change — not a generated UUID:

- `2026-10-08-fish-collected-state-refactor.md` ✅
- `crystalline-honking-aurora.md` ❌

## Why

- Plans saved inside the project are versioned with the code, visible in the IDE, and shareable.
- The default Claude plans folder is outside the repo — writing there first wastes tokens and requires a second copy.
- Splitting design from implementation lets a reader find the decision record or the executable plan without opening both.
