# GPUI WASM Test Demo - Issue Details

This document provides detailed context for each beads issue created for the test demo.

---

## Performance Issues

### blade-cags: Profile frame timing breakdown
**Priority**: P1

**Problem**:
Frame duration is 50-70ms but we don't know where time is spent between layout, paint, and GPU rendering.

**Current State**:
- `measure("frame duration", ...)` wraps entire `window.draw(cx)` + `window.present()`
- No granular timing for individual phases
- Can't identify which phase is the bottleneck

**Investigation Steps**:
1. Add `measure()` around `window.draw(cx)` separately from `window.present()`
2. Add timing inside Scene construction (`scene.rs`)
3. Add timing around GPU `encoder.submit()`
4. Add timing around each `draw_quads_internal` call

**Files**:
- `vendor/gpui-ce/src/window.rs:1111` - frame duration measure
- `vendor/gpui-ce/src/platform/web/renderer.rs:444-529` - draw() method

**Success Criteria**:
Console shows: `layout: Xms, paint: Xms, gpu: Xms`

---

### blade-v47m: getImageData bottleneck
**Priority**: P1

**Problem**:
`context.get_image_data()` is a synchronous GPU readback that stalls the pipeline.

**Location**: `vendor/gpui-ce/src/platform/web/text_system.rs:396-399`

**Code**:
```rust
let image_data = state
    .context
    .get_image_data(0.0, 0.0, width as f64, height as f64)
    .map_err(|e| anyhow::anyhow!("get_image_data failed: {:?}", e))?;
```

**Why It's Slow**:
1. `getImageData` forces synchronous GPU→CPU transfer
2. Browser must flush GPU pipeline and wait
3. Each unique glyph triggers this on first render
4. Even cached glyphs cost the initial rasterization

**Potential Fixes**:
1. Batch multiple glyphs into single canvas, single readback
2. Use OffscreenCanvas with async readback (if available)
3. Pre-rasterize common glyphs at startup
4. Use WebGL/WebGPU for glyph rasterization instead of Canvas 2D

**Metrics to Collect**:
- Count of `rasterize_glyph` calls per frame
- Time spent in `get_image_data` specifically
- Cache hit rate for glyphs

---

### blade-5q3u: Glyph cache effectiveness
**Priority**: P1

**Problem**:
Unknown if glyph cache is working correctly across frames.

**Current Cache Implementation**:
- `vendor/gpui-ce/src/platform/web/web_atlas.rs:124` - cache lookup
- Key: `AtlasKey` (font_id, glyph_id, size, scale_factor, etc.)
- Storage: `FxHashMap<AtlasKey, AtlasTile>`

**Questions to Answer**:
1. Is the cache persisting between frames?
2. What's the cache hit rate?
3. Are cache keys too specific (causing misses)?
4. Is the atlas being cleared unexpectedly?

**Investigation**:
1. Add logging: `log::debug!("glyph cache HIT/MISS for {:?}", key)`
2. Count hits vs misses per frame
3. Check if atlas is cleared anywhere unexpected
4. Verify cache key includes only necessary fields

**Files**:
- `vendor/gpui-ce/src/platform/web/web_atlas.rs` - atlas implementation
- `vendor/gpui-ce/src/window.rs:3090-3094` - cache usage

---

### blade-nwkn: Batch fragmentation
**Priority**: P1

**Problem**:
4-5 separate quad batches per frame instead of 1 batched draw call.

**Evidence**:
```
draw_quads_internal: drawing 40 quads at offset 0
draw_quads_internal: drawing 16 quads at offset 6400
draw_quads_internal: drawing 8 quads at offset 8960
draw_quads_internal: drawing 3 quads at offset 10240
```

**Why This Matters**:
- Each batch = separate draw call
- Each draw call has overhead (state changes, sync)
- 4-5 draws vs 1 draw is 4-5x overhead

**Causes of Fragmentation**:
1. Z-ordering forces primitive interleaving
2. Content masks (overflow clipping) break batches
3. Different texture atlases break sprite batches
4. Scene construction order

**Investigation**:
1. Log why each batch boundary occurs
2. Check `BatchIterator` logic in `scene.rs`
3. Check if content masks are over-used
4. See if z-ordering can be optimized

**Files**:
- `vendor/gpui-ce/src/scene.rs:146-200` - BatchIterator
- `vendor/gpui-ce/src/platform/web/renderer.rs:467-520` - batch processing

---

### blade-6tar: sync_buffer_range overhead
**Priority**: P2

**Problem**:
`sync_buffer_range` called per batch may cause GPU stalls.

**Location**: `vendor/gpui-ce/src/platform/web/renderer.rs:573`

**Current Code**:
```rust
gpu.sync_buffer_range(quad_buffer, buffer_offset, data_size);
```

**Investigation**:
1. Check if this triggers immediate GPU sync
2. Check if dirty range tracking is efficient
3. Consider batching all syncs together

**Files**:
- `blade-graphics/src/webgpu/mod.rs:894-896` - sync_buffer_range impl
- `blade-graphics/src/webgpu/mod.rs:899-909` - mark_buffer_dirty_range

---

### blade-tfmy: Layout recalculation
**Priority**: P2

**Problem**:
Layout may be recalculated every frame even when nothing changed.

**Investigation**:
1. Check if layout is cached
2. Check what triggers layout invalidation
3. Check if hover events cause full relayout

**Files**:
- `vendor/gpui-ce/src/window.rs` - layout logic
- `vendor/gpui-ce/src/taffy.rs` - layout engine

---

### blade-02pr: FPS counter display
**Priority**: P2

**Problem**:
Need visible FPS counter to monitor performance during testing.

**Implementation**:
1. Add moving average of frame times (last 60 frames)
2. Display FPS and frame time in corner of demo
3. Color code: green (<16ms), yellow (16-33ms), red (>33ms)

---

## Visual Issues

### blade-jsp3: Opacity/alpha incorrect (Q04)
**Priority**: P2

**Test**: Q04 - Alpha/Opacity test
**Expected**: Overlapping divs at 0.25, 0.5, 0.75, 1.0 opacity show smooth blending
**Observed**: Needs verification - user reported "looks off"

**Possible Causes**:
1. Incorrect blend mode in WebGPU
2. Premultiplied alpha mismatch
3. Incorrect alpha value in Hsla→shader conversion

---

### blade-j5ww: Alpha blending mode
**Priority**: P2

**Check**: WebGPU blend state configuration

**Location**: `vendor/gpui-ce/src/platform/web/renderer.rs` - pipeline creation

**Expected Blend Mode** (standard alpha blending):
```
srcFactor: SrcAlpha
dstFactor: OneMinusSrcAlpha
```

---

### blade-nh8n: Premultiplied alpha
**Priority**: P2

**Check**: Are colors being premultiplied correctly?

**Common Issue**:
- GPU expects premultiplied: `rgb * a`
- Code sends straight alpha: `rgb`
- Result: incorrect blending

**Files to Check**:
- Hsla to shader uniform conversion
- Quad shader color handling
- WebGPU texture format (RGBA8 vs RGBA8Premultiplied)

---

### blade-xfdf: Monochrome sprites (I01)
**Priority**: P2

**Test**: I01 - Monochrome Sprite (stars ★)
**Expected**: Unicode symbols rendered with tint color
**Observed**: May not be rendering at all

**Investigation**:
1. Check if monochrome sprite batches exist in scene
2. Check atlas lookup for text glyphs
3. Check monochrome sprite shader
4. Add debug logging for sprite rendering

**Files**:
- `vendor/gpui-ce/src/platform/web/renderer.rs:481-497` - mono sprite drawing
- `vendor/gpui-ce/src/platform/web/renderer.rs:599-665` - draw_mono_sprites_internal

---

### blade-57mv: Polychrome sprites (I02)
**Priority**: P2

**Test**: I02 - Polychrome Sprite
**Expected**: Full-color images rendered correctly
**Observed**: May not be rendering

**Investigation**:
Same as monochrome but for polychrome pipeline.

**Files**:
- `vendor/gpui-ce/src/platform/web/renderer.rs:499-516` - poly sprite drawing
- `vendor/gpui-ce/src/platform/web/renderer.rs:668-733` - draw_poly_sprites_internal

---

### blade-c7wo: Sprite atlas lookup
**Priority**: P2

**Check**: Is `atlas.get_texture_info(texture_id)` returning valid data?

**Location**: `vendor/gpui-ce/src/platform/web/renderer.rs:482, 500`

**Verification**:
1. Log when texture_id lookup fails
2. Check if atlas textures are created
3. Verify texture view is valid

---

### blade-mikw: Sprite debug logging
**Priority**: P3

**Add logging**:
1. Count of mono sprites per batch
2. Count of poly sprites per batch
3. Atlas texture lookup success/failure
4. Sprite bounds and positions

---

## Issue Dependencies

```
blade-cags (Profile timing)
    ↓
    ├── blade-v47m (getImageData)
    │       ↓
    │       └── blade-5q3u (Glyph cache)
    │
    ├── blade-nwkn (Batch fragmentation)
    │
    └── blade-6tar (Buffer sync)

blade-jsp3 (Opacity incorrect)
    ↓
    ├── blade-j5ww (Blend mode)
    └── blade-nh8n (Premultiplied alpha)

blade-xfdf (Mono sprites)
    ↓
    └── blade-c7wo (Atlas lookup)

blade-57mv (Poly sprites)
    ↓
    └── blade-c7wo (Atlas lookup)
```

---

## Quick Reference

| Issue | Title | Priority |
|-------|-------|----------|
| blade-cags | Profile frame timing breakdown | P1 |
| blade-v47m | getImageData bottleneck | P1 |
| blade-5q3u | Glyph cache effectiveness | P1 |
| blade-nwkn | Batch fragmentation | P1 |
| blade-6tar | sync_buffer_range overhead | P2 |
| blade-tfmy | Layout recalculation | P2 |
| blade-02pr | FPS counter display | P2 |
| blade-jsp3 | Opacity/alpha incorrect | P2 |
| blade-j5ww | Alpha blending mode | P2 |
| blade-nh8n | Premultiplied alpha | P2 |
| blade-xfdf | Monochrome sprites | P2 |
| blade-57mv | Polychrome sprites | P2 |
| blade-c7wo | Sprite atlas lookup | P2 |
| blade-mikw | Sprite debug logging | P3 |
