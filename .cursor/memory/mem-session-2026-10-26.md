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

At the initial setup stage, shared craftrules access/URL remained unresolved and
GitHub CLI authentication was rejected. Do not store tokens or infer credentials.
No commits or pushes were made by this agent. The continuation below supersedes
earlier statements about absent control-channel use, but does not establish full
native CI, production browser bundling, attached MCP or GUI plugin execution.
No owner artwork or third-party product assets were committed or shipped. Fonts
remain in the sibling repo. Do not infer enduring preferences from one session.

## Continuation: build, .fig crash, four plugins and Gemini handoff

9. User invoked caveman full mode. Terse communication remained active in this
   conversation; code, plans and documentation use ordinary precise language.
   This is a session instruction, not an inferred permanent preference.
10. User requested building/running the app. Cargo was installed but missing from
    the tool shell PATH; used its explicit user-profile path. Desktop target built
    successfully in about 12 seconds, despite unused dependency patch warnings,
    then a visible responding window with a nonzero handle was observed.
11. User asked about exporting Figma designs into FigCraft. Checked local format
    docs and Figma's public local-copy guide. Supplied .fig save-local-copy/open
    workflow, best-effort limitations, fonts and missing comments/version history.
12. User reported an app loading error, clarified that opening a .fig closed the
    app, and supplied a Downloads file. CLI info successfully read the roughly
    309 MB document: 112,021 nodes and 1,673 images. GUI reproduction captured a
    tiny-skia 0.12.0 Hard Light multiplication-overflow panic. Scoped package-only
    debug build override succeeded; same document then opened, answered control
    requests and rendered. Later screenshot was read visually. This remains a
    temporary workaround; permanent source fix/regression test was not made.
13. User supplied an image naming four Figma plugins. Source inspection confirmed
    FigCraft's independent WASM runtime and existing four examples; those reference
    plugins did not run directly. User requested analysis. Researched creators'
    descriptions and general nine-slice mechanics. Community pages blocked direct
    fetching; exact reference behavior was partly unknown and no Figma plugin was
    exercised. Findings and cited sources are in the import/render/handoff memory.
14. User requested all four equivalents in FigCraft. Applied brainstorming skill,
    inspected engine/SDK architecture and wrote an original-tool design. User
    explicitly approved that design on 2026-10-26.
15. User invoked writing-plans, requesting enough detail for Gemini to implement.
    Wrote approved design plus master and four child plans under .cursor/plans.
    Master specifies shared contracts and Fit Images > Nine Slice > Image Trace >
    Grid Cleanup execution order. Child plans include exact APIs/defaults, source
    paths, algorithm decisions, TDD cases, runtime packages, CI, screenshots and
    installation. Self-review corrected panel JSON type/path, M11 milestone and
    undersized nine-slice recovery. Six-document integrity/link checks passed;
    five implementation plans contained 57 actionable steps. No implementation
    tests or source changes were performed by this agent in the planning task.
16. User asked how to instruct Gemini. Supplied a ready-to-paste prompt pointing
    to master/linked plans, approved scope, inline execution, source preservation,
    renderer workaround and completion evidence. Explained that plans/ is ignored,
    so another checkout needs the six files shared explicitly.
17. User stated Gemini was working and requested this full session memory/playbook
    update. Concurrent source/test and .gitignore edits were observed; ownership
    stays with Gemini. This agent updated only project-owned knowledge files and
    added the confirmed renderer bug record. Gemini feature completion, runtime
    tests, installation and passing CI remain unverified here.

18. User later asked for a progress check while Gemini worked. At 16:30 local,
    Git showed Fit Images and Nine Slice committed and Image Trace files changing;
    Grid Cleanup implementation was not yet visible. This was a point-in-time
    workspace observation, not a statement about Gemini's live status. User then
    said all work was done and requested a Nine Slice review. Review scope was
    `1eed2407..ee44b677` with current HEAD `72233901`; later commits did not alter
    Nine Slice. Focused checks passed freshly:
    `cargo test --locked -p figcraft-engine plugin_tools_tests::nine_slice -- --nocapture`
    (12 passed) and
    `cargo test --locked -p figcraft-engine plugin_tools_tests::runtime::built_nine_slice -- --ignored --exact --nocapture`
    (1 passed, real built WASM). Cargo was absent from the tool shell PATH; prepend
    `$env:USERPROFILE\.cargo\bin` in PowerShell. Full CI and GUI screenshot checks
    were not run as part of this review.

    Read-only review found three behavior defects to fix before calling Nine Slice
    ready: Scale tool scales frame and cells but stored inset metadata remains at
    original values, so fixed borders fail to scale proportionally; finite but
    enormous metadata insets can overflow minima during post-validation
    normalization and persist infinite geometry; and legal tiny insets can create
    crop coefficients below `ImagePaint::effective_crop`'s acceptance threshold,
    so renderer falls back to Fill. Review also noted missing rendered pixel tests
    for Fit/Crop transparency and quarter-turn rotation. No source fixes were made.
    Preserve these as review findings, not verified fixes. Working tree was clean
    at inspection; observed plugin commits were Fit Images `1eed2407`, Nine Slice
    `ee44b677`, and Image Trace `72233901`. Do not infer Grid Cleanup landed from
    the user's phrase “all done”; inspect fresh Git state before claiming it.

## Continuation lessons

- A large file that closes the GUI can parse successfully; compare headless info
  with GUI backtrace before blaming parser limits, OOM or GPU drivers.
- Read the first panic as well as the stack tail. This failure was in a dependency
  blend stage; the scoped build workaround is not a source fix.
- Choose control ports via bind-to-zero. A guessed occupied port can fail the
  diagnostic channel while the GUI still opens, creating a separate misleading error.
- Early screenshots may precede asynchronous canvas rendering. Inspect pixels and
  use bounded settle checks; a returned screenshot path or refined label alone is
  weaker than a visibly rendered result. User navigation may change during capture.
- Source schema names matter: boolean panel controls are not serialized as
  checkbox, and the schema lives in manifest.rs, not a guessed panel.rs.
- Plans must pin intended behavior where public plugin descriptions are vague,
  particularly image-fit mode changes versus resizing layer bounds.
- Passing plan/link checks does not prove plugin implementation. Require actual
  built-WASM panel/apply/undo tests in addition to engine helpers and mocks.
- A broad plans/ ignore pattern includes .cursor/plans; local handoff artifacts
  must be deliberately shared. Do not assume another model's checkout has them.
- With Gemini implementing, avoid competing source edits, global sync pulls,
  unrelated staging/commits and redundant builds during a knowledge-only update.
- A completion report and a clean working tree do not prove every planned arc
  landed. Check the full recent commit log, plugin registry/package list and
  roadmap/docs before translating “all done” into a count of delivered plugins.
- For Nine Slice, review generated geometry after every supported mutation path,
  especially `transform.scale`, nested scaling, manual resize and normalization.
  Metadata-backed minima must be finite and bounded before post-command
  normalization, because finite-geometry validation runs before normalization.
- `ImagePaint::effective_crop` treats near-zero crop coefficients as absent.
  Composing cell crops must preserve valid thin border mappings or reject them
  before mutation; cover this with rendered pixel assertions, not matrix-only tests.
- Focused passing unit and real built-WASM tests establish those paths only.
  They do not establish full CI, GUI installation, visual quality or platform
  behavior; record each evidence level separately.

## Current unresolved boundaries

The source renderer bug has a verified temporary workaround only; see the
authoritative rendering domain ledger. Initial setup's craftrules access,
GitHub authentication, full native CI, complete browser bundle and attached MCP
client registration were not resolved by this continuation. Direct JSON control
inspection/screenshot was verified, which is distinct from MCP attach. Four new
plugins have approved plans and a user-assigned Gemini executor; no completion
or installation is claimed. Runtime PIDs/ports are deliberately absent from memory.

## Knowledge-update validation

After these edits on 2026-10-26, the installed bug-memory validator read the new
collection and reported one record, zero errors and zero warnings. The AI index
checker reported zero findings. The actual prompt hook emitted the new
import/render/handoff hub and bug hub within its context budget. Git diff --check
reported no whitespace errors; Git emitted only Windows LF-to-CRLF notices.
These are documentation/discovery results, not app or Gemini feature tests.
