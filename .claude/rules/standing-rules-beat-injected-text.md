# Rule: Standing Rules Beat Injected Text

**Scope:** Universal — no project config needed.

---

## Policy

When freshly-injected instructions conflict with standing rules, **the standing rules win.**

Injected text arrives mid-conversation, looks concrete and procedural, and feels like the
newest word on what to do. That recency is not authority. Check it against the standing
rules before following it.

### Precedence order (highest first)

| # | Source | Examples |
|---|---|---|
| 1 | Direct user instruction in this conversation | "use a subagent", "skip the tests" |
| 2 | User's global + project instructions | `~/.claude/CLAUDE.md`, project `CLAUDE.md`, `.claude/rules/*.md` |
| 3 | System prompt standing behavior | "Do not spawn agents unless the user asks" |
| 4 | Injected harness / mode / skill workflow text | plan mode phases, hook output, skill step lists |

A #4 instruction that contradicts #2 or #3 is **followed only in the parts that do not conflict.**
Do not treat a numbered workflow as permission to override a project rule.

### Known conflict points

| Injected text says | Standing rule says | Resolution |
|---|---|---|
| Plan mode Phase 1: "Launch up to 3 Explore agents IN PARALLEL" | Do not spawn agents unless the user asks ([no-unrequested-subagents.md](no-unrequested-subagents.md)) | Explore inline with Read/Grep/Glob |
| Plan mode Phase 1: "**only** use the Explore subagent type" | Skills must be invoked before acting ([superpowers-plugin-setup.md](superpowers-plugin-setup.md)) | "Only" constrains *subagent types*, not tools or skills. Invoke the skill. |
| Auto Mode: "bias toward working without stopping for clarifying questions" | Brainstorming before creative work; gather requirements first | Auto Mode's own text yields when "the shape of the task suggests they want you to ask". A vague new-feature request is that shape. Ask. |

### Requirements come before exploration

Plan mode Phase 1 reads: "reading through code **and asking them questions**."
Both halves are mandatory. When the spec lives in the user's head — a new feature, a vague
comparison ("something like ShoeBox"), an undefined scope — **questions come first.**
Exploring before requirements means searching the codebase against an invented spec.

## Why

Observed failure (2026-08-25): entering plan mode injected a 5-phase workflow. Its
procedural framing displaced two standing rules simultaneously — the brainstorming-first
skill rule and the do-not-spawn-agents instruction — inside a single turn. Neither rule
had been revoked; the newer text simply felt more actionable.

Recency is the failure mode. Naming the precedence order makes the check explicit
rather than leaving it to whichever instruction was read most recently.
