# FigCraft Windows setup and verification

Observed and performed: 2026-10-26. Recheck before claiming current readiness.

## Workstation setup

| Item | Observed state |
|---|---|
| Rust/Cargo | Installed stable 1.99.1, x86_64-pc-windows-msvc |
| Components | rustfmt, Clippy and rust-analyzer installed |
| Targets | Native MSVC and wasm32-unknown-unknown installed |
| Web tool | Trunk 0.22.0 official Windows binary, published SHA-256 checked |
| Native prerequisites | Existing VS Build Tools 2022, MSVC 14.44.35207, Windows SDK 10.0.26100.0 |
| Existing tools | Git, Node.js, Python 3.12, winget, GitHub CLI, Git Bash |
| Rust bin | %USERPROFILE%\.cargo\bin; installer added it to user PATH |
| Git Bash | C:\Program Files\Git\bin\bash.exe exists; bash was not on the tool shell's PATH |

Rust initially was absent, not merely hidden on PATH. New terminals pick up the
installed user PATH. Existing tool shells may need to prepend the cargo bin
directory explicitly. No editor application or editor extension was installed.

## Dependencies and access

Locked Cargo dependencies were downloaded successfully, including pinned egui,
winit and craft-libs repositories. Cargo.lock was preserved.

Fonts were cloned to D:\Projects\craft-fonts and checked out at
37827adc41fd165f9a9234ac40f94dbad353aa69, the revision in this checkout's CI.
The existing .cargo/config.toml already points to ../craft-fonts. Fonts were not
copied into FigCraft. Recheck the CI pin before future refreshes.

Cloning https://github.com/storytold/craftrules.git to the documented
D:\craftrules location returned Repository not found. GitHub CLI subsequently
returned HTTP 401 with rejected credentials. It is not established whether the
repo is private, removed or at another URL. No credentials were changed. User
was asked for an accessible URL or local checkout; none was supplied in this
session. This standards checkout remains pending; builds did not require it.

## AI configuration adoption

Source is D:\UnityProject\claude-unity-shared; destination is this project.
User clarified that the requested direction was pull, not export to shared.
Created project.config.json compatibility inputs and used the shared sync.ps1.
Its Unity-specific values must not be interpreted as real Rust architecture.

Initial pull wrote 51 manifest entries/settings or skill directories. Root
AGENTS.md/CLAUDE.md initially had no GENERATED:REGISTRY markers and were skipped.
Markers were subsequently added while retaining their original instructions.
Project rule .claude/rules/figcraft-project.md and Cursor entry
.cursor/rules/figcraft-context.mdc were added. Shared-owned source files were
preserved. The Unity character-refactor playbook remains synced but is outside
FigCraft's scope. A zero-change dry run and zero-finding index check passed.

## Verified outcomes

| Check | Session result |
|---|---|
| cargo build --workspace --locked -j 4 | Passed; initial build took about 10m16s |
| cargo build --locked -p figcraft -j 4 | Passed; subsequent build about 2.21s |
| Compiled xtask.exe wasm | Passed browser/core target checks, plugin formatting/Clippy/tests and release WASM builds |
| CLI smoke | Created rectangle, saved/reopened .figcraft, rendered 160x90 PNG |
| MCP | initialize succeeded; tools/list returned 18 tools |
| Server | --help ran successfully; no hosted server was deployed |
| Layers/assets | Compiled xtask runner checks passed |
| AI config | Index checker and zero-change shared-sync preview passed |
| Desktop | Visible window launched repeatedly; process responded and had a window handle |
| Later .fig diagnostics | CLI info succeeded on owner's 309 MB file; GUI Hard Light overflow reproduced; scoped workaround rebuild opened and visibly rendered it |
| Direct control | Later free-port GUI run answered ui.inspect and ui.screenshot; PNG inspected |

Not performed: full native workspace tests, full cargo xtask ci, a complete
Trunk release bundle, attached MCP verification against the visible app, GUI
plugin import, or Swatch Grid execution through the UI. Do not imply those passed.
Native builds had unused dependency patch warnings; their exit codes were zero.

Logs and synthetic artifacts are local/gitignored under plan/setup/. Quickstart
is plan/SETUP.md; tool facts are plan/setup/tools.json. These can be absent in a
fresh clone. Existing Git state was branch Hung, HEAD 08c1df76 (~FIX: AI Config)
before setup edits. No commit or push was made in this session. Recheck Git and
process state rather than assuming this snapshot remains current.

## Later same-day renderer and executor boundary

The tool shell again lacked Cargo on PATH despite the installed toolchain. An
explicit `$env:USERPROFILE\.cargo\bin\cargo.exe` invocation built the desktop.
The normal debug binary could open an empty document but crashed drawing the
owner's .fig due to tiny-skia 0.12.0 Hard Light u16 overflow. The package-only
overflow-check override was verified as a temporary workaround; ordinary rebuild
may reintroduce it. See
[import/render handoff memory](mem-figcraft-import-render-and-plugin-handoff.md)
and rendering bug ledger for exact evidence and limits. No global profile or
dependency source was changed by this agent.

Owner subsequently assigned Gemini the approved four-plugin implementation.
Concurrent code changes are expected; do not overwrite or commit them during
maintenance of these setup notes. Fresh implementation verification is pending.
