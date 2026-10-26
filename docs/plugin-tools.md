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
