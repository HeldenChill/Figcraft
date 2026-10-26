# FigCraft import, renderer diagnosis and plugin handoff

Observed and discussed: 2026-10-26. This records dated evidence and approved
planning; current app/process state still requires fresh inspection.

## Figma interchange

Owner asked whether a Figma design can be moved into FigCraft. Guidance supplied:
Figma Main menu > File > Save local copy produces `.fig`; open it through
FigCraft File > Open or drag it into the window, then save native `.figcraft`.
Import is best effort, with warnings; missing fonts can change layout. Figma's
local copy excludes comments/version history. Public source:
[Figma local-copy guide](https://help.figma.com/hc/en-us/articles/8403626871063-Save-a-local-copy-of-files).
FigCraft behavior was checked in `docs/file-format-parity.md`, not assumed from
the presence of an Open menu item.

## Confirmed renderer crash and local workaround

The authoritative bug record is in
[rendering bug memory](bugs/mem-bugs-rendering.md). Import diagnosis and recovery
procedure: [playbook](../playbooks/figcraft-import-and-plugin-handoff.md).

Owner reported the app closed while loading a `.fig` file from Downloads, named
`Pet Vs Monster - UI.fig` (308,729,635 bytes, about 309 MB). CLI `info` succeeded:
112,021 nodes, 4 styles, 127 variables and 1,673 images. Warnings included vector
segment settings, per-region fills, image-filter approximations and unsupported
PATTERN paints. Successful parsing did not establish rendering fidelity.

Launching the GUI with this file reproduced a panic in tiny-skia 0.12.0:
`src/wide/u16x16_t.rs:172:17`, `attempt to multiply with overflow`. Backtrace
included `pipeline::lowp::hard_light`, FigCraft drawing and Rayon band rendering.
Dependency source inspection showed eager evaluation of both Hard Light branches;
an unused u16 arithmetic branch can overflow with debug overflow checks enabled.
This diagnosis points to rendering, not a parser failure or an identified GPU fault.

Verified temporary build command:

```powershell
cargo --config 'profile.dev.package.tiny-skia.overflow-checks=false' build --locked -p figcraft -j 4
```

Build succeeded in about 2m24s. The rebuilt GUI opened the original file, answered
control requests, reported no `lastError`, rendered its game screens and remained
responding during observation. A screenshot was read visually. No permanent source
fix or regression test was implemented here. A normal rebuild can restore the
failure; do not label the bug resolved or disable checks across the whole workspace.

Local evidence under ignored `plan/setup/`:

| File | What it established |
|---|---|
| `fig-import-info.log`, `fig-import-error.log` | Headless info succeeded with counts and warnings, exit 0 |
| `fig-gui-stderr.log` | Original GUI renderer panic and backtrace |
| `fig-workaround-stderr.log` | Observed workaround run had no panic in captured stderr |
| `fig-workaround-response-1.json` | Control UI inspection returned ok and no lastError |
| `fig-workaround.png` | First screenshot, before detailed canvas content settled |
| `fig-workaround-frame.png` | Later screenshot visibly contained rendered game UI |

Evidence can be absent on another checkout. Screenshots contain owner artwork:
keep local, do not commit, publish, or repurpose as assets. Runtime session JSONs
contain temporary process/port values; rediscover them instead of copying into memory.

## Four reference plugins: research, not black-box testing

Owner supplied an image listing Image Tracer, 9-Slice Scaling (New), Make images fit
and 8×4-Point Grid System, then requested analysis and four FigCraft equivalents.
At inspection time FigCraft had its own WASM runtime and four existing examples,
not these Figma plugins. Figma JS plugins cannot run directly in that runtime.

| Reference | Evidence and conclusion | Remaining uncertainty |
|---|---|---|
| Image Tracer | Creator Dave Williames describes color/monochrome vector tracing and mask use; [creator page](https://dwilliames.gumroad.com/l/image-tracer) | Exact controls, transparency, simplification and batch behavior not exercised |
| 9-Slice Scaling (New) | Public listing identified; generic nine-slice technique fixes corners, stretches edges on one axis and center on both; [technique reference](https://docs.unity3d.com/es/2018.4/Manual/9SliceSprites.html) | Exact plugin layer structure, controls, export metadata and rasterization not verified |
| Make images fit | Icons8 describes select-and-run image fitting; [creator guide](https://dev.blog.icons8.com/articles/figma-plugins-for-design-nerds/amp/) | Public description did not establish fill-mode change versus layer resizing |
| 8×4-Point Grid System | Creator Youssef Mizani describes 4/8px snapping, selectable properties, variable creation/binding and auto layout protection; [creator announcement](https://www.linkedin.com/posts/youssefmizani_figma-uidesign-uxdesign-activity-7462210959269416960-2rNM) | Exact rounding, property set and existing-binding handling not tested |

Figma Community blocked direct web fetching. No source from Figma/plugins was
downloaded or inspected. No hands-on Figma plugin test was performed. These are
behavior-inspired original tools; do not claim exact reference-plugin parity.
The suggested priority for this game UI was nine-slice usefulness; implementation
order below differs to establish shared interfaces with the smallest operation.

## Approved design and Gemini execution

Owner approved the written design, requested the writing-plans skill for a detailed
Gemini implementation handoff, then stated Gemini was working. Preserve that
assignment: do not launch a competing implementation or rewrite its source.
No implementation subagents were requested. The later explicit code-review skill
invocation required one read-only reviewer for Nine Slice; no implementation was
delegated here.

Local files, intentionally named under project plan rules:

- `.cursor/plans/design/2026-10-26-game-ui-plugin-pack.md`: approved design.
- `.cursor/plans/implement/2026-10-26-game-ui-plugin-pack.md`: master entry point,
  global contracts, delivery steps and ready-to-paste Gemini prompt.
- Four same-directory child plans: `2026-10-26-fit-images.md`,
  `2026-10-26-nine-slice.md`, `2026-10-26-image-trace.md`,
  `2026-10-26-grid-cleanup.md`.

As observed, `.gitignore` pattern `plans/` ignores `.cursor/plans` too. Another Git
checkout will not receive these six local documents unless explicitly shared or
staged. Source tests and a `.gitignore` edit appeared after Gemini began; inspect
fresh commit/worktree state before deciding whether they landed.

| Planned plugin | Stable plugin ID | Planned engine command |
|---|---|---|
| Fit Images | `org.figcraft.tools.fit-images` | `image.fit` |
| Nine Slice | `org.figcraft.tools.nine-slice` | `image.nineSlice` |
| Image Trace | `org.figcraft.tools.image-trace` | `image.trace` |
| Grid Cleanup | `org.figcraft.tools.grid-cleanup` | `arrange.snapToGrid` |

Plan decisions: original Rust/WASM API-2 panels; algorithms behind engine commands;
offline operation, atomic undo and bounded traversal; Fit/Fill changes preserve
layer sizes; nine-slice reuses original image hashes/crops; tracing uses bounded
monochrome/color contours with holes and isolated masks; grid defaults protect
geometry/layout/bindings and optionally create owned numeric tokens.

Master requires actual built-WASM runtime tests, CI, synthetic screenshot review,
packaging and installation. None of those outcomes for the new four plugins was
verified by this agent. Plan integrity/link checks passed over six documents and
57 actionable checkboxes; those were document checks, not implementation tests.

## Reusable source observations for the executor

- `crates/plugin/src/manifest.rs` defines panel controls; boolean checkbox JSON uses
  `type: boolean`, not `checkbox`. Manifest defaults use `default`, panel values use `value`.
- Plugin ABI 2 adds panels/settings; ABI 3 adds widgets; ABI 4 adds save-file support.
- `plugins/` is a separate workspace; `xtask/src/main.rs::EXAMPLE_PLUGINS` controls
  which modules/manifests are copied to dist. Declaring a workspace package alone
  does not prove a distributable pair exists.
- `crates/render/src/paint.rs` already pads uncovered Fit/Crop sampling with
  transparency. Preserve that when composing nine-slice image crops.
- `props::resize_node` and layout `set_size` do not universally clamp manual sizes
  to `child_layout` minima. The plan explicitly handles generated nine-slice borders.
- Grid cleanup must read final normalized values; raw width/gap changes can be
  overridden by bindings or Hug/Fill/automatic spacing.
- Plugin milestone in inspected `docs/roadmap.md` is M11; M9 is multiplayer.

## Gemini progress check and Nine Slice review

At 16:30 local on 2026-10-26, the working tree showed Fit Images and Nine Slice
committed, Image Trace files changing and no visible Grid Cleanup implementation.
That snapshot was superseded. At 17:52, the tree was clean and all four plugin
commits plus build artifacts were present, matching the user's completion report.

Read-only Nine Slice review covered `1eed2407..ee44b677` plus current engine paths.
Fresh tests passed: 12 focused Nine Slice tests and one ignored runtime test that
loaded the built WASM. Cargo required prepending `$env:USERPROFILE\.cargo\bin` to
the PowerShell PATH. Full CI and screenshot checks were not run. Findings, no fixes:

| Severity | Trigger and effect |
|---|---|
| P1 | `parse_insets_metadata` accepts huge finite values such as `1e308`; after `node.set` changes dimensions, normalization sums them to infinity and persists invalid geometry after finite checks already ran. Bound each inset and sum; add regression coverage. |
| P2 | `transform.scale` scales frame/cells/native minima but not inset plugin metadata; normalization rebuilds cells with old borders, distorting the artwork. Scale metadata too, including nested panels. |
| P2 | Legal tiny insets can make composed crop coefficients smaller than `ImagePaint::effective_crop` accepts; renderer drops the crop and falls back to Fill. Support these mappings or reject them before mutation. |

Rendered pixel tests also remain absent for Fit/Crop transparency and quarter-turn
rotation; matrix tests alone do not establish sampling. No source changes were made
during review. Latest inspection at 17:52 found no subsequent Nine Slice fix, so
these findings remain unresolved. The plugin pack itself is complete in Git:
Fit Images `1eed2407`, Nine Slice `ee44b677`, Image Trace `72233901`, Grid Cleanup
`0d7c50b9`. The four `target/plugins/dist/` folders each held a manifest and WASM.
Commit `5631dd49` fixed test Clippy warnings. A full `cargo xtask ci` process had
been observed but its final exit/result was not captured; do not claim the CI gate
passed on that basis. No Gemini process or Cargo build remained at final check.

These are dated observations. Fresh source and implementation evidence govern
future answers.
