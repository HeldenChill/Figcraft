# Rule: Skip subagent-driven-development skill

**Scope:** Universal — apply to any project that already has a mechanical correctness gate.

---

## Policy

**Do NOT invoke `superpowers:subagent-driven-development`** for plan execution.

Dispatch layer agents directly instead, using whatever quality gate the project already has
(compile gate, test suite, lint CI, etc.) as the correctness floor.

## Why

The skill dispatches 3 subagents per task: implementer + spec reviewer + code quality reviewer.
Each spawns cold and re-loads the full system prompt, CLAUDE.md, and all rules before doing
any work. On a project with a heavy config (rules, memory hooks, agent files), this overhead
compounds across every task in a plan.

The two reviewer subagents exist to catch errors the implementer might miss. When the project
already has a mechanical gate that catches those errors (e.g. `dotnet build`, `pytest`, `tsc`),
the reviewers add token cost without adding accuracy — the gate is faster and more reliable.

## When the skill IS appropriate

Use `superpowers:subagent-driven-development` only when **all** of these are true:

- The plan has 5+ independent tasks
- There is no mechanical correctness gate for the work (pure docs, config-only, infra scripts)
- The spec compliance risk is high enough to justify the reviewer cost

## Replacement workflow

1. Dispatch the responsible agent directly (per the project's layer/agent dispatch rule).
2. Provide the full brief as required by that project's dispatch contract.
3. Rely on the project's mechanical gate (compile, test, lint) as the quality floor.
4. Read the diff and verify manually for judgment-heavy changes.
