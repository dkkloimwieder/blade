# WebGPU Inspector Capture Analysis

**Source:** `bunny_0.html`, `bunny_5k.html`, `bunny_10k.html` (WebGPU Inspector recordings)
**Demo:** Bunnymark (blade example compiled to WASM)

---

## Capture Summary

| Metric | bunny_0.html | bunny_5k.html | bunny_10k.html |
|--------|--------------|---------------|----------------|
| Frames captured | 1001 | 1001 | 1001 |
| File size | 4.7 MB | 4.7 MB | 4.7 MB |
| Total lines | 32,320 | 32,320 | 32,320 |
| Instance count | 1,331 bunnies | 4,795 bunnies | 10,948 bunnies |

---

## Per-Frame GPU Work

Each frame executes:

| Operation | Count | 1,331 bunnies | 4,795 bunnies | 10,948 bunnies |
|-----------|-------|---------------|---------------|----------------|
| Compute pass | 1 | Physics | Physics | Physics |
| Render pass | 1 | Rendering | Rendering | Rendering |
| Dispatch | 1 | 6 workgroups | 19 workgroups | 43 workgroups |
| Draw | 1 | `draw(4, 1331)` | `draw(4, 4795)` | `draw(4, 10948)` |
| setBindGroup | 4 | 2+2 | 2+2 | 2+2 |
| setPipeline | 2 | 1+1 | 1+1 | 1+1 |
| writeBuffer | ~1 | Uniforms | Uniforms | Uniforms |

**Scaling:** Instance count scales 8.2× (1,331 → 10,948) while workgroups scale 7.2× (6 → 43). Command structure identical across all captures - only dispatch/draw parameters change. File size constant at 4.7 MB regardless of instance count.

---

## GPU Resources

### Buffers

| Label | Size | Usage |
|-------|------|-------|
| instances | 2.4 MB | Bunny position, velocity, color data |
| staging | 2.4 MB | Staging buffer for instance uploads |
| vertex | 32 B | Quad vertices (4 × vec2<f32>) |
| Uniform Buffer Ring 0 | 512 B | Transform/globals (triple buffered) |
| Uniform Buffer Ring 1 | 512 B | Transform/globals |
| Uniform Buffer Ring 2 | 512 B | Transform/globals |

### Pipelines

| Pipeline | Type | Purpose |
|----------|------|---------|
| render | Render | Instanced sprite rendering |
| physics | Compute | Bunny physics simulation |

---

## Shaders

### Compute: `cs_update`

```wgsl
@compute @workgroup_size(256, 1, 1)
fn cs_update(@builtin(global_invocation_id) id: vec3<u32>) {
    // Per-bunny physics:
    // - Apply velocity to position
    // - Apply gravity to velocity
    // - Bounce off screen bounds
}
```

- Workgroup size: 256 threads
- Dispatch: 6 workgroups = 1536 threads (covers 1331 bunnies)

### Vertex: `vs_main`

```wgsl
@vertex
fn vs_main(vertex: Vertex, @builtin(instance_index) instance_id: u32) -> VertexOutput {
    let instance = instances[instance_id];
    let offset = vertex.pos * globals.sprite_size;
    let pos = globals.mvp_transform * vec4(instance.position + offset, 0, 1);
    return VertexOutput(pos, vertex.pos, unpack_color(instance.color));
}
```

- Reads instance data from storage buffer
- Applies MVP transform
- Unpacks RGBA color from u32

### Fragment: `fs_main`

```wgsl
@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return vertex.color * textureSampleLevel(sprite_texture, sprite_sampler, vertex.tex_coords, 0);
}
```

- Samples sprite texture
- Multiplies by instance color

---

## Performance Characteristics

### Efficient Patterns

1. **Instanced rendering**: Single draw call for all sprites (scales from 1K to 11K with no additional draw calls)
2. **GPU-side physics**: Compute shader updates all instances in parallel (workgroups scale linearly)
3. **Triple-buffered uniforms**: Prevents CPU-GPU stalls
4. **Minimal state changes**: 2 pipeline binds per frame (constant regardless of instance count)

### Data Layout

```
InstanceData (24 bytes):
  position: vec2<f32>  (8 bytes)
  velocity: vec2<f32>  (8 bytes)
  color: u32           (4 bytes)
  pad: u32             (4 bytes)
```

Instance buffer usage:
- 1,331 bunnies: 32 KB
- 4,795 bunnies: 115 KB
- 10,948 bunnies: 263 KB
Allocated: 2.4 MB (room for ~100K bunnies)

---

## Comparison with Native

| Aspect | WebGPU (Chrome) | Native (Vulkan) |
|--------|-----------------|-----------------|
| API overhead | Dawn translation layer | Direct Vulkan |
| Shader compilation | WGSL → SPIR-V at runtime | Pre-compiled |
| Buffer mapping | Async required | Can be sync |
| Frame structure | Identical | Identical |

The WebGPU version uses the same efficient patterns as native blade.

---

## How to Reproduce These Captures

### 1. Build and Run the Demo

```bash
cd /path/to/blade

# Build with WebGPU backend
RUSTFLAGS="--cfg blade_wgpu" cargo run-wasm --example bunnymark

# Server starts at http://localhost:8000
```

### 2. Modify Bunny Count (Optional)

Edit `examples/bunnymark/main.rs` around line 695:

```rust
for _ in 0..11 {  // Change iteration count
    ex.increase();
}
```

| Iterations | Bunnies |
|------------|---------|
| 6 | ~1,300 |
| 9 | ~5,000 |
| 11 | ~11,000 |

### 3. Launch Chrome with WebGPU

```bash
google-chrome \
  --enable-unsafe-webgpu \
  --enable-features=Vulkan,VulkanFromANGLE \
  --use-angle=vulkan \
  --enable-dawn-features=allow_unsafe_apis \
  --user-data-dir=/tmp/chrome-webgpu-profile \
  http://localhost:8000
```

### 4. Install WebGPU Inspector

Chrome Web Store: https://chromewebstore.google.com/detail/webgpu-inspector/holcbbnljhkpkjkhgkagjkhhpeochfal

### 5. Capture

1. Open DevTools (F12)
2. Find **WebGPU Inspector** tab
3. Click **Record** to start multi-frame capture
4. Wait a few seconds
5. Click **Stop**
6. Save the HTML file

### Firefox Alternative

```bash
# Enable WebGPU in about:config first:
# dom.webgpu.enabled = true

firefox http://localhost:8000
```

WebGPU Inspector is also available for Firefox:
https://addons.mozilla.org/firefox/addon/webgpu-inspector/
