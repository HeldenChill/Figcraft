# Playbook: Adopt master AI config in a project (and hand work to an external executor)

**Use when:** onboarding a Unity project onto `claude-unity-shared` for the first time, migrating an older project's `.cursor/agents` / ad-hoc memories into the Universal + Project layout, or handing an AI-config plan to another agent (Gemini/Antigravity) and reviewing its result.

Distilled from a full adoption run (2026-10-22): first pull, project layer build-out, retiring six layer agents, external-executor implementation, review, shared-repo fixes, Gemini config added to master.

---

## 1. First pull

1. **`project.config.json` must exist** at the project root, or `sync.ps1` refuses to run. Fill it from facts, not guesses:
   - `L5_ASSEMBLY` / `L5_PATH`: the composition-root asmdef `name` and its folder (`Assets/**/*.asmdef`, pick the one that references the most game assemblies).
   - `UNITY_VERSION`: `ProjectSettings/ProjectVersion.txt` → `m_EditorVersion`.
   - `LOCATOR_SERVICE`: grep `class Locator` / `interface IGameplayService`.
   - `INTEGRITY_CHECK`: grep `IntegrityCheck`; if none, write `(none — review asmdef references manually after structural changes)`.
2. Always run from the project root with an explicit shared path:
   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File <shared-root>/sync.ps1 pull -SharedRepo <shared-root> -WhatIf
   ```
   Read every `WOULD` and `WARNING` line, then run without `-WhatIf`.
3. Expected first-pull noise:
   - `WARNING kept .cursor/agents/<role>.md: retired in master but edited locally` — the old layer agents. Handled in §3.
   - `check-ai-index.js` → `indexSentinelsMissing` — the project has no Memory index yet. Handled in §2.

## 2. Build the Project layer

Model it on a project that already completed adoption. Create:

| What | Where | Notes |
|---|---|---|
| Layer map + inline-only + runtime-honesty rule | `.claude/rules/layer-architecture-dispatch.md` | Layer table from the **real asmdefs**, not from old agent prompts (they drift — check for asmdefs missing from the map and asmdef file names that differ from assembly names) |
| Memory consumption / writing rules | `.claude/rules/memory-first-workflow.md`, `memory-write-rules.md` | Usually copy verbatim from an adopted project |
| Knowledge index rule | `.claude/rules/project-knowledge-index.md` | Set `DOCS_DIR = (none yet)` if there is no docs folder |
| IDE sync rule | `.claude/rules/unity-ide-sync.md` | Project menu path + autosync script path |
| Root adapters | `CLAUDE.md` and `AGENTS.md` (identical) | Empty `<!-- GENERATED:REGISTRY:BEGIN -->` / `END` pair; sync fills it |
| Cursor entry | `.cursor/rules/shared-claude-context.mdc` | Points Cursor at `CLAUDE.md` |
| Playbook index | `.cursor/playbooks/README.md` | Sync adds a `Playbooks:` link to the registry **only once this file exists** — pull again after creating it |
| Plans | `.cursor/plans/design/`, `.cursor/plans/implement/` | |

Every rule file needs frontmatter `description:` — the registry's "What it governs" column reads it. Skip `project-values.md` and `pkg-*` rules when no `com.hung.*` package is installed.

### Memory index (exact format the tools parse)

Append to the canonical architecture memory:

```
## Memory index

| Topic | File |
|---|---|
<!-- LOCAL-MEMORY-INDEX:BEGIN -->
| HUB: <domain hub description> | `mem-<hub>.md` |
| <one-line description> | `mem-<topic>.md` |
<!-- LOCAL-MEMORY-INDEX:END -->
```

- The header **must** be `| Topic | File |`. Any other header text is reported as `rowUnparseable`.
- Every `mem-*.md` except the canonical file needs exactly one row; the canonical file gets none.
- At least one `HUB:` row, or the prompt hook prints no hubs.
- Before handing the block to anyone, test it: copy `.cursor/memory/` to a scratch folder, append the block, run `node .claude/tools/ai-audit/check-ai-index.js --project <scratch>` and `node .claude/hooks/memory-index-injector.js < /dev/null` from that folder.

## 3. Retire the old layer agents

Harvest first, delete second. Agent prompts are mostly generic role text; mine only the project-specific parts:

| Agent content | Destination |
|---|---|
| Layer map, dependency direction, exception approval | `layer-architecture-dispatch.md` + canonical memory (approver becomes "the user") |
| IDE/asset handoff instructions | `unity-ide-sync.md` |
| SDK hard rules, platform checklists, analytics contract, test matrix | a feature playbook memory (`mem-<area>-playbook.md`, e.g. an SDK integration playbook) |
| Release gate, startup sequencing, rollback | process playbook `.cursor/playbooks/release-readiness.md` |
| Ownership tables, "skill improvement", portability/reuse advice, handoff-to-other-agent contracts | drop — superseded by inline work and the sync system |

Then:
- Replace any `## Ownership Model by Agent` section in memories with a short "Working model: inline, no project agents".
- Rewrite live references to agent names; **leave dated history/changelog lines unchanged**.
- Delete `.cursor/agents/` and any skill folder the owner retires; rewrite pointers to them.
- Verify: `sync.ps1 pull -WhatIf` shows `0 warnings`, the index check shows `0 finding(s)`, and a grep for the agent names hits only history lines.

## 4. Fixing a synced file

A file the WhatIf lists as `[U]` / `[T]` cannot be fixed in the project — the next pull overwrites it. Fix it in `claude-unity-shared` on a branch (`files/...`; also the root `playbooks/` copy if one exists), pull into the project, then `node tools/pr-check.js --base main` **after committing** (it only checks committed changes).

## 5. Gemini / Antigravity config

`GEMINI.md`, `.agents/rules/*.md` and the `.agents/skills/` folders in `manifest.json` `skillDirs` are Universal. A pull replaces condensed project-made versions of those skills with the upstream ones. Project-only Gemini skills go in other `.agents/skills/<name>/` folders. Remove leftover duplicates (e.g. a custom TDD skill beside `test-driven-development`) so Gemini does not pick the stale one.

## 6. Handing the plan to an external executor (Gemini)

Write the executor plan to `.cursor/plans/implement/` so that every step is literal:

- **Full file content** for every new file; **exact find/replace blocks** (fenced, whole lines) for every edit. Never put find strings with escaped backticks in inline code.
- **Expected output** after every command, plus "if output differs, STOP and report". Do not pin counts the tool may vary (e.g. the WhatIf `unchanged` count).
- **Hard boundaries:** no git commands (the owner commits), never edit synced files, never touch `Assets/`, `ProjectSettings/`, `Packages/`; edit only files in the task's Files block.
- State the line-ending convention of the files being edited (existing memories are often CRLF).
- Account in the "expected hits" list for every text the plan itself introduces (e.g. "retired" mentions), and avoid replacement blocks that end in a blank line when the next line is a heading.

## 7. Reviewing the executor's result

Check, in this order:
1. **Out-of-plan changes:** `git status --short` / `git diff --cached --stat`. Executors may vendor whole skill sets or edit their own instruction file (`GEMINI.md`) unasked — split those out or get the owner's decision.
2. **Fidelity:** each created file equals the plan block (normalize CRLF before comparing); each edit applied exactly; unlisted rewordings noted.
3. **Encoding and line endings:** search changed files for mojibake (`â€`, `Ã`, `Â§`, `â†`). Windows PowerShell 5.1 *displays* UTF-8 as mojibake in its console — confirm with a UTF-8 reader before calling a file corrupted. Small per-file diff stats mean no whole-file EOL rewrite.
4. **Gates:** index check `0 finding(s)`; `sync.ps1 pull -WhatIf` zero writes and zero warnings.
5. **Claims in the executor's log:** treat "clean"/"done" lines as claims; re-run the commands.

## 8. Environment notes

- Git Bash can die mid-session (`fatal error - add_item ... errno 1`); switch to PowerShell — every command above has a PowerShell form.
- Unity regenerates `*.csproj` constantly; filter `\.csproj$` out of `git status` when reviewing.
