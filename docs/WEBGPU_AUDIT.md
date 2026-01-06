# WebGPU Backend Feature Audit (Browser/WASM)

> Comprehensive analysis of blade-graphics WebGPU backend for browser deployment
> Audit Date: 2026-01-06

---

## Executive Summary

The WebGPU backend is **functionally complete for browser graphics operations**. It provides a solid implementation for compute shaders, render passes, and standard graphics features within the constraints of the browser WebGPU specification.

| Category | Status | Notes |
|----------|--------|-------|
| Core Graphics | ✅ Complete | Render/compute pipelines, resources |
| Compute Shaders | ✅ Complete | Full dispatch support |
| Transfer Operations | ✅ Complete | Buffer/texture copies |
| GPU Timing | ⚠️ Infrastructure Only | WASM async readback not implemented |
| Bindless Resources | ❌ N/A | Not in browser WebGPU spec |
| Ray Tracing | ❌ N/A | Not in browser WebGPU spec |

**Total Lines**: ~4,278

---

## 1. Implemented Features (Browser WebGPU)

### 1.1 Resource Management

| Feature | Status | Notes |
|---------|--------|-------|
| Buffer Creation | ✅ | Device/Upload/Shared memory with shadow buffers |
| Buffer Sync | ✅ | Dirty tracking, auto sync before submit |
| Texture Creation | ✅ | All dimensions, mip levels, MSAA |
| Texture Views | ✅ | Full subresource specification |
| Samplers | ✅ | All filter/address modes, comparison, anisotropy |
| Resource Destruction | ✅ | Proper bind group cache invalidation |

**Shadow Buffer Model**: For `Memory::Upload` and `Memory::Shared`, maintains CPU-side shadow memory and syncs via `queue.write_buffer()`. This avoids WebGPU's complex async buffer mapping.

### 1.2 Pipeline System

| Feature | Status | Notes |
|---------|--------|-------|
| Compute Pipelines | ✅ | Full workgroup support |
| Render Pipelines | ✅ | All primitive/blend/depth-stencil states |
| Shader Compilation | ✅ | WGSL → Naga → WGSL with @group/@binding |
| Vertex Fetch | ✅ | Automatic location assignment |
| Bind Group Layouts | ✅ | Auto-generated from ShaderDataLayout |

### 1.3 Command Encoding

| Feature | Status | Notes |
|---------|--------|-------|
| Transfer Pass | ✅ | fill, copy buffer/texture |
| Compute Pass | ✅ | dispatch, dispatch_indirect |
| Render Pass | ✅ | Full render target set, MSAA resolve |
| Draw Commands | ✅ | draw, draw_indexed, indirect variants |
| Scissor/Viewport | ✅ | Full specification |
| Stencil Reference | ✅ | Dynamic state |

### 1.4 Bind Group Caching

| Feature | Status | Notes |
|---------|--------|-------|
| LRU Cache | ✅ | 1024 entries, reduces bind group recreation |
| Cache Invalidation | ✅ | On resource destruction |
| Dependency Tracking | ✅ | Tracks which cache entries use which resources |

### 1.5 Surface/Presentation (Browser)

| Feature | Status | Notes |
|---------|--------|-------|
| Canvas Discovery | ✅ | Auto-finds `id="blade"` |
| `create_surface_from_canvas()` | ✅ | Explicit canvas API |
| Frame Acquisition | ✅ | Graceful Timeout/Outdated/Lost handling |
| Surface Reconfiguration | ✅ | Size, present mode |
| Firefox Compatibility | ✅ | Uses adapter-reported format (rgba8unorm) |

### 1.6 Initialization (WASM)

```rust
// Required async initialization for browser
let context = blade_graphics::Context::init_async(desc).await?;
```

- Uses `wgpu::Backends::BROWSER_WEBGPU`
- Async adapter/device request
- Device lost callback registered

---

## 2. Partially Implemented Features

### 2.1 GPU Timing Queries

**Status**: ⚠️ Infrastructure exists, readback not implemented

| Component | Status | Location |
|-----------|--------|----------|
| Feature Detection | ✅ | `TIMESTAMP_QUERY` checked |
| Query Set Creation | ✅ | Triple-buffered ring |
| Timestamp Recording | ✅ | Begin/end per pass |
| Resolve to Buffer | ✅ | `resolve_query_set()` called |
| Async Readback | ❌ | `advance_frame()` skips readback |
| Results API | ⚠️ | Returns empty `&[]` on WASM |

**Current WASM Behavior** (`mod.rs:268-291`):
```rust
#[cfg(target_arch = "wasm32")]
pub fn advance_frame(&mut self) {
    // On WASM, GPU timing queries require special browser flags and async
    // buffer mapping is complex. Skip readback for now.
    // Users should use browser DevTools Performance tab for GPU profiling.
    readback_frame.reset();
}
```

**Root Cause**: Browser WebGPU requires:
1. `--enable-dawn-features=allow_unsafe_apis` in Chrome
2. Async buffer mapping with `map_async()` + future spawning
3. Quantization (100μs resolution in non-isolated contexts)

**Open Issue**: `blade-9rv` - WebGPU: Implement WASM GPU timing readback

---

## 3. Features Not Available in Browser WebGPU

These are **not bugs** - they're WebGPU spec limitations:

### 3.1 Ray Tracing / Acceleration Structures

| API | Behavior |
|-----|----------|
| `create_acceleration_structure()` | `panic!()` with helpful message |
| `acceleration_structure()` encoder | `panic!()` |
| `capabilities().ray_query` | Returns `empty()` |

Correct behavior - ray tracing is not in WebGPU spec.

### 3.2 Bindless / Resource Arrays

| API | Behavior |
|-----|----------|
| `TextureArray::bind_to()` | `unimplemented!()` with message |
| `BufferArray::bind_to()` | `unimplemented!()` with message |

Correct behavior - binding arrays not in browser WebGPU. Use individual bindings.

### 3.3 Dual-Source Blending

| API | Behavior |
|-----|----------|
| `BlendFactor::Src1` variants | `panic!()` with message |
| `capabilities().dual_source_blending` | Returns `false` |

Correct behavior - not in base WebGPU spec.

### 3.4 External Memory

| API | Behavior |
|-----|----------|
| `Memory::External(_)` | `panic!()` with message |

Correct behavior - cross-process memory sharing not in WebGPU.

---

## 4. Capabilities Reporting

```rust
pub fn capabilities(&self) -> crate::Capabilities {
    crate::Capabilities {
        ray_query: crate::ShaderVisibility::empty(),
        sample_count_mask: 0b0101, // 1 and 4 samples
        dual_source_blending: false,
    }
}
```

| Capability | Value | Correct? |
|------------|-------|----------|
| `ray_query` | Empty | ✅ |
| `sample_count_mask` | 1, 4 | ✅ Standard WebGPU |
| `dual_source_blending` | false | ✅ |

---

## 5. Browser & Platform Compatibility

### 5.1 Chrome on Linux

**Status**: Experimental - requires flags

**Required Launch Flags**:
```bash
chrome --enable-unsafe-webgpu --enable-features=Vulkan,VulkanFromANGLE
```

Or enable in `chrome://flags`:
- `#enable-unsafe-webgpu`
- `#enable-vulkan`

**Known Issues**:
- ⚠️ **Wayland not compatible with Vulkan** - use `--ozone-platform=x11`
- Intel Gen12+ coming in Chrome 144 (Stable 2025-01-13)
- Other GPUs require manual flag enabling
- NVIDIA + Wayland requires sandbox disabling (`--no-sandbox`)

**Performance**: Reports of slow performance on Linux compared to Windows/macOS due to experimental Vulkan backend.

### 5.2 Firefox on Linux

**Status**: Available in Nightly, not yet in Stable (expected 2026)

**Enable**: Set `dom.webgpu.enabled` = true in `about:config` (requires restart)

**Known Issues** (from [Mozilla Bugzilla](https://bugzilla.mozilla.org)):
- **Wayland canvas blank** ([bug 1966566](https://bugzilla.mozilla.org/show_bug.cgi?id=1966566)) - try `dom.webgpu.allow-present-without-readback` = false
- **Driver crashes** - Update Mesa drivers; NVIDIA still has issues
- **Random crashes** - Often driver-related, updating Mesa/NVIDIA drivers helps
- 3 FTE vs Chrome's ~30 WebGPU developers

**Why Firefox may perform better**: Both blade and Firefox use **wgpu** - same underlying Rust library. Chrome uses Dawn (C++). wgpu may have better Vulkan codepaths on Linux.

### 5.3 Surface Format Detection

**Firefox Quirk** (`surface.rs:132-138`):
```rust
let blade_format = match format {
    wgpu::TextureFormat::Rgba8Unorm => crate::TextureFormat::Rgba8Unorm,
    // Firefox WebGPU only supports rgba8unorm, not bgra8unorm-srgb
    ...
};
```

✅ Correctly uses adapter-reported format rather than assuming Bgra8UnormSrgb.

### 5.4 Canvas Element Discovery

```rust
// Auto-discovers canvas with id="blade"
let canvas = web_sys::window()
    .and_then(|w| w.document())
    .and_then(|d| d.get_element_by_id(crate::CANVAS_ID))
    ...
```

Also provides `create_surface_from_canvas(canvas)` for explicit control.

### 5.5 Error Handling

- Device lost callback registered for graceful degradation
- Surface errors (Timeout, Outdated, Lost) return invalid frame rather than panic
- OutOfMemory is the only surface error that panics (unrecoverable)

### 5.6 Platform Recommendation

| Platform | Browser | Status |
|----------|---------|--------|
| Linux | Firefox Nightly | Most stable for wgpu-based apps |
| Linux | Chrome | Experimental, requires flags |
| Windows | Chrome/Edge | Best supported |
| macOS | Chrome/Safari | Well supported |

---

## 6. Shader Validation (Browser Context)

`shader.rs:35-46`:
```rust
#[cfg(blade_wgpu)]
caps.set(
    naga::valid::Capabilities::RAY_QUERY,
    !device_caps.ray_query.is_empty(),
);
// Note: NON_UNIFORM_INDEXING capabilities NOT enabled for WebGPU
```

✅ Correctly disables non-uniform indexing validation for WebGPU (not supported).

---

## 7. Build & Warnings

```bash
RUSTFLAGS="--cfg blade_wgpu" cargo check -p blade-graphics
```

**Status**: ✅ Compiles

**Warnings**:
| Warning | Severity | Notes |
|---------|----------|-------|
| `source` field never read | Low | Shader debug info |
| `uses_dual_source` never used | Low | Dead code - feature not available |

These warnings are acceptable for browser-only deployment.

---

## 8. Browser Profiling Options

The current backend provides **infrastructure for timestamp queries** but lacks readback. Here are all available profiling approaches:

### 8.1 Chrome DevTools Performance Tab

**What it shows**: JavaScript execution time, frame boundaries, call stacks
**Limitation**: CPU-only, no GPU timing, sampling-based (may miss short calls)

### 8.2 Chrome Perfetto Tracing (Recommended)

**Access**: `chrome://tracing` → "Record a new Trace" or use Perfetto Chrome extension

**Key Category**: Enable `gpu.dawn` to capture:
- Dawn implementation internals
- GPU process communication
- Texture uploads, buffer operations
- Frame timing from browser's perspective

**Navigation**: W/S to zoom, A/D to pan the flame graph

**Tip**: Use Chrome Canary with minimal tabs to reduce trace noise.

### 8.3 WebGPU Inspector Extension

**Install**: [Chrome Web Store](https://chromewebstore.google.com/detail/webgpu-inspector/holcbbnljhkpkjkhgkagjkhhpeochfal)

**Features**:
- Live GPU object inspection
- Frame capture with render pass outputs
- Buffer/texture data inspection
- **Shader live editing**
- Frame time plotting
- Object allocation tracking

This is the **best tool for debugging rendering issues** without code changes.

### 8.4 Timestamp Queries (In-App)

**Current Status**: Infrastructure only, no readback on WASM

**What's needed** (from [WebGPU Fundamentals](https://webgpufundamentals.org/webgpu/lessons/webgpu-timing.html)):

```javascript
// 1. Request feature
const device = await adapter.requestDevice({
  requiredFeatures: ['timestamp-query']
});

// 2. Create query set + buffers
const querySet = device.createQuerySet({ type: 'timestamp', count: 2 });
const resolveBuffer = device.createBuffer({ size: 16, usage: QUERY_RESOLVE | COPY_SRC });
const resultBuffer = device.createBuffer({ size: 16, usage: COPY_DST | MAP_READ });

// 3. Add to pass
const pass = encoder.beginRenderPass({
  ...descriptor,
  timestampWrites: {
    querySet,
    beginningOfPassWriteIndex: 0,
    endOfPassWriteIndex: 1,
  }
});

// 4. Resolve & copy after pass.end()
encoder.resolveQuerySet(querySet, 0, 2, resolveBuffer, 0);
encoder.copyBufferToBuffer(resolveBuffer, 0, resultBuffer, 0, 16);

// 5. Async readback
await resultBuffer.mapAsync(GPUMapMode.READ);
const times = new BigUint64Array(resultBuffer.getMappedRange());
const gpuTimeNs = Number(times[1] - times[0]);
resultBuffer.unmap();
```

**Precision**: Quantized to 100μs by default (security). Enable `enable-webgpu-developer-features` flag for higher precision.

**WASM Challenge**: `mapAsync` requires async handling that wgpu/Rust on WASM doesn't trivially support.

### 8.5 Profiling Strategy Recommendation

| Goal | Best Tool |
|------|-----------|
| Find JavaScript bottlenecks | DevTools Performance |
| Understand GPU command flow | Perfetto `gpu.dawn` |
| Debug render pass issues | WebGPU Inspector |
| Measure shader performance | Timestamp queries (once implemented) |
| Compare technique performance | Timestamp queries |

---

## 9. Identified Issues

### 9.1 Critical: None

### 9.2 High Priority

1. **WASM GPU Timing Readback** (existing issue: `blade-9rv`)
   - Infrastructure complete: query sets, resolve, copy all work
   - Missing: async buffer mapping for readback on WASM
   - **Workaround**: Use Perfetto `gpu.dawn` tracing or WebGPU Inspector
   - **Impact**: Cannot get per-pass GPU timing from within the app

### 9.3 Low Priority

2. **Sample Count Query**
   - Hardcoded to 1, 4 - could query adapter
   - Browser WebGPU typically only supports these anyway

---

## 10. WebGPU Spec Compliance

Based on [W3C WebGPU Spec](https://www.w3.org/TR/webgpu/) (Dec 2025):

| Spec Feature | Status |
|--------------|--------|
| GPUAdapter | ✅ |
| GPUDevice | ✅ |
| GPUBuffer | ✅ |
| GPUTexture | ✅ |
| GPUSampler | ✅ |
| GPUBindGroup | ✅ |
| GPUComputePipeline | ✅ |
| GPURenderPipeline | ✅ |
| GPUCommandEncoder | ✅ |
| GPURenderPassEncoder | ✅ |
| GPUComputePassEncoder | ✅ |
| Timestamp Queries (optional) | ⚠️ Recording only |
| Error Scopes | ⚠️ Disabled on WASM (ordering issues) |

---

## 11. Test Status

| Test | Status | Notes |
|------|--------|-------|
| WASM Bunnymark | ✅ | Render + compute |
| WASM Mini | ✅ | Compute only |
| WASM Frustum-Cull | ⚠️ | Visual artifacts (separate issue `blade-b8e`) |
| Surface Resize | ✅ | Reconfigure works |
| Firefox | ✅ | Format detection works |
| Chrome | ✅ | Primary target |

---

## 12. Conclusion

The WebGPU backend is **ready for browser deployment** for standard graphics workloads.

**Strengths**:
- Complete core graphics API coverage
- Proper unsupported feature handling with clear messages
- Firefox compatibility
- Effective bind group caching
- Shadow buffer model avoids async mapping complexity

**Known Gaps**:
- GPU timing readback (infrastructure exists, readback not implemented)

**Recommendation**:
- Close audit issue
- `blade-9rv` remains open for timing readback
- `blade-b8e` is a separate visual bug, not a backend issue

---

## References

- [W3C WebGPU Specification](https://www.w3.org/TR/webgpu/)
- [WebGPU Timing Performance](https://webgpufundamentals.org/webgpu/lessons/webgpu-timing.html)
- [GPUQuerySet MDN](https://developer.mozilla.org/en-US/docs/Web/API/GPUQuerySet)
- [Chrome WebGPU Timestamp Queries](https://developer.chrome.com/blog/new-in-webgpu-120)

---

*Audit completed for blade-graphics v0.7.0 WebGPU backend (browser target)*
