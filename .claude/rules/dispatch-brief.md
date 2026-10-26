# Rule: Dispatch Brief Contract

**Scope:** Universal — no project config needed.

---

## Policy

**Applies only when the user explicitly asks for an agent.** Default is inline (`layer-architecture-dispatch.md`, `no-unrequested-subagents.md`). A spawned generic agent (`general-purpose`, `Explore`) starts cold: its brief is all it knows.

Every brief contains these blocks:

```markdown
## Task
[What to do, acceptance criteria, decisions already made — authoritative, do not re-litigate.]

## Read first
[The exact memory/doc files that hold the context, e.g. `.cursor/memory/mem-<topic>.md`.]

## Constraints
[Layer direction high→low; `.asmdef` edits need user approval; no prefab/scene edits —
agents have no Unity MCP; files the agent must not touch.]

## Model
[Tier per model-routing.md + one-line reason.]

## Report contract
[What to return. For code: one `Build evidence: <Assembly> — 0 errors (<N> warnings)` line per
built assembly (`pkg-framework-compile-verification.md`). For search: file:line list only.]
```

## Why

Generic agents know nothing about this project. Naming the memory to read and the boundaries to keep turns silent wrong assumptions into cheap, visible questions. The compile gate catches a missing asmdef reference, so no reference matrix is required.
