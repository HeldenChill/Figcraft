# Rendering bugs

Project-owned. Last updated 2026-10-26. Historical evidence is not a current build
qualification. Local reproduction documents and screenshots are not committed.

## Ledger

| ID | Type | Severity | Status | Title | Source | Evidence | Found | Updated | Resolution |
|---|---|---|---|---|---|---|---|---|---|
| BUG-0001 | bug | high | <span class="st-confirmed">CONFIRMED</span> | Hard Light image rendering panics in tiny-skia debug arithmetic and closes the app | Owner-reported .fig load; tiny-skia 0.12.0 lowp Hard Light and wide/u16x16_t.rs:172:17 | Local plan/setup/fig-gui-stderr.log; CLI import exit 0; workaround log and visually inspected screenshot | 2026-10-26 | 2026-10-26 | Temporary package-only overflow-check build override verified; permanent source fix and regression test pending |

## Reproduction and limits

Owner's roughly 309 MB `.fig` imported successfully in the existing headless CLI,
reporting 112,021 layers and 1,673 images. GUI loading then panicked with
`attempt to multiply with overflow`; backtrace passed through tiny-skia's lowp
Hard Light stage and Rayon band rendering. Both expressions of the vectorized
blend branch are evaluated; an unused u16 multiplication can overflow in debug.

Verified workaround rebuild:

```powershell
cargo --config 'profile.dev.package.tiny-skia.overflow-checks=false' build --locked -p figcraft -j 4
```

The rebuilt GUI rendered the original document and answered control inspection
and screenshots without the captured panic. No dependency source, tracked Cargo
profile or production code was changed. Ordinary rebuilds may reintroduce it.
This is not full file-fidelity, release, wasm or CI verification.

Before a permanent fix, create a minimal synthetic Hard Light regression fixture;
do not add the owner's document/artwork to tests. Preserve overflow checks in
FigCraft's own code. Check current dependency version/source and any work Gemini
has landed before assuming this bug remains unchanged.

## History

- 2026-10-26: Reproduced GUI failure, separated parsing from rendering, identified
  dependency arithmetic path, and verified temporary rebuild workaround.
- 2026-10-26: Recorded as confirmed rather than resolved; implementation of the
  four requested plugins was handed to Gemini separately.
