# WebGPU Post-Processing Effects Example

> Multi-pass rendering with render-to-texture

This example demonstrates **render-to-texture** and **post-processing pipelines**, a fundamental pattern for adding visual effects to 3D scenes.

## What This Example Demonstrates

| Pattern | Description |
|---------|-------------|
| **Render-to-texture** | Rendering to an offscreen texture instead of the screen |
| **Multi-pass rendering** | Scene pass → Post-FX pass → Screen |
| **Uniform buffers** | Time-based animation via `var<uniform>` |
| **Texture sampling** | Reading the offscreen texture in a fragment shader |
| **Post-processing effects** | Vignette and chromatic aberration |

## Running

### Browser (WebGPU)

```bash
RUSTFLAGS="--cfg blade_wgpu" cargo run-wasm -p blade-graphics --example webgpu-post-fx
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
RUSTFLAGS="--cfg blade_wgpu" cargo run -p blade-graphics --example webgpu-post-fx
```

## Expected Output

A spinning triangle with animated colors, rendered with:
- **Vignette effect** (darkened corners/edges)
- **Chromatic aberration** (subtle RGB channel offset that pulses over time)

## Code Structure

```
webgpu-post-fx/
├── main.rs       # Setup, multi-pass rendering, resize handling
├── scene.wgsl    # Scene rendering (spinning triangle with animated colors)
└── postfx.wgsl   # Post-processing effects (vignette, chromatic aberration)
```

## Key Patterns Explained

### 1. Offscreen Render Target Setup

```rust
// Create a texture that can be both rendered to AND sampled
let offscreen_texture = context.create_texture(gpu::TextureDesc {
    name: "offscreen",
    format: gpu::TextureFormat::Rgba8Unorm,
    size: extent,
    // TARGET: can be rendered to
    // RESOURCE: can be sampled in shaders
    usage: gpu::TextureUsage::TARGET | gpu::TextureUsage::RESOURCE,
    ..
});

let offscreen_view = context.create_texture_view(offscreen_texture, ..);
```

### 2. Multi-Pass Rendering

```rust
fn render(&mut self) {
    // Pass 1: Render scene to offscreen texture
    if let mut pass = encoder.render("scene", gpu::RenderTargetSet {
        colors: &[gpu::RenderTarget {
            view: self.offscreen_view,  // Render TO the offscreen texture
            init_op: gpu::InitOp::Clear(..),
            finish_op: gpu::FinishOp::Store,
        }],
        ..
    }) {
        // Draw scene geometry
    }

    // Pass 2: Apply post-fx and render to screen
    if let mut pass = encoder.render("postfx", gpu::RenderTargetSet {
        colors: &[gpu::RenderTarget {
            view: frame.texture_view(),  // Render to screen
            ..
        }],
        ..
    }) {
        // Bind offscreen texture as input, draw fullscreen quad
        rc.bind(0, &PostFxParams {
            scene_texture: self.offscreen_view,  // Sample FROM offscreen
            scene_sampler: self.sampler,
            ..
        });
        rc.draw(0, 6, 0, 1);  // Fullscreen quad (6 vertices)
    }
}
```

### 3. Uniform Buffer Declaration (WGSL)

```wgsl
// Struct must match Rust layout exactly (16-byte aligned for uniforms)
struct Uniforms {
    time: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

// Use var<uniform> for uniform buffer binding
var<uniform> uniforms: Uniforms;
```

### 4. Post-Processing Effects (WGSL)

```wgsl
// Vignette: darken edges based on distance from center
fn vignette(uv: vec2<f32>, strength: f32) -> f32 {
    let center = uv - 0.5;
    let dist = length(center);
    return 1.0 - smoothstep(0.3, 0.7, dist * strength);
}

// Chromatic aberration: offset RGB channels differently
fn chromatic_aberration(uv: vec2<f32>, amount: f32) -> vec3<f32> {
    let center = uv - 0.5;
    let offset = center * amount;

    let r = textureSample(scene_texture, scene_sampler, uv + offset).r;
    let g = textureSample(scene_texture, scene_sampler, uv).g;
    let b = textureSample(scene_texture, scene_sampler, uv - offset).b;

    return vec3<f32>(r, g, b);
}
```

### 5. Handling Window Resize

When the window resizes, the offscreen texture must be recreated:

```rust
fn resize(&mut self, size: PhysicalSize<u32>) {
    // Reconfigure surface
    self.context.reconfigure_surface(&mut self.surface, ..);

    // Destroy old offscreen resources
    self.context.destroy_texture_view(self.offscreen_view);
    self.context.destroy_texture(self.offscreen_texture);

    // Recreate at new size
    self.offscreen_texture = self.context.create_texture(..);
    self.offscreen_view = self.context.create_texture_view(..);
}
```

## Common Post-Processing Effects

This pattern enables many effects:

| Effect | Technique |
|--------|-----------|
| **Bloom** | Threshold bright pixels, blur, blend back |
| **Motion blur** | Accumulate previous frames |
| **Depth of field** | Blur based on depth buffer |
| **Color grading** | LUT (lookup table) sampling |
| **FXAA/SMAA** | Edge detection and blending |
| **Screen-space reflections** | Ray march in screen space |

## Next Steps

After understanding this example, explore:
- **game-of-life** - Ping-pong compute shaders
- **bunnymark** - Instanced rendering with compute physics
