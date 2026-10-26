# FigCraft Windows setup and operations

Created from the 2026-10-26 session. Project-owned, not a shared manifest entry.
Use this for local setup, shared AI adoption, builds, launches and plugin help.
Start with AGENTS.md and the memory index; recheck the current machine and pins.

## 1. Orient and inspect

From D:\Projects\Figcraft, read ROADMAP.md, docs/gaps.md, Cargo.toml and
.cargo/config.toml. Check git status and the active branch before changing
anything. FigCraft uses Rust/egui, not Unity. Do not invent C# assemblies to fill
shared templates. Architecture prose can be stale; confirm format claims in
source. Preserve existing user changes and running documents.

Inspect tools before installing replacements:

```powershell
Get-Command rustc,cargo,rustup,git,node,python,winget -ErrorAction SilentlyContinue
Get-ChildItem "$env:USERPROFILE\.cargo\bin" -ErrorAction SilentlyContinue
```

On Windows, confirm the MSVC compiler and Windows SDK exist, not just the Visual
Studio product name. The session used VS Build Tools 2022 with MSVC 14.44 and SDK
10.0.26100.0. Check free disk before downloading dependencies and compiling.

## 2. Bootstrap tools only when missing

Use the official Rust installer from static.rust-lang.org. Its basename must be
rustup-init.exe: the executable dispatches by filename, and a custom basename
can fail with unknown proxy name. Install native MSVC stable, rustfmt, Clippy,
rust-analyzer and wasm32-unknown-unknown. The workspace requires Rust >=1.95;
1.99.1 was installed in this session, not pinned as a permanent requirement.

Installer invocation once the official file is downloaded:

```powershell
& "$env:TEMP\rustup-init.exe" -y --profile minimal --default-toolchain stable --component rustfmt,clippy --target wasm32-unknown-unknown
```

Refresh the current shell without changing machine-wide PATH:

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
rustup component add rust-analyzer
rustc --version
cargo --version
rustup target list --installed
```

New terminals inherit rustup's user PATH update. Install Trunk from its official
trunk-rs/trunk GitHub release, or cargo install trunk --locked. Match Windows
architecture and verify release checksum before copying a downloaded binary to
the user cargo bin. Session version: 0.22.0. Use the current official repository
for downloads; an old trunkrs.dev documentation URL returned unrelated content
and was not used as an installation source. Trunk fetches matching auxiliary
wasm-bindgen/Binaryen tools when needed.

Installations and clones outside the workspace require the execution harness's
appropriate filesystem/network permission. Do not bypass a rejected approval.

## 3. Resolve source dependencies

```powershell
cargo fetch --locked
```

Clone craft-fonts as a sibling checkout, never into FigCraft. Read the font rev
from .github/workflows/ci.yml and check it out explicitly. Session revision:
37827adc41fd165f9a9234ac40f94dbad353aa69. .cargo/config.toml already supplies
CRAFT_FONTS_DIR. Missing fonts are optional for ordinary native builds; required
web releases enforce specific faces. Keep licence/provenance records in the
font repository; do not introduce font files into this repo.

The documented ../../craftrules resolves to D:\craftrules on this workstation.
Read its AGENTS.md when available. Repository not found does not prove whether
the source is private or removed. If CLI auth also returns 401, request an
accessible URL/local checkout or user-managed authentication; keep unaffected
setup moving. Do not copy credentials or infer access. This checkout remained
pending at session end.

## 4. Pull shared AI config into FigCraft

The shared source is D:\UnityProject\claude-unity-shared. Resolve direction from
the user's intent: pull means shared to project; push exports manifested project
copies to the shared source and is a different operation. Root FigCraft
instructions and project-owned knowledge must not become Universal templates.

Run from the project root with project.config.json present:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File D:\UnityProject\claude-unity-shared\sync.ps1 pull -SharedRepo D:\UnityProject\claude-unity-shared -WhatIf
# Inspect all listed writes, retirements, missing sources and generated warnings.
powershell -NoProfile -ExecutionPolicy Bypass -File D:\UnityProject\claude-unity-shared\sync.ps1 pull -SharedRepo D:\UnityProject\claude-unity-shared
node .claude/tools/ai-audit/check-ai-index.js --project .
```

Use this child PowerShell invocation if direct script execution is blocked; do
not relax the machine's execution policy. project.config.json's Unity identity
keys are compatibility inputs, not proof of Unity architecture here. Project
context lives in .claude/rules/figcraft-project.md. Retain scoped shared examples
and make their applicability explicit in project-owned guidance.

Root registries regenerate only between GENERATED:REGISTRY markers. No markers
means the root is skipped, not integrated. Preserve root instructions when
adding markers. .cursor/playbooks/README.md triggers a generated discovery link.
The memory hook needs the canonical file and HUB rows; a clean checker alone can
also mean no canonical index exists, so exercise the hook after adoption.

Repeat -WhatIf after applying: expect zero changes/warnings. Do not alter shared
source files or add FigCraft-only content to its manifest as part of a pull.

## 5. Build and verify

```powershell
cargo build --workspace --locked -j 4
cargo build --locked -p figcraft -j 4
cargo xtask wasm
```

The first native workspace build took about 10 minutes on this machine. Capture
logs under plan/setup/ if useful; use process exit status and final messages as
evidence. Unused dependency patch warnings were nonfatal. Windows PowerShell
may display stderr warning text as NativeCommandError records in redirected
logs, even when Cargo exits zero.

If cargo xtask silently rebuilds many host dependencies after a workspace
build, quiet alias output and feature unification can explain it. Check owned
processes and resource use before calling it a hang. An already-built native
task runner can avoid that launcher rebuild:

```powershell
$env:CARGO_BUILD_JOBS = '4'
.\target\debug\xtask.exe wasm
.\target\debug\xtask.exe layers
.\target\debug\xtask.exe assets
```

This does not skip the checks the task runner itself performs. Do not kill all
Cargo/rustc processes; stop only a positively identified session-owned process
tree when necessary. Preserve Cargo.lock and do not cargo update merely to
silence patch warnings.

For a meaningful runtime smoke check, create a synthetic rectangle through the
CLI, save .figcraft, reopen info and render PNG. Session fixtures and logs are
under plan/setup/. No personal documents are needed. Native compilation plus
this smoke check is not a substitute for the full cargo xtask ci commit gate.
Full native tests and a complete Trunk release bundle were not run in-session.

Browser development uses trunk serve from apps/figcraft-web, localhost:8767.
The HTML selects web-release; dist/web is build output. cargo xtask wasm checks
the browser target and builds/tests plugins, but does not prove a Trunk bundle
was emitted or served successfully.

## 6. Launch or reopen the visible app

```powershell
$app = Start-Process -FilePath 'D:\Projects\Figcraft\target\debug\figcraft.exe' -WorkingDirectory 'D:\Projects\Figcraft' -WindowStyle Normal -PassThru
```

The user explicitly requested a visible app. Background helper/service launches
otherwise follow the harness's hidden-window convention. Allow startup time,
refresh the process, check HasExited, MainWindowHandle and Responding. A returned
PID alone does not prove the window opened. Reacquire process state on each
request; never reuse an old PID. If the user closed the app and requests another
launch, start it again; do not forcibly terminate other sessions or lose work.

## 7. MCP checks and usage

See docs/mcp.md and the MCP/plugin topic memory. Run the built CLI directly:

```powershell
.\target\debug\figcraft-cli.exe mcp
```

initialize and tools/list were verified over newline JSON-RPC. There were 18
tools in version 0.11.2. For JSON protocol probes from PowerShell, prefer a
single-quoted here-string piped to python - over a quote-heavy python -c string;
the latter lost embedded quotes in this session. Parse actual responses rather
than treating exit zero alone as a successful handshake.

To operate a visible app, start it with --control and attach to that same free
loopback port. Merely launching figcraft.exe does not enable attachment. Avoid
closing a document just to add a port; preserve work and obtain a safe new
session as appropriate. No MCP connector registration was completed in-session.

## 8. Guide plugin installation precisely

1. Open a document. Click the FigCraft logo at the top of the narrow far-left
   rail; its tooltip is Main menu.
2. Choose Plugins > Development > Import plugin from manifest...
3. For the built example, choose
   D:\Projects\Figcraft\target\plugins\dist\swatch-grid\figcraft-plugin.json.
   Keep swatch_grid.wasm beside it. Build outputs can be regenerated by the
   plugin/WASM task if absent.
4. Choose Plugins > Swatch grid > Palettes..., select Ocean, then Generate.
5. Use From hex colours... for custom colors, swatch size and spacing. The
   palette panel itself uses fixed size 64 and gap 16; it controls palette,
   columns, labels and label color.

Generated swatches and hex-label text are editable layers. Other built examples
are tidy-names, counter and export-names. Manage plugins > Installed lists
installed plugins; widgets have a separate Plugins > Widgets section. FigCraft
plugins are its own sandboxed WASM format; Figma plugins do not run directly.
Reading source and giving these instructions does not establish that the user
has actually imported or run the plugin.

## 9. Preserve experience and verify discovery

Keep topic facts in project-owned indexed memories, procedures in this playbook,
dated session history in its session memory, and bulky logs in gitignored plan/.
Record limits, failures and pending access, not just successful outcomes.
Never persist secrets, unstable PIDs or assumed enduring user preferences.

After edits: run the AI index checker, exercise the prompt hook, regenerate root
registries via the shared pull, repeat a zero-change preview and git diff --check.
Do not rerun a lengthy native build merely for prose changes. Before a commit,
follow the repository's full quality gates; no commit or push was made during
the setup session.
