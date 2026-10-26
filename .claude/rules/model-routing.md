# Rule: Model Routing

**Scope:** Universal — no project config needed.

---

## Policy

The main thread picks the model tier for every **explicitly requested** agent spawn via the `Agent` tool's `model` parameter, recorded in the brief's `Model` block (see `dispatch-brief.md`). There are no project agent definitions; routing is a per-task decision.

### Routing table

| Task class | Model | Rationale |
|---|---|---|
| Read/summarize 1–2 known files; verify a fact; locate a symbol; regenerate generated artifacts | `haiku` | Retrieval and mechanical transforms need no deep reasoning |
| Broad search / exploration fan-outs (Explore agent) | `haiku` | Volume work; conclusion only |
| Single-layer implementation: scoped feature, bugfix with confirmed root cause, applying a pre-approved design | `sonnet` | Workhorse tier; full coding competence on a scoped task |
| Cross-layer root-cause analysis; design-fork framing; restoration triage; architecture decisions; PM gating | `opus` or inherit | Errors here cascade into re-dispatches costing more than the model delta |

Debug-bug phases: Phases 1–3 (analyze/narrow/confirm) route by the table above based on scope; Phases 4–5 (apply fix) are usually `sonnet` because the root cause and chosen design arrive pre-confirmed in the brief.

### Guardrails (non-negotiable)

1. **Escalate, never struggle.** If a `haiku` agent's report shows uncertainty, scope growth, or it touched more files than briefed — re-dispatch at `sonnet`. Do not patch doubtful output on the main thread.
2. **No downgrade mid-task.** Follow-ups to a running agent (SendMessage) continue at its original tier or higher.
3. **The compile gate is the accuracy floor.** Every code task produces build evidence regardless of model (`pkg-framework-compile-verification.md`), so a cheaper model cannot silently ship a broken edit — worst case is a visible build failure. This is what makes aggressive down-routing safe.

## Why

Without routing, a 2-file summary costs the same per-token rate as a cross-layer refactor. Routing puts cheap tiers on retrieval and mechanical work while keeping full intelligence on judgment-heavy work — token-efficient without moving the correctness floor, because correctness is enforced by the gate, not the model.
