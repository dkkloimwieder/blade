# WebGPU Game of Life Example

> Compute shader ping-pong simulation pattern

This example demonstrates the **ping-pong double-buffering pattern** commonly used in GPU simulations where the output of one frame becomes the input of the next.

## What This Example Demonstrates

| Pattern | Description |
|---------|-------------|
| **Ping-pong buffers** | Two textures swapped each frame (read→write, write→read) |
| **Storage textures** | `texture_storage_2d` for compute shader read/write |
| **Compute dispatch** | Workgroup-based parallel execution |
| **Cellular automata** | Conway's Game of Life rules |
| **Separate pipelines** | Compute pipeline (simulation) + Render pipeline (display) |

## Running

### Browser (WebGPU)

```bash
RUSTFLAGS="--cfg blade_wgpu" cargo run-wasm -p blade-graphics --example webgpu-game-of-life
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
RUSTFLAGS="--cfg blade_wgpu" cargo run -p blade-graphics --example webgpu-game-of-life
```

## Expected Output

A 128x128 grid showing Conway's Game of Life:
- **Green cells**: Alive
- **Dark cells**: Dead
- Patterns evolve following standard rules (B3/S23)

## Code Structure

```
webgpu-game-of-life/
├── main.rs       # Setup, ping-pong logic, render loop
├── compute.wgsl  # Simulation rules (neighbor counting, state update)
└── render.wgsl   # Display (sample texture, colorize)
```

## Key Patterns Explained

### 1. Ping-Pong Double Buffering

```rust
// Two textures: A and B
let textures = [texture_a, texture_b];
let mut current_read: usize = 0;

// Each frame:
fn render(&mut self) {
    let read_idx = self.current_read;
    let write_idx = 1 - self.current_read;

    // Compute: read from A, write to B
    pc.bind(0, &ComputeParams {
        input_tex: self.texture_views[read_idx],
        output_tex: self.texture_views[write_idx],
    });
    pc.dispatch([groups_x, groups_y, 1]);

    // Render: display B (the newly computed state)
    rc.bind(0, &RenderParams {
        state_texture: self.render_views[write_idx],
        ...
    });

    // Swap for next frame
    self.current_read = write_idx;
}
```

### 2. Storage Texture Declaration (WGSL)

```wgsl
// Read-only storage texture (input)
var input_tex: texture_storage_2d<rgba8unorm, read>;

// Write-only storage texture (output)
var output_tex: texture_storage_2d<rgba8unorm, write>;

// Reading: textureLoad (no sampler needed)
let value = textureLoad(input_tex, coord);

// Writing: textureStore
textureStore(output_tex, coord, vec4<f32>(result));
```

### 3. Neighbor Counting with Wrapping

```wgsl
fn count_neighbors(pos: vec2<i32>, dims: vec2<i32>) -> u32 {
    var count: u32 = 0u;

    for (var dy: i32 = -1; dy <= 1; dy++) {
        for (var dx: i32 = -1; dx <= 1; dx++) {
            if (dx == 0 && dy == 0) { continue; }

            // Wrap coordinates (torus topology)
            let nx = (pos.x + dx + dims.x) % dims.x;
            let ny = (pos.y + dy + dims.y) % dims.y;

            let neighbor = textureLoad(input_tex, vec2<i32>(nx, ny));
            if (neighbor.r > 0.5) {
                count += 1u;
            }
        }
    }
    return count;
}
```

### 4. Compute Dispatch Sizing

```rust
// Grid dimensions
const GRID_WIDTH: u32 = 128;
const GRID_HEIGHT: u32 = 128;
const WORKGROUP_SIZE: u32 = 8;

// Dispatch enough workgroups to cover the grid
let groups_x = (GRID_WIDTH + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
let groups_y = (GRID_HEIGHT + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
pc.dispatch([groups_x, groups_y, 1]);
```

### 5. Conway's Game of Life Rules

```wgsl
// B3/S23: Born with 3 neighbors, Survive with 2-3 neighbors
var next_alive = false;
if (is_alive) {
    next_alive = (neighbors == 2u || neighbors == 3u);
} else {
    next_alive = (neighbors == 3u);
}
```

## Texture Format Choice

Uses `Rgba8Unorm` for WebGPU compatibility:
- Widely supported as storage texture format
- 4 bytes per cell (could optimize to 1 byte with different format)
- Stores state as float: 1.0 = alive, 0.0 = dead

## Variations

The ping-pong pattern applies to many simulations:
- **Fluid dynamics**: Velocity/pressure fields
- **Particle systems**: Position/velocity buffers
- **Image processing**: Blur, convolution filters
- **Physics**: Wave equation, heat diffusion

## Next Steps

After understanding this example, explore:
- **post-fx** - Render-to-texture for post-processing effects
- **bunnymark** - Compute physics with render instancing
