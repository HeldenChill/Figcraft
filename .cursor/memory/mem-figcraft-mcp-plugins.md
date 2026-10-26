# FigCraft MCP and plugin knowledge

Observed: 2026-10-26. Reference docs/mcp.md and docs/plugins.md for current API.

## MCP

FigCraft ships an MCP server in crates/mcp, exposed by figcraft-cli. It speaks
newline-delimited JSON-RPC over stdio. initialize succeeded with protocol
2025-06-18, server name figcraft and version 0.11.2. Docs also list 2025-03-26 and
2024-11-05 client compatibility; those versions were not tested in this session.

Headless invocation from the project root:

```powershell
.\target\debug\figcraft-cli.exe mcp
```

Attached mode uses a matching loopback control port on a separately running app:

```powershell
.\target\debug\figcraft.exe --control 7979
.\target\debug\figcraft-cli.exe mcp --attach 7979
```

Pick a free port, not necessarily 7979. Initial setup launches had no --control
argument. Later same-day crash diagnostics did launch with a dynamically selected
free port and verified direct JSON control inspection/screenshot. MCP --attach
itself and client connector registration were not exercised. No connector config
was added; do not infer registration from a working CLI or control socket.

tools/list returned these 18 names: list_commands, run_command, batch,
create_node, set_properties, inspect_document, get_node, select, export,
screenshot, open_file, save_file, undo, redo, pointer, key, type_text, inspect_ui.

Headless --automation-read-root and --automation-write-root can confine file
access; directories must already exist. These flags govern headless access and
must not be assumed to secure an attached app. Incoming frames have a 32 MiB
limit. Attached mutations are not automatically replayed after transport errors
because the first attempt may already have changed the document.

## Plugins and menu location

FigCraft uses its own WebAssembly plugin format, not Figma's plugin runtime.
The sandbox uses wasmi with instruction/memory limits and no WASI filesystem,
network or clock. Plugins request read/write access and allowed engine commands.
API 2 supports panels/settings; API 3 adds widgets, API 4 adds save-file support.
A collaboration server
can host a plugin directory; none was hosted during this session.

The main menu is behind the FigCraft logo at the top of the narrow far-left
icon rail, with tooltip Main menu. Plugins is inside that menu. Source:
crates/ui-egui/src/layers.rs (rail), menus.rs and plugins_ui.rs.

Import: logo > Plugins > Development > Import plugin from manifest...
Select a figcraft-plugin.json, keeping its referenced .wasm beside it. Installed
plugins appear in Plugins. Commands with parameters open a form; click Run.
Manage plugins > Installed shows installed entries and has an Import button.
Widgets are under Plugins > Widgets. No UI plugin import was verified by this
agent; later control-channel screenshots did not establish plugin installation.

Four example packages were built under target/plugins/dist/: tidy-names,
swatch-grid, counter and export-names. Each has figcraft-plugin.json plus WASM.
This describes initial setup, before Gemini's later four-tool implementation.
The separate plugins/ workspace provides the Rust SDK and examples. Regenerate
them with cargo xtask plugins or the broader cargo xtask wasm gate. target/
contents are build artifacts, not durable source files.

## Swatch Grid

Example manifest: plugins/examples/swatch-grid/figcraft-plugin.json.
Built manifest on this machine:
D:\Projects\Figcraft\target\plugins\dist\swatch-grid\figcraft-plugin.json.
Observed plugin version 1.2.0, API 2, ID org.figcraft.examples.swatch-grid.

It creates editable colored rectangle layers in a grid centered in the view,
optionally with editable hex-label text, and selects the created layers.

- Palettes... opens a panel with Sunset, Ocean, Forest and Greys. Choose a
  palette and Generate. The panel remembers the chosen palette, controls columns,
  labels and label color, and uses fixed 64-unit swatches with 16-unit gaps.
- From hex colours... accepts custom colors and controls columns, swatch size,
  gap and labels. Defaults: 3 columns, size 64, gap 16, labels enabled.
- Custom colors accept #RGB or #RRGGBB, optional #, separated by commas,
  whitespace, semicolons or newlines. Invalid entries are logged/skipped.
- Limits from source: 256 swatches, 1-24 columns, size 8-512, gap 0-256.

Prior user guidance about adjusting size/gap refers specifically to From hex
colours..., not the built-in-palette panel. This distinction comes from source;
the UI was not exercised during the session.

## Later four-tool request and active handoff

Owner requested original equivalents of Image Tracer, 9-Slice Scaling (New),
Make images fit and 8×4-Point Grid System. Research, unknown reference details,
approved design, exact planned IDs/contracts and Gemini assignment are in
[import/render/plugin handoff memory](mem-figcraft-import-render-and-plugin-handoff.md).
Master plan is local `.cursor/plans/implement/2026-10-26-game-ui-plugin-pack.md`.
Owner says Gemini is implementing; do not claim new packages exist or run another
implementation until its output is reviewed. Plans are ignored by Git by default.
