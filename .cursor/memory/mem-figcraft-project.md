# FigCraft project knowledge

Observed: 2026-10-26. Revalidate version and progress against source and roadmap.

## Identity and scope

- Workspace: D:\Projects\Figcraft; upstream declared as storytold/figcraft.
- Cargo workspace version 0.11.2, Rust edition 2024, minimum Rust 1.95.
- UI dependencies: egui/eframe 0.37, pinned to reviewed forks in Cargo.toml.
- MIT OR Apache-2.0 code; clean-room implementation of Figma design workflows.
- Targets: Windows, macOS, Linux, FreeBSD and browser WASM.
- Desktop app, CLI, collaboration server and web app under apps/.
- Inspected tree had 17 library crates and 848 Rust files under crates/ and apps/.
  These are dated inventory counts, not a maintained metric.

## Architecture

| Layer | Crates and responsibility |
|---|---|
| L0 | geom and color: geometry, paints and color primitives |
| L1 | doc: serializable persistent document model |
| L2 | layout, text, pathops, plugin, collab: layout, shaping, vector operations, sandbox, collaboration core |
| L3 | render and format: CPU canvas rendering and document formats |
| L4 | svg and pdf: vector file workflows |
| L5 | engine: commands, sessions, tools, undo/redo and application behavior |
| L6 | ui-egui, mcp, collab-net: frontend, automation adapter and network transport |
| Apps | figcraft, figcraft-cli, figcraft-server, figcraft-web |

The UI is thin and swappable. Menus, shortcuts, CLI, control channel, MCP and
plugins share command IDs. Commands request UI; programmatic calls do not open
dialogs. cargo xtask layers enforces the dependency direction. The canvas uses
CPU rendering; using wgpu to present the desktop does not make it a GPU canvas.

## Features and limits

Implemented areas include frames, vectors, text, auto layout, components and
variants, styles, variables, prototyping, developer inspection and collaboration
previews. Native .figcraft files, SVG/PDF/images, best-effort .fig and .sketch
import, and experimental .fig export are present. Consult current format docs
and code for exact coverage.

ROADMAP.md labeled the app alpha, estimated feature breadth around 85% and
real-work readiness around 53%, and measured menu coverage 212/212. These were
documentation claims read in the session, not independently remeasured parity.
Ranked gaps include real-file .fig fidelity, multiplayer/accounts, interaction
fidelity, localization review, plugin ecosystem, GPU rendering, platform QA,
accessibility and AI features. Figma plugins do not run directly.

docs/architecture.md contains stale statements, notably that no .fig reader
exists; crates/format/src/fig and the CLI contradict that statement. Cargo/source
and newer area docs should settle such conflicts. Do not repeat its old line
counts as current measurements.

## Project constraints

- Follow root AGENTS.md for clean-room boundaries, original/licensed assets and
  attribution, no third-party product imagery, and fonts outside this repository.
- Unsafe code is forbidden; shipped code must avoid panic/unwrap/expect paths.
- Preserve WASM support. Full pre-commit gate: cargo xtask ci.
- Fonts: optional sibling craft-fonts; standards: documented ../../craftrules.
- Universal AI rules came from a Unity-oriented shared repo. Unity assembly,
  Locator and character-system examples do not establish architecture here.

## Source pointers

Cargo.toml; AGENTS.md; README.md; ROADMAP.md; docs/gaps.md;
docs/target-app-parity.md; docs/architecture.md; docs/control-protocol.md;
docs/mcp.md; docs/plugins.md; xtask/src/layers.rs; crates/engine/src/cmd/.
