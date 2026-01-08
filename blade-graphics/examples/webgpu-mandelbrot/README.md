# WebGPU Mandelbrot Fractal Example

> Compute shader fractal visualization

This example demonstrates **compute shaders** for generating fractal images, a common pattern for GPU-accelerated visualization and simulation.

## What This Example Demonstrates

| Pattern | Description |
|---------|-------------|
| **Compute shaders** | GPU parallel computation with `@compute @workgroup_size(8, 8)` |
| **Storage texture output** | Writing to `texture_storage_2d<rgba8unorm, write>` |
| **Uniform buffers** | Passing parameters (center, zoom, iterations) |
| **Compute → Render pipeline** | Generate in compute, display in render pass |
| **Escape-time algorithm** | Classic Mandelbrot iteration `z = z² + c` |

## Running

### Browser (WebGPU)

```bash
RUSTFLAGS="--cfg blade_wgpu" cargo run-wasm -p blade-graphics --example webgpu-mandelbrot
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
RUSTFLAGS="--cfg blade_wgpu" cargo run -p blade-graphics --example webgpu-mandelbrot
```

## Expected Output

A colorful Mandelbrot fractal that automatically zooms into an interesting region over time. Colors are generated using HSV color space for smooth gradients.

## Code Structure

```
webgpu-mandelbrot/
├── main.rs       # Setup, animation loop, compute dispatch
├── compute.wgsl  # Mandelbrot iteration and coloring
└── render.wgsl   # Fullscreen quad display
```

## Key Patterns Explained

### 1. Compute Shader Structure

```wgsl
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let coord = vec2<i32>(global_id.xy);

    // Bounds check (workgroups may exceed texture size)
    if (coord.x >= i32(dims.x) || coord.y >= i32(dims.y)) {
        return;
    }

    // ... compute pixel value ...

    textureStore(output_tex, coord, vec4<f32>(color, 1.0));
}
```

### 2. Storage Texture Declaration

```wgsl
// Write-only storage texture
var output_tex: texture_storage_2d<rgba8unorm, write>;

// Writing (no sampler needed)
textureStore(output_tex, coord, value);
```

### 3. Compute Dispatch Sizing

```rust
const WORKGROUP_SIZE: u32 = 8;

// Dispatch enough workgroups to cover the texture
let groups_x = (width + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
let groups_y = (height + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
pc.dispatch([groups_x, groups_y, 1]);
```

### 4. Mandelbrot Algorithm

```wgsl
// z = z² + c, iterate until escape or max iterations
var z = vec2<f32>(0.0, 0.0);
for (var i: u32 = 0u; i < max_iterations; i++) {
    // z² = (a+bi)² = (a²-b²) + (2ab)i
    z = vec2<f32>(
        z.x * z.x - z.y * z.y + c.x,
        2.0 * z.x * z.y + c.y
    );

    // Escape condition: |z| > 2
    if (dot(z, z) > 4.0) {
        break;
    }
    iteration++;
}
```

### 5. HSV Color Mapping

```wgsl
// Map iteration count to hue for smooth color cycling
let t = f32(iteration) / f32(max_iterations);
let hue = t * 3.0 % 1.0;
let color = hsv_to_rgb(hue, 0.8, 1.0 - t * 0.3);
```

## Fractal Parameters

| Parameter | Description |
|-----------|-------------|
| `center_x`, `center_y` | Point in complex plane to center view |
| `zoom` | Zoom factor (higher = more zoomed in) |
| `max_iterations` | More iterations = more detail but slower |

The example animates zoom toward the point (-0.7436, 0.1318), a visually interesting region with infinite detail.

## Performance Notes

- **Workgroup size 8x8** balances occupancy and memory access patterns
- **256 max iterations** provides good detail without excessive computation
- Higher zoom levels may show precision limits (f32 has ~7 significant digits)
- For deeper zooms, consider using f64 or perturbation theory

## Variations

The compute shader pattern applies to many visualizations:
- **Julia sets** - Similar iteration, different starting point
- **Newton fractals** - Root-finding visualization
- **Fluid simulation** - Velocity/pressure field updates
- **Particle systems** - Position/velocity integration
- **Image processing** - Filters, convolutions, transformations

## Next Steps

After understanding this example, explore:
- **game-of-life** - Ping-pong compute simulation
- **post-fx** - Render-to-texture post-processing
