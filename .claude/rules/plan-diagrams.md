# Rule: Plan Diagrams

**Scope:** Universal — no project config needed.

---

## Policy

When writing a plan file, **prefer Mermaid diagrams over prose descriptions** wherever the plan describes:

- A data flow (how data moves between objects/systems)
- A call sequence (who calls what, in what order)
- Before/after comparisons of an architecture or flow
- Class or struct relationships (fields added, methods deleted, ownership)
- A decision tree or branching logic

### Priority order for plan content

1. **Mermaid diagram** — for anything spatial, sequential, or relational
2. **Table** — for mapping (files → changes, fields → sources, layers → owners)
3. **Bullet list** — for simple enumeration with no structure
4. **Prose paragraph** — only for nuance that diagrams and tables cannot capture (risk reasoning, design intent, edge-case explanation)

### Required diagrams in plans

Every plan that changes a data pipeline or call flow **must include**:

- A **current flow** diagram (what exists today, including what is wrong)
- A **target flow** diagram (what it looks like after the fix)

If the plan changes class structure (fields added/removed, methods deleted), also include a **class changes** diagram.

### Diagram types to use

| Plan content | Diagram type |
|---|---|
| Call/data flow | `flowchart TD` or `flowchart LR` |
| Sequence of method calls | `sequenceDiagram` |
| Class fields and relationships | `classDiagram` |
| State transitions | `stateDiagram-v2` |

Follow [mermaid-diagrams.md](mermaid-diagrams.md) for forbidden characters and safe label syntax.

## Why

Diagrams communicate architecture intent faster than paragraphs, survive ambiguity better, and are easier to verify against code when implementing. A plan that is mostly prose is harder to execute without re-reading the full context.
