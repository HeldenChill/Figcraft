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
- Protected or locked layers return a BadParams error.

