# Plugin tool bugs

Project-owned. Updated 2026-10-27. Review scope: five findings from
9e7d2ce2..0e1c3665. Raw evidence stays in ignored plan/plugin-review/.

## Ledger

| ID | Type | Severity | Status | Title | Source | Evidence | Found | Updated | Resolution |
|---|---|---|---|---|---|---|---|---|---|
| BUG-0002 | bug | high | <span class="st-resolved">RESOLVED</span> | Nine Slice metadata normalization creates infinite dimensions | plugin_tools/nine_slice.rs persisted inset parsing | slice_oversized_metadata_cannot_corrupt_geometry failed with inf dimensions, then passed including native save/reopen | 2026-10-26 | 2026-10-27 | Positive finite inset bounds checked before minima are computed |
| BUG-0003 | bug | medium | <span class="st-resolved">RESOLVED</span> | Scale restores old Nine Slice border widths | cmd/arrange.rs scale_node and Nine Slice normalization | slice_scale_updates_borders_directly_and_nested failed with 10 instead of 20, then passed with undo/redo, subsequent resize and pixel assertions | 2026-10-26 | 2026-10-27 | Shared subtree scaling updates validated inset metadata; out-of-range scaling rolls back |
| BUG-0004 | bug | medium | <span class="st-resolved">RESOLVED</span> | Tiny Nine Slice crops silently become Fill | plugin_tools/nine_slice.rs composed crops and ImagePaint effective_crop | slice_unrenderable_crops_are_rejected_atomically failed then passed for tiny centers and zoomed source crops | 2026-10-26 | 2026-10-27 | Preflight rejects mappings the renderer would discard, before output creation |
| BUG-0005 | bug | medium | <span class="st-resolved">RESOLVED</span> | Batch trace output is covered by later source images | plugin_tools/trace.rs stored sibling insertion indexes | trace_batch_places_each_result_above_its_live_source failed then passed for both outputs and selection orders plus undo/redo | 2026-10-27 | 2026-10-27 | Mutation resolves each source's current sibling position |
| BUG-0006 | bug | medium | <span class="st-resolved">RESOLVED</span> | Grid cleanup falsely protects Fixed and absolute grid children | plugin_tools/grid.rs track-based dimension protection | grid_fixed_children_snap_independently_of_track_sizing failed then passed; Fill protection regression also passed | 2026-10-27 | 2026-10-27 | Dimension protection uses child sizing and text auto-sizing rather than parent track sizing |

## History and limits

- 2026-10-27: Reproduced all five failures before production edits. Applied one
  root-cause fix at a time and passed its focused regression. Combined plugin
  suite passed 64 tests including all four packaged WASM runtime tests. Read-only
  follow-up review found no remaining blockers in these five fixes. Full CI and
  visual verification pending at this entry's creation.
- Separate renderer BUG-0001 remains outside this fix scope.
- 2026-10-27: Full `cargo xtask ci` exited 0. Synthetic panel export was
  visually inspected: 300x180 after 2x Scale and resize with 30-unit borders.
  Workspace checks, WASM checks and package builds passed. Existing unrelated
  WASM warnings remain. The five records are resolved by source changes and
  regressions; this does not qualify exact third-party plugin parity.
