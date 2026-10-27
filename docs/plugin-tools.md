# FigCraft Plugin Tools

This document describes the FigCraft Game UI plugin pack: original Rust/WASM plugins designed for fast, non-destructive game interface asset and layout manipulation.

## Fit Images

**Plugin ID:** `org.figcraft.tools.fit-images`  
**Package:** `plugins/examples/fit-images`  
**Command:** `image.fit`  
**Manifest:** `fit_images.wasm` (API 2)

### Intent & Behavior

Fit Images changes the image fill scale mode between **Fit** and **Fill** across selected layers without altering layer dimensions, rotation, or image adjustments (exposure, contrast, saturation, etc.).

- Operates on visible image fills (`PaintKind::Image`).
- Preserves stroke paints, solid fills, and non-image fills untouched.
- Detaches layer fill style reference (`styles.fill`) only when an actual scale mode change occurs; unchanged layers retain their styles.
- Supports optional recursive traversal (`descendants: true`), deduplicating nested selections and pruning protected subtrees.
- Preserves selection and viewport.

### Panel Controls

- **Mode:** Choice between `fit` and `fill` (default: `fit`).
- **Include descendants:** Checkbox (default: `false`).
- **Apply:** Runs `image.fit` on the active selection.
- **Close:** Closes the settings panel.

### Headless & CLI Contract

```json
{"command": "image.fit", "params": {"ids": [12], "mode": "fit", "descendants": false}}
```

Result returns a standardized `ToolReport`:
```json
{
  "visited": 1,
  "changed": 1,
  "skipped": 0,
  "protectedFields": 0,
  "created": [],
  "warnings": []
}
```

### Limitations & Protection

- Design documents only; board documents are rejected with a Disabled error.
- Locked and hidden layers (or layers with locked/hidden ancestors) are protected and skipped.
- Layers inside immutable instances or widgets are protected.
- Missing selection or missing IDs return a BadParams error.

## Nine Slice

**Plugin ID:** `org.figcraft.tools.nine-slice`  
**Package:** `plugins/examples/nine-slice`  
**Command:** `image.nineSlice`  
**Manifest:** `nine_slice.wasm` (API 2)

### Intent & Behavior

Nine Slice slices an image rectangle into a 3x3 resizable frame consisting of 9 child rectangles sharing the same image hash:
- 4 fixed corners (Top-Left, Top-Right, Bottom-Left, Bottom-Right) with fixed dimensions.
- 4 stretching edges (Top, Bottom stretch horizontally; Left, Right stretch vertically).
- 1 stretching center (stretches horizontally and vertically).
- Replaces source node at the same z-order and position, preserving parent and transforms.
- Sets up standard FigCraft pin constraints on all 9 children.
- Attaches `figcraft.tools.nineSlice` metadata to the parent frame and `figcraft.tools.nineSliceCell` to the children.
- Enforces minimum frame size (`left + right`, `top + bottom`), guarding against undersizing during manual resize or interactive drag.
- Scale scales the stored border widths too, including panels inside scaled frames or groups. Later resizing preserves those scaled borders.

### Panel Controls

- **Left:** Inset width in px (0 = 1/3 automatic).
- **Right:** Inset width in px (0 = 1/3 automatic).
- **Top:** Inset height in px (0 = 1/3 automatic).
- **Bottom:** Inset height in px (0 = 1/3 automatic).
- **Apply:** Runs `image.nineSlice` on the single selected image rectangle.
- **Close:** Closes the settings panel.

### Headless & CLI Contract

```json
{"command": "image.nineSlice", "params": {"ids": [12], "left": 16.0, "right": 16.0, "top": 16.0, "bottom": 16.0}}
```

Result returns a standardized `ToolReport`:
```json
{
  "visited": 1,
  "changed": 1,
  "skipped": 0,
  "protectedFields": 0,
  "created": [13],
  "warnings": []
}
```

### Limitations & Protection

- Design documents only; board documents return a Disabled error.
- Source layer must be exactly one rectangle with exactly one visible image fill (`Fill`, `Fit`, or `Crop`).
- Layers with visible strokes, effects, corner radii, or masks are rejected with descriptive errors.
- Insets must satisfy `left + right < width` and `top + bottom < height`.
- Persisted insets outside the positive finite range up to 1,000,000 are ignored. Scale refuses border widths outside that range.
- Crops too small for the renderer to preserve are rejected before any output is created; adjust the insets or source crop.
- Protected or locked layers return a BadParams error.

## Image Trace

**Plugin ID:** `org.figcraft.tools.image-trace`  
**Package:** `plugins/examples/image-trace`  
**Command:** `image.trace`  
**Manifest:** `image_trace.wasm` (API 2)

### Intent & Behavior

Image Trace converts bitmap images into editable vector layers or isolated masked frames.
- **Monochrome mode:** Traces silhouette based on luminance threshold (0–255, default 128) and alpha cutoff (1–255, default 16), with optional inversion.
- **Color mode:** Deterministic frequency-weighted median-cut color quantization producing up to 16 palette colors.
- **Clean cell-boundary contour walker:** Extracts closed vector paths with holes (EvenOdd winding), avoiding cross-diagonal bridges or pixelation artifacts.
- **Vector output:** Inserts a Group containing one Vector node per palette color directly above the source layer.
- **Mask output:** Inserts a clipped Frame containing a compound white Vector mask at the bottom and a clone of the source image on top, avoiding unintended sibling masking.

### Panel Controls

- **Mode:** `monochrome` or `color` (default: `monochrome`).
- **Threshold:** 0–255 (default: `128`).
- **Invert silhouette:** Checkbox (default: `false`).
- **Max colors:** 2–16 (default: `8`).
- **Alpha cutoff:** 1–255 (default: `16`).
- **Resolution:** 16–512 (default: `256`).
- **Output:** `vectors` or `mask` (default: `vectors`).
- **Apply:** Runs `image.trace` on selected image rectangles.
- **Close:** Closes the settings panel.

### Headless & CLI Contract

```json
{"command": "image.trace", "params": {"ids": [12], "mode": "monochrome", "threshold": 128, "invert": false, "colors": 8, "alphaCutoff": 16, "resolution": 256, "output": "vectors"}}
```

### Limitations & Protection

- Maximum 16 eligible sources per call.
- Maximum 512×512 raster tracing resolution; images >16M pixels or >32 MiB rejected before decoding.
- Source layer must be an unnested non-mask rectangle with exactly one visible image fill (`Fill`, `Fit`, or `Crop`), no strokes, effects, corner radii, or children.
- Cell-boundary paths have no curve smoothing in v1 (pixel-exact vector boundaries).
- Transparent/empty traces produce no nodes and report as skipped.

## Grid Cleanup

**Plugin ID:** `org.figcraft.tools.grid-cleanup`  
**Package:** `plugins/examples/grid-cleanup`  
**Command:** `arrange.snapToGrid`  
**Manifest:** `grid_cleanup.wasm` (API 2)

### Intent & Behavior

Grid Cleanup snaps layer dimensions, positions, auto-layout padding, gaps, and corner radii to a 4px or 8px grid using nearest-neighbor rounding (ties round away from zero).

- **Position Snapping:** Snaps local/frame-relative coordinates (disabled by default; respects auto-layout flow children).
- **Dimension Snapping:** Snaps width and height (disabled by default; respects Hug/Fill sizing and minimum dimensions).
- **Auto Layout Gaps & Padding:** Snaps main and cross gaps (respects auto-distributed SpaceBetween/SpaceAround gaps) and 4-way padding (enabled by default).
- **Corner Radii:** Snaps per-corner radii (enabled by default).
- **Token Binding:** Optionally binds snapped scalar values to reusable Number tokens in an owned variable collection `FigCraft / Grid Cleanup`. Tokens are shared across layers for equal values.
- **Safe Binding Replacement:** Leaves existing variable bindings untouched unless `replaceBindings: true` is explicitly passed.

### Panel Controls

- **Grid step:** `4` or `8` (default: `4`).
- **Include descendants:** Checkbox (default: `false`).
- **Positions:** Checkbox (default: `false`).
- **Dimensions:** Checkbox (default: `false`).
- **Padding:** Checkbox (default: `true`).
- **Gaps:** Checkbox (default: `true`).
- **Corner radii:** Checkbox (default: `true`).
- **Bind to owned variables:** Checkbox (default: `false`).
- **Replace existing bindings:** Checkbox (default: `false`).
- **Apply / Close:** Triggers or dismisses panel.

### Headless & CLI Contract

```json
{"command": "arrange.snapToGrid", "params": {"ids": [12], "step": 4, "descendants": false, "properties": {"positions": false, "dimensions": false, "padding": true, "gaps": true, "radii": true}, "bindVariables": false, "replaceBindings": false}}
```

### Limitations & Protection

- Design documents only; board documents return a Disabled error.
- Flow-child positions, text auto-resize, and Hug/Fill sizing are protected from accidental fixed snaps.
- Auto-distributed gaps are protected from being converted to fixed values.
- Maximum 512 new tokens created per invocation; maximum 4,096 tokens in owned collection; maximum 64 modes per collection.
- Reusable tokens are tracked using strict metadata markers (`figcraft.tools.grid-cleanup/v1;step=...;value=...`).


