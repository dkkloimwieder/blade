# Firefox Bunnymark WGPU Trace Analysis

> Trace: `bench-trace/firefox-bunnymark-trace/`
> Captured: 2026-01-06
> Browser: Firefox 146 (stable) with `dom.webgpu.enabled`

---

## Summary

| Metric | Value |
|--------|-------|
| Total frames | 10,596 |
| Trace size | 454,519 lines |
| Buffer data files | ~10,400 .bin files |
| Shader files | 3 .wgsl files |
| Resolution | 1280x887 |
| Surface format | bgra8unorm |

---

## API Call Distribution

| Call Type | Count | Per-Frame | Notes |
|-----------|-------|-----------|-------|
| `Submit` | 10,596 | 1.0 | Frame boundaries |
| `WriteBuffer` | 10,598 | ~1.0 | SimParams updates |
| `CreateTexture` | 10,596 | 1.0 | Swapchain texture |
| `CreateTextureView` | 10,596 | 1.0 | Swapchain view |
| `FreeTexture` | 10,595 | ~1.0 | Previous frame cleanup |
| `DestroyTextureView` | 4,490 | 0.42 | Deferred cleanup |
| `DestroyTexture` | 4,490 | 0.42 | Deferred cleanup |
| `CreateBuffer` | 7 | - | Initial setup only |
| `CreateBindGroup` | 7 | - | Initial setup only |
| `CreateShaderModule` | 3 | - | Initial setup only |
| `CreateRenderPipeline` | 1 | - | Initial setup only |
| `CreateComputePipeline` | 1 | - | Initial setup only |
| `CopyBufferToBuffer` | 1 | - | Initial instance data |

---

## Resource Analysis

### Buffers (7 total, created once)

| Purpose | Size | Usage |
|---------|------|-------|
| Instance data (GPU) | 2,400,000 bytes | Storage buffer, 100K bunnies × 24 bytes |
| SimParams (triple buffered) | 512 bytes × 3 | Uniform buffer |
| Vertex buffer | Small | Quad vertices |
| Texture upload staging | Variable | Copy source |

### Shaders

1. **data1.wgsl / data2.wgsl** - Render shader (vertex + fragment)
   - Instanced rendering of sprites
   - Reads from `instances` storage buffer
   - Uses `globals` uniform for MVP transform

2. **data3.wgsl** - Compute shader
   - Physics simulation (gravity, bounce)
   - Workgroup size: 256×1×1
   - Updates position/velocity per bunny

### Bind Groups

- **Compute BindGroup**: SimParams + instance data (read-write)
- **Render BindGroup**: Globals uniform, sprite texture, sampler, instance data (read-only)

---

## Per-Frame Pattern

```
1. WriteBuffer (SimParams - 512 bytes)
2. Submit (possibly empty - render/compute passes not fully traced)
3. FreeTexture (previous swapchain)
4. CreateTexture (new swapchain, 1280×887)
5. CreateTextureView (for new swapchain)
```

**Note**: The trace shows `Submit(N, [])` with empty command arrays for most frames. This suggests:
- wgpu trace may not capture render/compute pass details by default
- Or Firefox's wgpu integration limits trace depth

---

## Performance Observations

### Good Patterns

1. **No resource churn**: Buffers and pipelines created once at startup
2. **Efficient updates**: Only SimParams (512 bytes) written per frame
3. **Instance data**: Large buffer (2.4MB) uploaded once, updated via compute shader
4. **Bind group caching**: Only 7 bind groups created total

### Potential Issues

1. **Swapchain texture recreation**: Every frame creates/destroys texture + view
   - This is normal WebGPU behavior (compositor-owned textures)
   - No optimization possible here

2. **Deferred cleanup**: ~4,490 texture destroys vs ~10,596 creates
   - Suggests Firefox batches some cleanup
   - Not a performance issue

---

## Instance Data Structure

```wgsl
struct InstanceData {
    position: vec2<f32>,  // 8 bytes
    velocity: vec2<f32>,  // 8 bytes
    color: u32,           // 4 bytes
    pad: u32,             // 4 bytes (alignment)
}                         // Total: 24 bytes
```

With 100,000 instances: 24 × 100,000 = 2,400,000 bytes

---

## Trace Limitations

The Firefox WGPU trace does NOT capture:
- Individual draw/dispatch calls within passes
- Render pass configuration details
- GPU timing information
- Detailed command buffer contents

For deeper analysis, use:
- **WebGPU Inspector** browser extension (captures full frame)
- **RenderDoc** with wgpu player replay

---

## Replay Instructions

```bash
# Clone wgpu if not already
git clone https://github.com/gfx-rs/wgpu
cd wgpu/player

# Replay trace (headless for RenderDoc)
cargo run --release -- ~/dev/gfxx/blade/bench-trace/firefox-bunnymark-trace

# Replay with window
cargo run --release --features winit -- ~/dev/gfxx/blade/bench-trace/firefox-bunnymark-trace
```

---

## Conclusions

The bunnymark trace shows a **well-optimized rendering pattern**:

1. Resources created once at startup
2. Per-frame overhead is minimal (one small buffer write)
3. Compute shader handles physics simulation on GPU
4. No unnecessary resource recreation

The main limitation is that WGPU_TRACE doesn't capture full render/compute pass details in Firefox. For comprehensive frame analysis, use WebGPU Inspector extension or replay through RenderDoc.

---

## How to Capture Firefox WGPU Traces

### 1. One-Time Setup

Create a dedicated Firefox profile:

```bash
mkdir -p ~/firefox-wgputrace-profile
firefox -ProfileManager --no-remote
# Create profile named "wgputrace" pointing to above directory
# Launch profile, enable dom.webgpu.enabled in about:config
```

### 2. Start the Demo

```bash
cd /path/to/blade
RUSTFLAGS="--cfg blade_wgpu" cargo run-wasm --example bunnymark
# Server at http://localhost:8000
```

### 3. Capture Trace

```bash
mkdir -p ~/wgpu-trace

MOZ_DISABLE_GPU_SANDBOX=1 WGPU_TRACE=~/wgpu-trace \
  firefox --profile ~/firefox-wgputrace-profile --no-remote \
  http://localhost:8000
```

Let the demo run, then close Firefox. Trace is in `~/wgpu-trace/0/`.

### 4. Analyze

```bash
# Count frames
grep -c "Submit" ~/wgpu-trace/0/trace.ron

# Count API calls by type
grep -oE "^    [A-Za-z]+" ~/wgpu-trace/0/trace.ron | sort | uniq -c | sort -rn
```

### Alternative: WebGPU Inspector

For interactive frame capture without WGPU_TRACE:

1. Install from https://addons.mozilla.org/firefox/addon/webgpu-inspector/
2. Open DevTools (F12)
3. Find WebGPU Inspector tab
4. Click **Record** for multi-frame capture
