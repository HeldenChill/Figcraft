# Rule: Superpowers Framework & Skill Orchestration

## Core Policy for Skill Usage

1. **Invoke before acting**: If any specialized skill applies (e.g., brainstorming, systematic-debugging, writing-plans, test-driven-development), consult and follow its steps before writing code or modifying assets.
2. **Follow disciplined steps**: Follow workflow steps systematically rather than rushing into code changes.
3. **One skill at a time**: Execute multi-stage workflows sequentially.
4. **Questions before assumptions**: When requirements or edge cases are unclear, clarify requirements first.
5. **No subagent fan-out**: Execute plans inline (`executing-plans`). Do not spawn subagents unless the user explicitly asks; `subagent-driven-development` and `dispatching-parallel-agents` are intentionally not shipped.

## Skill → Task Mapping

| Task | Skill | Location |
|---|---|---|
| New feature or substantial refactor | `brainstorming` | `.agents/skills/brainstorming/` |
| Bug with unclear root cause / exception | `systematic-debugging` | `.agents/skills/systematic-debugging/` |
| Design → step-by-step implementation plan | `writing-plans` | `.agents/skills/writing-plans/` |
| Execute an approved plan | `executing-plans` | `.agents/skills/executing-plans/` |
| Unit tests first | `test-driven-development` | `.agents/skills/test-driven-development/` |
| Before claiming work is done | `verification-before-completion` | `.agents/skills/verification-before-completion/` |
| Asking for / acting on review | `requesting-code-review`, `receiving-code-review` | `.agents/skills/` |
| Isolated workspace / wrapping up a branch | `using-git-worktrees`, `finishing-a-development-branch` | `.agents/skills/` |
| Which skill applies? | `using-superpowers` | `.agents/skills/using-superpowers/` |
| Concise, low-token communication mode | `caveman` | `.agents/skills/caveman/` |
