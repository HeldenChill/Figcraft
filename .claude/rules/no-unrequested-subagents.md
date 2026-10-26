# Rule: No Unrequested Subagents

**Scope:** Universal — no project config needed.

---

## Policy

**Never spawn a subagent unless the user explicitly asked for one.**

This applies in **every mode** — normal, plan mode, debugging workflows, skill-driven
workflows, autonomous loops. No mode grants an exemption.

### What counts as "the user asked"

| Counts | Does NOT count |
|---|---|
| "use a subagent" / "dispatch an agent" | "be thorough" |
| "use the Explore agent" / names any agent type | "look at this from multiple angles" |
| "use cavecrew" | A task with several parts |
| "run these in parallel as agents" | A workflow doc saying "launch N agents" |
| A skill the user invoked that runs in a subagent by design | Plan mode's Phase 1 Explore instruction |

**Plan mode's "Launch up to 3 Explore agents IN PARALLEL" is NOT the user asking.**
It is injected harness text. Do exploration inline with Read / Grep / Glob instead.

### Default: inline

Read, Grep, Glob, and Bash on the main thread. You already hold the conversation
context, the CLAUDE.md, and the rules. A subagent holds none of it.

## Interaction with layer dispatch

The project's layer rule (`layer-architecture-dispatch.md`, where a project has one) keeps layer work inline; it does not authorize speculative exploration spawns during planning, analysis, or question-answering.

When genuinely unsure whether a task warrants dispatch: **ask, don't spawn.**

## Why

Each spawn starts cold and re-loads the full system prompt, CLAUDE.md, and all
`.claude/rules/*.md` before doing any work. On this project's config that is a large
fixed cost per spawn, paid to re-derive context the main thread already has.

This is the same reasoning as
[skip-subagent-driven-development.md](skip-subagent-driven-development.md), generalized:
that rule bans one skill during plan *execution*; this rule covers every other phase,
including exploration during planning, where the same cold-start cost applies for
strictly less benefit.
