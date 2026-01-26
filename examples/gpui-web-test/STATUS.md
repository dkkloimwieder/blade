# GPUI WASM Test Demo - Current Status

**Date**: 2026-01-16
**Test URL**: http://localhost:8000 (via `cargo run-wasm --release --example gpui-web-test`)

## Executive Summary

The GPUI WASM correctness testing demo compiles and runs. **Performance is now acceptable with release builds** (~6-12ms per frame, 60+ FPS). The primary issue was building with dev profile instead of release.

---

## Performance Status

### RESOLVED: Build Profile Issue

**Root Cause**: WASM was being built with `dev` profile (unoptimized) instead of `release`.

| Metric | Dev Build | Release Build | Improvement |
|--------|-----------|---------------|-------------|
| Taffy layout | 33-71ms | 1.8-3.1ms | **~20x faster** |
| Frame duration | 60-130ms | **5.6-11.5ms** | **~10x faster** |
| FPS | 8-15 | **60+** | Target achieved |

**Fix**: Always use `--release` flag:
```bash
cargo run-wasm --release --example gpui-web-test
```

### Current Performance Breakdown (Release Build)
```
taffy tree: 215 nodes (counted in 0µs)
taffy measure: 762 calls, 0.50ms total (0.001ms avg)
taffy algo: 1.80ms (total 2.30ms - measure 0.50ms)
layout_as_root: 3.1ms
prepaint_tree: 100µs
prepaint_root: 3.6ms
paint_root: 400µs
layout+paint: 4.2ms
gpu_render: 800µs
frame duration: 5.6ms
```

### Remaining Optimization Opportunities

1. **Mouse move triggers full re-render** - Moving mouse causes complete layout+paint even when nothing changes visually
2. **Layout runs every frame** - No dirty tracking; entire tree re-laid-out on each frame
3. **762 measure calls** - Text elements measured 3.5x each (normal for flexbox, but could cache)

---

## Visual Rendering Issues

### Opacity/Alpha Blending (Q04 Test)
- **Test**: Q04 - Alpha/Opacity with overlapping divs at 0.25, 0.5, 0.75, 1.0
- **Status**: Needs verification with release build

### Sprite Rendering (I01-I04 Tests)
- **Test**: I01-I04 Sprite tests
- **Status**: I01 FIXED - stars now render with correct tint colors
- **Fix**: Narrowed emoji detection in `text_system.rs` to exclude Miscellaneous Symbols range (0x2600-0x27BF)

---

## Test Categories Status

| Category | Tests | Compiles | Renders | Correct |
|----------|-------|----------|---------|---------|
| Quads (Q01-Q14) | 14 | ✓ | ✓ | ? |
| Layout Flex (L01-L19) | 19 | ✓ | ✓ | ? |
| Sizing (S01-S06) | 6 | ✓ | ✓ | ? |
| Spacing (SP01-SP07) | 7 | ✓ | ✓ | ? |
| Text (T01-T11) | 11 | ✓ | ✓ | ? |
| Mouse Events (E01-E12) | 12 | ✓ | ✓ | ? |
| Keyboard Events (K01-K06) | 6 | ✓ | ✓ | ? |
| Scroll/Wheel (SC01-SC03) | 3 | ✓ | ✓ | ? |
| Drag/Drop (D01-D05) | 5 | ✓ | ? | ? |
| Focus (F01-F04) | 4 | ✓ | ? | ? |
| Tooltips (TT01-TT03) | 3 | ✓ | ? | ? |
| Sprites (I01-I04) | 4 | ✓ | ✓ | I01 ✓ |
| Stress (ST01-ST08) | 8 | ✓ | ? | ? |
| Shadows (SH01-SH05) | 5 | ✓ | ✓ | ✓ |
| Paths (P01-P04) | 4 | ✓ | ✓ | ✓ |
| Underlines (U01-U03) | 3 | ✓ | ✓ | ? |

Legend:
- ✓ = Confirmed working
- ? = Needs verification

---

## Profiling Instrumentation Added

The following timing instrumentation was added for debugging:

- `vendor/gpui-ce/src/window.rs` - Frame timing breakdown (layout, paint, GPU)
- `vendor/gpui-ce/src/taffy.rs` - Taffy layout timing (node count, measure calls, algo time)
- `vendor/gpui-ce/src/platform/web/text_system.rs` - Glyph rasterization timing
- `vendor/gpui-ce/src/platform/web/web_atlas.rs` - Cache hit/miss statistics
- `vendor/gpui-ce/src/platform/web/renderer.rs` - GPU render timing

---

## Reproduction Steps

1. Build and run with release: `cargo run-wasm --release --example gpui-web-test`
2. Open http://localhost:8000 in Chrome/Firefox
3. Open browser DevTools Console to see frame timing logs
4. Observe ~6-12ms frame duration (60+ FPS)

---

## Resolved Issues

- **blade-tfmy**: Layout recalculation slowness - Fixed by using release build
- **blade-13ip**: Emoji rasterization bug - Fixed BGRA handling for emojis
- **blade-xfdf**: Monochrome sprites (I01) - Fixed emoji detection to exclude Miscellaneous Symbols (0x2600-0x27BF)

## Open Issues

- **blade-gtnd**: Ensure WASM examples default to release build
- **blade-jsp3**: Opacity/alpha rendering verification
