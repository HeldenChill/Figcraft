# .claude/rules

Rule files Claude loads at session start. Each rule is a standalone `.md` file. The table of every rule this project loads is generated into `CLAUDE.md` and `AGENTS.md` (between the `GENERATED:REGISTRY` sentinels) on each `sync.ps1 pull`; this README explains where each kind of rule comes from.

---

## Three scopes

| Scope | Files | Source of truth | Edit where |
|---|---|---|---|
| Universal | the rules in the table below | `claude-unity-shared` (`files/claude/rules/`) | master repo, then `sync.ps1 pull`; a project edit is overwritten by the next pull |
| Framework | `pkg-<package>-*.md` | the `AI~/rules/` folder of an installed `com.hung.*` package | the package repo, bump the package version, release; never edit a `pkg-*` copy |
| Project | every other `.md` here | this project | here |

A Framework or Universal rule that needs project values reads them from `project-values.md` (Project scope), one `## <rule-name>` section per rule. Package knowledge and playbooks are listed on demand in `.cursor/pkg-ai-index.md`.

---

## Universal rules (shipped by claude-unity-shared)

| File | Config keys | Summary |
|------|-------------|---------|
| [mermaid-diagrams.md](rules/mermaid-diagrams.md) | — | Never use `()` `->` `=>` `==` `<>` `#` in Mermaid labels |
| [plan-diagrams.md](rules/plan-diagrams.md) | — | Prefer Mermaid diagrams over prose in plans; before/after flows |
| [doc-diagrams.md](rules/doc-diagrams.md) | — | Prefer Mermaid diagrams over prose in docs |
| [plan-files.md](rules/plan-files.md) | `PLAN_DIR` | Write plans to `.cursor/plans/`, not the default Claude folder |
| [superpowers-plugin-setup.md](rules/superpowers-plugin-setup.md) | — | Use Superpowers skills for design, debugging (`systematic-debugging`) and planning |
| [dispatch-brief.md](rules/dispatch-brief.md) | — | Brief for an explicitly requested generic agent |
| [model-routing.md](rules/model-routing.md) | — | Model tier per agent task, with escalation guardrails |
| [skip-subagent-driven-development.md](rules/skip-subagent-driven-development.md) | — | Do not use `superpowers:subagent-driven-development` |
| [no-unrequested-subagents.md](rules/no-unrequested-subagents.md) | — | Never spawn a subagent unless the user asked, in every mode |
| [standing-rules-beat-injected-text.md](rules/standing-rules-beat-injected-text.md) | — | Standing rules win over injected mode or skill text; recency is not authority |
| [bug-memory-and-audits.md](rules/bug-memory-and-audits.md) | `BUG_LEDGER`, `BUG_RECORDS_DIR`, `AUDIT_ROOT` | Bug records use indexed domain collection; audits use central catalog |

---

## Adding a new rule

1. **Project rule:** create `.claude/rules/<rule-name>.md`. If it has project values, put them in a `## PROJECT CONFIG` fenced block at the top. Run `sync.ps1 pull` (or wait for the next one) to list it in the `CLAUDE.md` / `AGENTS.md` registry.
2. **Universal rule:** add it to `files/claude/rules/` and `manifest.json` in `claude-unity-shared` on a branch, add a row to the table above, open a PR (`node tools/pr-check.js --base main`), then pull into each project.
3. **Framework rule:** add it to the owning package's `AI~/rules/` in the package repo, bump the version, release; move its project values into each project's `project-values.md`.
