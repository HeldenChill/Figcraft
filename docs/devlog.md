# Development log

## 2026-10-27 — M11 plugin review fixes

Five regression cases reproduced the reviewed failures: oversized Nine Slice
metadata produced infinite geometry, Scale restored old border widths, tiny
cell crops fell back to Fill, batch traces used stale stacking positions, and
Grid Cleanup protected Fixed children based on their parent track sizing.

The fixes validate persisted border units before normalization, scale those
units through the shared subtree Scale path, reject unsupported composed crops
before creating nodes, resolve trace insertion positions against live siblings,
and protect dimensions according to child sizing rather than grid tracks.

Regression coverage includes save/reopen, nested scaling, subsequent resize,
undo/redo, both trace outputs and selection orders, and Fixed/absolute children
in fixed and flexible grid tracks.

Verification: all five regressions failed before their fixes and passed after.
The combined plugin suite passed 64 tests, including four built-WASM runtime
cases. `cargo xtask ci` passed formatting, workspace Clippy, workspace tests and
doctests, layering, asset attribution, WASM checks and plugin package builds.
Read-only follow-up review found no blockers. A freshly built CLI exported a
synthetic 300x180 panel after 2x Scale and resize; the PNG was visually inspected
and retained 30-unit corners and borders. Local evidence is in
`plan/plugin-review/` and is not committed. WASM checks emitted existing
warnings in unrelated font/plugin and collaboration paths.
