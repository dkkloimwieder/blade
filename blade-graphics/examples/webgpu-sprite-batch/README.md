# WebGPU Sprite Batch Example

> Efficient 2D sprite rendering using instancing

This example demonstrates **instanced rendering** for 2D games, a fundamental pattern for rendering many sprites efficiently with a single draw call.

## What This Example Demonstrates

| Pattern | Description |
|---------|-------------|
| **Instanced rendering** | One draw call for 100 sprites |
| **Storage buffer** | Per-sprite data (position, size, rotation, color) |
| **Vertex buffer** | Shared quad geometry |
| **Screen-space coords** | 2D coordinate system with (0,0) at top-left |
| **Alpha blending** | Semi-transparent sprites |

## Running

### Browser (WebGPU)

```bash
RUSTFLAGS="--cfg blade_wgpu" cargo run-wasm -p blade-graphics --example webgpu-sprite-batch
```

Open http://localhost:8000 in a WebGPU-capable browser.

**Chrome on Linux** requires flags:
```bash
google-chrome \
  --user-data-dir=/path/to/repo/.chrome-profile \
  --enable-unsafe-webgpu \
  --enable-features=Vulkan,VulkanFromANGLE \
  --use-angle=vulkan \
  --enable-dawn-features=allow_unsafe_apis \
  http://localhost:8000
```

### Native

```bash
RUSTFLAGS="--cfg blade_wgpu" cargo run -p blade-graphics --example webgpu-sprite-batch
```

## Expected Output

A colorful animated spiral of 100 rotating squares, each with:
- Rainbow colors cycling over time
- Individual rotation
- Varying sizes (smaller in center, larger at edges)
- Semi-transparent blending

## Code Structure

```
webgpu-sprite-batch/
├── main.rs     # Setup, sprite generation, instanced draw
└── shader.wgsl # Vertex transform, color unpacking
```

## Key Patterns Explained

### 1. Sprite Data Structure

```rust
#[repr(C)]
struct SpriteData {
    position: [f32; 2],  // Screen position
    size: [f32; 2],      // Width, height
    rotation: f32,       // Radians
    color: u32,          // Packed RGBA
    _pad: [f32; 2],      // Alignment padding
}
```

### 2. Storage Buffer for Sprites

```rust
// Create buffer
let sprite_buffer = context.create_buffer(gpu::BufferDesc {
    name: "sprites",
    size: (mem::size_of::<SpriteData>() * NUM_SPRITES) as u64,
    memory: gpu::Memory::Shared,
});

// Upload each frame
unsafe {
    ptr::copy_nonoverlapping(
        sprites.as_ptr(),
        sprite_buffer.data() as *mut SpriteData,
        sprites.len(),
    );
}
context.sync_buffer(sprite_buffer);
```

### 3. WGSL Storage Buffer Access

```wgsl
struct SpriteData {
    position: vec2<f32>,
    size: vec2<f32>,
    rotation: f32,
    color: u32,
    _pad: vec2<f32>,
}

var<storage, read> sprites: array<SpriteData>;

@vertex
fn vs_main(vertex: Vertex, @builtin(instance_index) instance_id: u32) -> VertexOutput {
    let sprite = sprites[instance_id];
    // Transform vertex using sprite data...
}
```

### 4. Instanced Draw Call

```rust
// Draw 6 vertices (quad), NUM_SPRITES instances
rc.draw(0, 6, 0, NUM_SPRITES as u32);
```

### 5. Screen-Space Coordinate Transform

```wgsl
// Convert world position to clip space
// Screen: (0,0) top-left, (width, height) bottom-right
// Clip: (-1,-1) bottom-left, (1,1) top-right
let clip_pos = vec2<f32>(
    (world_pos.x / globals.screen_size.x) * 2.0 - 1.0,
    1.0 - (world_pos.y / globals.screen_size.y) * 2.0
);
```

### 6. Color Packing/Unpacking

```rust
// Pack RGBA into u32 (Rust)
let color = (a as u32) << 24 | (b as u32) << 16 | (g as u32) << 8 | (r as u32);
```

```wgsl
// Unpack u32 to vec4 (WGSL)
fn unpack_color(packed: u32) -> vec4<f32> {
    return vec4<f32>(
        f32((packed >> 0u) & 0xFFu) / 255.0,
        f32((packed >> 8u) & 0xFFu) / 255.0,
        f32((packed >> 16u) & 0xFFu) / 255.0,
        f32((packed >> 24u) & 0xFFu) / 255.0
    );
}
```

## Performance Notes

- **Single draw call** for all sprites (vs one per sprite)
- **Storage buffer** allows GPU-side random access
- **Shared vertex buffer** - quad geometry uploaded once
- **Memory::Shared** for CPU→GPU updates each frame
- For static sprites, use `Memory::Device` and upload once

## Scaling Up

This pattern scales to thousands of sprites:

| Sprites | Pattern |
|---------|---------|
| 100 | This example (CPU updates each frame) |
| 1,000+ | Consider compute shader for physics |
| 10,000+ | Use GPU-only buffers, indirect draw |

See the **bunnymark** example for compute-driven sprite physics.

## Variations

The instanced sprite pattern applies to:
- **Particle systems** - Explosions, smoke, sparks
- **Tile maps** - 2D level rendering
- **UI elements** - Buttons, icons, text glyphs
- **Bullet hell** - Thousands of projectiles

## Next Steps

After understanding this example, explore:
- **bunnymark** - Compute shader physics for sprites
- **post-fx** - Post-processing effects
- **mandelbrot** - Compute shader visualization
