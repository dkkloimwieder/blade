# BladeRenderer Scene Primitive Rendering Analysis

> Research for GPUI WASM WebRenderer port

## 1. Scene Structure

The `Scene` (`vendor/gpui-ce/src/scene.rs`) contains 7 primitive types:

| Primitive | Purpose | Key Fields |
|-----------|---------|------------|
| `Shadow` | Drop shadows | blur_radius, bounds, corner_radii, color |
| `Quad` | Rectangles/buttons | bounds, background, border_color, corner_radii, border_widths |
| `Path` | Vector paths | vertices (PathVertex), color (Background) |
| `Underline` | Text underlines | bounds, color, thickness, wavy |
| `MonochromeSprite` | Glyphs (text) | bounds, color, tile (AtlasTile), transformation |
| `PolychromeSprite` | Images/emoji | bounds, tile, grayscale, opacity, corner_radii |
| `Surface` | Video (macOS-only) | bounds, CVPixelBuffer |

### Batching System

`scene.batches()` yields `PrimitiveBatch` in draw order:

```rust
pub(crate) enum PrimitiveBatch<'a> {
    Shadows(&'a [Shadow]),
    Quads(&'a [Quad]),
    Paths(&'a [Path<ScaledPixels>]),
    Underlines(&'a [Underline]),
    MonochromeSprites { texture_id: AtlasTextureId, sprites: &'a [MonochromeSprite] },
    PolychromeSprites { texture_id: AtlasTextureId, sprites: &'a [PolychromeSprite] },
    Surfaces(&'a [PaintSurface]),
}
```

- Primitives sorted by `order` field (z-index)
- Sprites additionally batched by `texture_id` to minimize texture binds

---

## 2. BladeRenderer Architecture

### 2.1 Pipelines (`blade_renderer.rs:126-135`)

```rust
struct BladePipelines {
    quads: gpu::RenderPipeline,
    shadows: gpu::RenderPipeline,
    path_rasterization: gpu::RenderPipeline,  // Intermediate pass
    paths: gpu::RenderPipeline,               // Copy from intermediate
    underlines: gpu::RenderPipeline,
    mono_sprites: gpu::RenderPipeline,
    poly_sprites: gpu::RenderPipeline,
    surfaces: gpu::RenderPipeline,            // macOS-only
}
```

### 2.2 Shader Data Structs

Each pipeline has its own `#[derive(ShaderData)]` struct:

```rust
// Common to all pipelines
struct GlobalParams {
    viewport_size: [f32; 2],
    premultiplied_alpha: u32,
    pad: u32,
}

// Quads
struct ShaderQuadsData {
    globals: GlobalParams,
    b_quads: gpu::BufferPiece,  // Storage buffer
}

// Sprites (text)
struct ShaderMonoSpritesData {
    globals: GlobalParams,
    gamma_ratios: [f32; 4],
    grayscale_enhanced_contrast: f32,
    t_sprite: gpu::TextureView,  // Atlas texture
    s_sprite: gpu::Sampler,
    b_mono_sprites: gpu::BufferPiece,
}
```

### 2.3 Key Resources

| Resource | Type | Purpose |
|----------|------|---------|
| `instance_belt` | BufferBelt | Staging per-frame instance data |
| `atlas` | BladeAtlas | Shared texture atlas for sprites |
| `atlas_sampler` | Sampler | Linear filtering for atlas |
| `path_intermediate_texture` | Texture | Offscreen path rasterization |
| `path_intermediate_msaa_texture` | Option<Texture> | MSAA resolve target |

---

## 3. Rendering Flow (`draw()` method)

```
1. command_encoder.start()
2. atlas.before_frame()           // Upload pending tiles
3. frame = surface.acquire_frame()
4. init_texture(frame)

5. For each batch in scene.batches():
   ├── Quads:     Upload to buffer → bind → draw(4 verts, N instances)
   ├── Shadows:   Upload to buffer → bind → draw(4 verts, N instances)
   ├── Paths:     [Special - see below]
   ├── Underlines: Upload to buffer → bind → draw(4 verts, N instances)
   ├── MonoSprites: Upload to buffer → bind texture → draw(4 verts, N instances)
   ├── PolySprites: Upload to buffer → bind texture → draw(4 verts, N instances)
   └── Surfaces:  [macOS-only video]

6. command_encoder.present(frame)
7. gpu.submit()
8. instance_belt.flush()
9. atlas.after_frame()
```

### Path Rendering (Special Case)

Paths use a two-pass approach:

1. **Rasterization pass**: Draw path triangles to intermediate texture
   - MSAA enabled (configurable via `ZED_PATH_SAMPLE_COUNT`)
   - Uses `path_rasterization` pipeline
   - Clears to TransparentBlack

2. **Copy pass**: Blit rasterized regions back to main target
   - Uses `paths` pipeline
   - Single draw per unique draw order, or union of bounds

---

## 4. Shader Analysis (`shaders.wgsl`)

### 4.1 Common Functions

| Function | Purpose |
|----------|---------|
| `to_device_position()` | Screen coords → NDC |
| `hsla_to_rgba()` | Color conversion |
| `quad_sdf()` | Signed distance for rounded rects |
| `blend_color()` | Premultiplied alpha handling |
| `gradient_color()` | Solid/gradient/pattern backgrounds |

### 4.2 Quad Shader (Most Complex)

**Vertex shader** (`vs_quad`):
- Generates 4 vertices from `vertex_id` (triangle strip)
- Passes clip distances, colors, instance ID

**Fragment shader** (`fs_quad`):
- Clip rect handling (no hardware clip_distance)
- Fast paths for unrounded/borderless quads
- SDF-based antialiasing for rounded corners
- Dashed border support
- Gradient/pattern backgrounds

### 4.3 Sprite Shaders

**MonochromeSprite** (text glyphs):
- Single-channel alpha from atlas
- Gamma correction for font rendering
- Transformation matrix support (rotation)

**PolychromeSprite** (images):
- RGBA from atlas
- Grayscale filter option
- Corner radius clipping via SDF

---

## 5. Data Structures for WebRenderer Port

### 5.1 Minimum for Quad Rendering

```rust
// GPU-side uniform
#[repr(C)]
struct GlobalParams {
    viewport_size: [f32; 2],
    premultiplied_alpha: u32,
    pad: u32,
}

// Storage buffer element (matches scene::Quad layout)
#[repr(C)]
struct Quad {
    order: u32,
    border_style: u32,
    bounds: Bounds,         // origin: [f32; 2], size: [f32; 2]
    content_mask: Bounds,
    background: Background, // Complex - tag + color data
    border_color: Hsla,     // h, s, l, a as f32
    corner_radii: Corners,  // 4x f32
    border_widths: Edges,   // 4x f32
}
```

### 5.2 For Text Rendering (MonochromeSprite)

```rust
#[repr(C)]
struct MonochromeSprite {
    order: u32,
    pad: u32,
    bounds: Bounds,
    content_mask: Bounds,
    color: Hsla,
    tile: AtlasTile,              // texture_id, tile_id, bounds
    transformation: TransformationMatrix,
}
```

---

## 6. Port Strategy for WebRenderer

### Phase 1: Basic Quads
1. Implement `GlobalParams` uniform buffer
2. Port `Quad` storage buffer layout
3. Copy `vs_quad`/`fs_quad` shaders (WGSL compatible)
4. Create quad pipeline with alpha blending

### Phase 2: Atlas Integration
1. Port `BladeAtlas` texture management
2. Add sampler for atlas
3. Enable `MonochromeSprite` for text

### Phase 3: Full Primitives
1. Add remaining pipelines (shadows, underlines, paths)
2. Implement path rasterization if needed (or simplify)

### Simplifications for WASM
- Skip `Surface` (macOS video)
- Consider skipping path MSAA initially
- May simplify gradient/dashed border support

---

## 7. Key Files Reference

| File | Lines | Content |
|------|-------|---------|
| `scene.rs` | 834 | Scene, primitives, batching |
| `blade_renderer.rs` | 1041 | BladeRenderer, pipelines, draw() |
| `shaders.wgsl` | 1300 | All WGSL shaders |
| `blade_atlas.rs` | ~400 | Texture atlas management |

---

*Generated for blade-k7ub research task*
