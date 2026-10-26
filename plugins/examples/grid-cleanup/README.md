# Grid Cleanup

Clean-room FigCraft plugin (`org.figcraft.tools.grid-cleanup`) to snap layout and geometry values to a 4px or 8px grid.

## Features

- **Grid Steps**: 4px or 8px nearest rounding (ties round away from zero).
- **Properties**:
  - `positions`: Relative position (disabled by default; respects auto-layout flow).
  - `dimensions`: Width and height (disabled by default; respects Hug/Fill and minimum bounds).
  - `padding`: Auto-layout padding (enabled by default).
  - `gaps`: Auto-layout main gap and cross gap (enabled by default; respects auto-distributed gaps).
  - `radii`: Corner radii (enabled by default).
- **Token Binding**: Optionally creates or reuses owned tokens under `FigCraft / Grid Cleanup`.
- **Binding Replacement**: Safely replace existing variable bindings when explicitly enabled.

## Permissions

- Requires `write` access and the `arrange.snapToGrid` command.
