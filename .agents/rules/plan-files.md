# Rule: Plan Files & Specification Storage

## Location Policy

All architecture documents, design specs, and step-by-step implementation plans must be stored under `.cursor/plans/`:

| Subfolder | What goes there | Typical source |
|---|---|---|
| `.cursor/plans/design/` | Specs, architecture decisions, and design reviews | `brainstorming` |
| `.cursor/plans/implement/` | Step-by-step executable implementation plans | `writing-plans` |

## Naming Convention

- Format: `YYYY-MM-DD-<kebab-case-descriptive-name>.md`
- Example: `2026-10-22-player-inventory-refactor.md`
- Do not use random IDs or UUIDs.
