# Session record: 2026-10-26

This is a factual session summary, not a raw chat transcript or a permanent
claim about current machine state. Topic memories and the playbook contain the
reusable details.

## Requests and outcomes

1. User initially requested syncing AI config to a folder named
   D:\UnityProject\claude-unity-shared. Inspection showed it was the shared
   source repo. User clarified: pull that shared configuration into FigCraft.
   Pulled 51 manifest entries/settings or directories using its documented script.
2. User requested project scouting. Inspected Cargo, docs, source and Git state.
   Identified Rust/egui architecture, alpha status, supported apps/formats, stale
   architecture notes, missing Rust/tools/font checkout and missing standards.
3. User requested all needed setup. Installed Rust/Cargo 1.99.1, formatting,
   linting, language server and WASM target; installed checksum-verified Trunk
   0.22.0; reused existing native build tools; fetched locked dependencies and
   CI-pinned fonts into the sibling repo. Added Rust-specific AI context and root
   rule registries. Standards checkout remained blocked by repo/access errors.
4. Built the native workspace and passed the compiled xtask WASM/plugin gate.
   CLI synthetic save/reopen/render, MCP initialization, layers/assets checks,
   server help and AI sync validation succeeded. Full native CI and Trunk bundle
   were not run. Evidence is under local plan/setup/.
5. User asked to build and run. Desktop build passed and a visible responding
   window was verified. User requested relaunches after closing it; launched
   again without terminating other processes or discarding documents. Runtime
   process IDs were temporary; no app PID should be reused later.
6. User asked about MCP. Tested initialize plus tools/list; confirmed 18 tools
   and described headless versus attached operation. App was not started with a
   control port, and no MCP client connector was configured.
7. User asked about plugins and how to import one. Confirmed native WASM plugin
   support and four built examples. Gave Swatch Grid's exact manifest path and
   explained its palette output. User could not locate Plugins; source showed
   that it is inside the logo Main menu on the far-left rail. Import/execution
   instructions were supplied, but installation was not performed or verified.
8. User requested saving all session/project knowledge and incorporating the
   experience into playbooks. Created indexed topic memories, a project-owned
   operating playbook, discovery links and a memory-first workflow rule.

## Lessons preserved

- Resolve sync direction and source ownership from README/manifest. Do not
  replace project Rust instructions with Unity templates.
- No registry markers means sync leaves root files untouched; add explicit
  markers when adopting generated discovery tables.
- A zero-finding index check without a canonical index does not establish that
  memory is connected. Create the canonical table and test actual hook output.
- Official rustup installer dispatch depends on filename rustup-init.exe.
  A renamed figcraft-rustup-init.exe failed as an unknown proxy; copying it to
  the required basename fixed installation.
- Windows PowerShell execution policy blocked direct sync.ps1 invocation. Use
  the documented child powershell -NoProfile -ExecutionPolicy Bypass -File
  invocation; no machine-wide policy change was needed.
- Python JSON embedded in a PowerShell -c argument lost quotes and failed.
  A single-quoted here-string piped to python - preserved JSON and newlines.
- Cargo -q can silently rebuild xtask dependencies due to different feature
  unification after a workspace build. Inspect owned processes/logs before
  diagnosing a hang; the built xtask.exe avoids that redundant launcher rebuild.
  Only the verified, session-owned duplicate Cargo process tree was stopped.
- Cargo warnings can look like PowerShell NativeCommandError entries in a
  redirected log even when the process exits zero. Check the exit code and
  Finished result; do not infer failure from formatting alone.
- Confirm a visible process has a window and is responding. A successful
  Start-Process result alone is not proof of a usable GUI.
- Need precise visual navigation: Plugins is nested under the logo menu.
- Swatch Grid's palette panel has fixed size/gap; its custom-hex command exposes
  those controls. Do not conflate the two command forms.

## Outstanding work and boundaries

Shared craftrules access/URL is unresolved; GitHub CLI authentication was rejected.
Do not store tokens or attempt to infer credentials. Full native CI, production
browser bundling, attached MCP and GUI plugin execution remain unverified.
No commits or pushes were made. Setup changes were left in the working tree.
No private documents, third-party product assets, real-user photos, or fonts were
added to this repo. Do not infer enduring user preferences from these requests.
