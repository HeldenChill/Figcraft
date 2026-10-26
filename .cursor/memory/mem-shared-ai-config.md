# Memory: Shared AI Config — Sync System

**ID:** `mem-shared-ai-config` — Recall for the central sync system that keeps Claude/Cursor AI config consistent across projects.

---

## What this system is

This project's `.claude/` and `.cursor/` AI config comes from three scopes:

| Scope | What | Source of truth | Reaches the project by |
|---|---|---|---|
| Universal | rules, hooks, tools, playbooks, the settings base and this memory, the same in every project | `claude-unity-shared` (`files/`, listed in `manifest.json`) | `sync.ps1 pull` |
| Framework | `pkg-<package>-*` rules, hooks, tools and skills, plus `.cursor/pkg-ai-index.md` | the `AI~/` folder of each installed `com.hung.*` package (`Package_Repo`, shipped by a release tag) | `sync.ps1 pull` reads `Packages/packages-lock.json` and copies them |
| Project | everything else: project rules, `project-values.md`, memories, plans, playbooks | this project | hand-written, never touched by sync |

Location of the shared repo: pass `-SharedRepo <path>` to `sync.ps1` or set `$env:SHARED_CONFIG_DIR`; do not assume the script's built-in default matches this workstation. Full mechanics: `docs/sync-system.md` in the shared repo.

---

## Generated files (never edit by hand)

| File | Made from |
|---|---|
| `.claude/settings.json` | the Universal base `files/claude/settings.json` + every installed package's `AI~/hooks/hooks.json` |
| `.claude/rules/pkg-*.md`, `.claude/hooks/pkg-*`, `.claude/tools/pkg-*`, `.claude/skills/pkg-*/` | installed packages' `AI~/` folders; each carries a `GENERATED from com.hung.<x>@<version>` header |
| `.cursor/pkg-ai-index.md` | installed packages' `AI~/knowledge/` and `AI~/playbooks/` |
| the table between `<!-- GENERATED:REGISTRY:BEGIN -->` and `<!-- GENERATED:REGISTRY:END -->` in `CLAUDE.md` / `AGENTS.md` | every `.claude/rules/*.md`, tagged Universal / Framework / Project |

To change a generated file, edit its source (master, or the package in `Package_Repo` with a version bump and release) and pull again. Text outside the registry sentinels in `CLAUDE.md` / `AGENTS.md` is hand-written and kept.

---

## The three buckets (Universal scope)

Every Universal file is classified in `manifest.json` by bucket:

| Bucket | What it means | Safe to edit per-project? |
|--------|---------------|--------------------------|
| **universal** | Byte-identical everywhere (line endings ignored); no substitution | No — next pull overwrites |
| **templated** | Contains `{{...}}` placeholders in master; project copy has real values | No — next pull overwrites |
| **local** | NOT in manifest; sync never touches it | Yes — always safe |

**Local files** include feature memories, plans (`.cursor/plans/`), project rules, `project-values.md` and `settings.local.json`.

**Gemini / Antigravity config (Universal):** `GEMINI.md`, `.agents/rules/*.md` (from `files/gemini/`, `files/agents/rules/`) and the skill folders under `.agents/skills/` listed in `manifest.json` `skillDirs` (Superpowers workflow skills + `caveman`, copied from upstream; `subagent-driven-development` and `dispatching-parallel-agents` are deliberately not shipped). A skill folder is rewritten when any shipped file is missing or differs; extra files a project adds inside it are kept. Project-only Gemini skills live in other `.agents/skills/<name>/` folders and are local.

**Retired files:** `manifest.json` `removed` lists files master no longer ships. A pull deletes a project's copy only while it still equals the last master text; an edited copy is kept with a `WARNING kept …` line, to merge or delete by hand.

---

## When you edit AI config in this project

1. **Is it a generated file?** Edit its source instead (table above).
2. **Is the file listed in `manifest.json` `files`?** No → it is local; edit freely. Yes → it is Universal; continue.
3. **Should every project get the change?** Edit the master copy on a branch of `claude-unity-shared` (or edit here and `sync.ps1 push` while master is on a branch — push refuses `main`/`master`), run `node tools/pr-check.js --base main`, open a PR. After the owner merges, `sync.ps1 pull` in each project and commit there.
4. **Is the value project-specific?** It does not belong in a Universal or Framework file: put it in `.claude/rules/project-values.md` (the section named after the rule) or in a local memory.
5. Run `sync.ps1 pull -WhatIf` before every pull: a local edit to a Universal file is overwritten by the next pull.

---

## The canonical project memory registry

`mem-arch-assemblies-canonical.md` holds the project's `## Memory index` table. Add new Project memories as two-column rows inside `LOCAL-MEMORY-INDEX` boundaries. The master context is a domain hub, not another mandatory registry.

The current manifest does not list the canonical architecture/index file; it is Project-owned and is not rewritten by current pull or exported by current push. Older active-splice descriptions are historical. Check current manifest membership before assuming ownership.

The shared memory hook reads this index, emits `HUB:` rows within a 3,000-character budget and tells the model how to search the full registry. It does not inject all memory bodies or all registry rows.

---

## The two sync directions

| Direction | Command | What it does |
|-----------|---------|--------------|
| pull | `.\sync.ps1 pull -SharedRepo <path>` | Master fills tokens and writes Universal files; then deletes unedited retired files, installs package `AI~` folders, generates `settings.json`, the package index and the registry. No commit. |
| push | `.\sync.ps1 push -SharedRepo <path>` | Project de-tokenizes and writes master files. Only while master is on a branch. Then commit there and open a PR. |

Always run with `-WhatIf` first.

---

## Supporting docs

- Full mechanics, gotchas, and extension guide: `docs/sync-system.md` in the shared repo
- Token values for this project: `project.config.json` in this project root
- File-to-bucket mapping and the `removed` list: `manifest.json` in the shared repo

## When a shared hook errors

- **`require is not defined in ES module scope`** on every prompt or every Edit/Write: the project root `package.json` sets `"type": "module"`, so Node loads the CommonJS hooks and tools under `.claude/` as ES modules. `.claude/package.json` (`{ "type": "commonjs" }`, Universal since 2026-10-18) scopes them back; if it is missing, run `sync.ps1 pull`.
- **Check every project before calling a hook problem universal:** run each command in `.claude/settings.json` `hooks` with a dummy JSON payload on stdin and `CLAUDE_PROJECT_DIR` set; a non-zero exit with stderr is what the Claude UI shows as a hook error.
- A PostToolUse `memUnregistered` right after creating a `mem-*.md` is expected when its index row is added in the next edit; re-run `node .claude/tools/ai-audit/check-ai-index.js` after the batch.

---

## `.opencode/` (project-bound AI tooling config)

OpenCode config lives at `.opencode/` and is **project-bound, not
master-synced**. It contains the opencode provider/model config
(`opencode.json`), project-specific agents/skills/commands/permissions,
the bootstrap guide, and a sample env file.

Sync is via git only — no `sync.ps1` for `.opencode/`. Per-machine
secrets go in `.opencode/.env` (gitignored) and reference
`ANTHROPIC_API_KEY` via `{env:VAR}` syntax in `opencode.json`.

Treat the whole folder as a `local` bucket entry: do not add it to the
shared manifest. If a project does not use opencode, the folder is
absent and the rules in this section do not apply.

## Audit repository guards

Portable scenarios use `guardRepos: ["shared-config"]`. Supply the actual shared Git root with `--shared-repo` or `SHARED_CONFIG_DIR`; the CLI value takes precedence. No machine-drive fallback is guessed.

Before an L3 run, use `run-scenarios.js --preflight` with the intended project/scenario/shared-repo inputs. It resolves selected guards and validates exact Git roots and clean state without creating output/worktrees, invoking clients or restoring repositories. Normal runs perform this validation before worktree mutation and again before each guarded run. A guard is not a general-purpose cleanup command.

AppsFlyer maintenance skill was retired from shared delivery. Existing unchanged installed copies are removed through the retirement manifest; SDK integration itself is unaffected.

## Bug memory and audit organization

Bug memories and audit reports are project-owned data. Universal rule `bug-memory-and-audits.md` and validator `validate-bug-memory.js` are delivered by shared sync. Project bug data, catalogs (`.cursor/audits/`), and bug records (`.cursor/memory/bugs/`) are never synchronized from master.
