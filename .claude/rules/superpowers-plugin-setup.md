---
description: Invoke the matching Superpowers skill before acting (brainstorming, systematic-debugging, writing-plans), one skill at a time
---
# Rule: Superpowers Plugin Setup

**Scope:** Universal — no project config needed.

---

## Hard Rules for Skill Usage

1. **Invoke before acting** — If any skill might apply (even 1% chance), invoke it before writing code, making changes, or taking actions.
2. **Follow exactly** — Skills are rigid workflows. Adapt only if the skill itself says so.
3. **One skill at a time** — Invoke sequentially, not in parallel.
4. **Skills before clarifying questions** — Even exploratory questions check for skills first.

## Skill → Task mapping

| Task | Skill |
|------|-------|
| New feature or refactor | `brainstorming` |
| Bug with unclear root cause | `systematic-debugging` |
| Design → code | `writing-plans` |
| Tests first | `tdd-workflow` |

Skills are loaded via the `Skill` tool. Use `claude plugin install claude-plugins-official/superpowers` to install.
