# Rule: Doc Diagrams

**Scope:** Universal — no project config needed.

---

## Policy

When writing or updating a project documentation file in the configured
DOCS_DIR, prefer Mermaid diagrams over prose descriptions wherever the content describes:

- A data flow (how data moves between objects/systems)
- A call sequence (who calls what, in what order)
- Architecture structure (assemblies, layers, ownership)
- Class or struct relationships (fields, methods, inheritance)
- A state machine or decision tree

### Priority order for doc content

1. **Mermaid diagram** — for anything spatial, sequential, or relational
2. **Table** — for mapping (files → assemblies, contracts → roles, fields → types)
3. **Bullet list** — for simple enumeration with no structure
4. **Prose paragraph** — only for nuance that diagrams and tables cannot capture (design intent, constraints, known pitfalls)

### Required diagrams in docs

Every doc that describes a data pipeline or call flow **must include** a flow diagram.

Every doc that describes assembly/layer structure **must include** an assembly diagram.

If a doc describes a before/after architectural change, include both a **current** and a **target** diagram.

### Diagram types to use

| Doc content | Diagram type |
|---|---|
| Call/data flow | `flowchart TD` or `flowchart LR` |
| Sequence of method calls | `sequenceDiagram` |
| Class fields and relationships | `classDiagram` |
| State transitions | `stateDiagram-v2` |
| Assembly/layer structure | `flowchart TD` with subgraphs per layer |

Follow [mermaid-diagrams.md](mermaid-diagrams.md) for forbidden characters and safe label syntax.

## Why

Docs that are mostly prose require a full read to extract the structure. Diagrams let any agent or developer orient instantly, verify architecture decisions against code, and spot regressions. A doc without a diagram is harder to maintain and easier to silently diverge from the code.
