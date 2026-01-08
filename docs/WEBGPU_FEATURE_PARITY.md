# WebGPU vs Vulkan Feature Parity Matrix

> Comprehensive comparison of blade-graphics backends for feature planning and portability decisions

---

## Quick Reference

| Feature | Vulkan | WebGPU | Gap Severity |
|---------|:------:|:------:|:------------:|
| Render Pipelines | Full | Full | None |
| Compute Pipelines | Full | Full | None |
| Ray Tracing | Full | None | Critical |
| Acceleration Structures | Full | None | Critical |
| Dual-Source Blending | Yes | Extension | Minor |
| Bindless/Arrays | Full | Limited | Moderate |
| GPU Timing | Full | Partial | Minor |
| External Memory | Yes | No | Moderate |

---

## 1. Pipeline Support

### 1.1 Render Pipelines

| Capability | Vulkan | WebGPU | Notes |
|------------|:------:|:------:|-------|
| Basic rendering | Yes | Yes | Full parity |
| Vertex shaders | Yes | Yes | |
| Fragment shaders | Yes | Yes | |
| Primitive types | All | All | Points, lines, triangles |
| Depth testing | Yes | Yes | |
| Stencil testing | Yes | Yes | |
| Blending | Full | Most | See dual-source below |
| MSAA (1, 4 samples) | Yes | Yes | |
| MSAA resolve | Yes | Yes | Via `FinishOp::ResolveTo` |
| Multiple render targets | Yes | Yes | |
| Depth bias | Yes | Yes | |
| Scissor/viewport | Yes | Yes | Dynamic state |

### 1.2 Compute Pipelines

| Capability | Vulkan | WebGPU | Notes |
|------------|:------:|:------:|-------|
| Compute shaders | Yes | Yes | Full parity |
| Workgroup dispatch | Yes | Yes | |
| Indirect dispatch | Yes | Yes | |
| Shared memory | Yes | Yes | `var<workgroup>` in WGSL |
| Subgroups | Yes | Limited | WebGPU subgroups proposal in progress |

### 1.3 Ray Tracing Pipelines

| Capability | Vulkan | WebGPU | Notes |
|------------|:------:|:------:|-------|
| Ray tracing pipelines | Yes | **No** | Not in WebGPU spec |
| Ray queries | Yes | **No** | Proposal exists, not standardized |
| Acceleration structures | Yes | **No** | |
| BLAS/TLAS building | Yes | **No** | |
| Shader binding tables | Yes | **No** | |

**WebGPU Status**: No official ray tracing support. Community projects like [WebRTX](https://github.com/codedhead/webrtx) provide compute-shader-based ray tracing, but hardware acceleration is not available.

---

## 2. Shader Features

### 2.1 Shader Language

| Aspect | Vulkan | WebGPU | Notes |
|--------|:------:|:------:|-------|
| Source format | SPIR-V | WGSL | Both via Naga |
| Shader compilation | Offline/runtime | Runtime | WebGPU compiles WGSL at load |
| Specialization constants | Yes | Yes | Pipeline-overridable constants |

### 2.2 Shader Capabilities

| Feature | Vulkan | WebGPU | Notes |
|---------|:------:|:------:|-------|
| Vertex shaders | Yes | Yes | |
| Fragment shaders | Yes | Yes | |
| Compute shaders | Yes | Yes | |
| Geometry shaders | Yes | **No** | Not in WebGPU |
| Tessellation shaders | Yes | **No** | Not in WebGPU |
| Mesh shaders | Extension | **No** | Not in WebGPU |
| Ray generation/hit/miss | Extension | **No** | Not in WebGPU |

### 2.3 Shader Features

| Feature | Vulkan | WebGPU | Notes |
|---------|:------:|:------:|-------|
| Texture sampling | Yes | Yes | |
| Storage buffers | Yes | Yes | `var<storage>` in WGSL |
| Uniform buffers | Yes | Yes | `var<uniform>` in WGSL |
| Push constants | Yes | **No** | Use uniform buffers instead |
| 16-bit types | Extension | Limited | `f16` available in WebGPU |
| 64-bit types | Extension | **No** | No `f64`/`i64` in WGSL |
| Atomic operations | Yes | Yes | |
| Derivative functions | Yes | Yes | Fragment only |
| Texture gather | Yes | Yes | |

---

## 3. Resource Binding

### 3.1 Binding Model

| Aspect | Vulkan | WebGPU | Notes |
|--------|:------:|:------:|-------|
| Descriptor sets | 4+ | 4 | WebGPU: max 4 bind groups |
| Bindings per set | 128+ | 1000 | Per adapter limits |
| Dynamic offsets | Yes | Yes | |
| Inline uniform blocks | Extension | **No** | Use uniform buffers |

### 3.2 Bindless / Resource Arrays

| Feature | Vulkan | WebGPU | Notes |
|---------|:------:|:------:|-------|
| Texture arrays | Yes | **No** | WebGPU arrays are fixed-size |
| Buffer arrays | Yes | **No** | Not in base spec |
| Non-uniform indexing | Yes | **No** | Security concern in browsers |
| Descriptor indexing | Yes | **No** | |

**Impact**: Scenes with many materials/textures require multiple draw calls with different bind groups rather than a single bindless draw.

**Blade Status**:
- `TextureArray::bind_to()` → `unimplemented!()`
- `BufferArray::bind_to()` → `unimplemented!()`

---

## 4. Buffer Features

### 4.1 Buffer Types

| Memory Type | Vulkan | WebGPU | Notes |
|-------------|:------:|:------:|-------|
| Device-local | Yes | Yes | `Memory::Device` |
| Host-visible | Yes | Yes | `Memory::Shared` / `Memory::Upload` |
| External/imported | Yes | **No** | Cross-process not supported |

### 4.2 Buffer Operations

| Operation | Vulkan | WebGPU | Notes |
|-----------|:------:|:------:|-------|
| Create | Yes | Yes | |
| Map/unmap | Direct | Shadow | WebGPU uses shadow memory + sync |
| Fill | Yes | Yes | |
| Copy buffer→buffer | Yes | Yes | |
| Copy buffer→texture | Yes | Yes | |
| Copy texture→buffer | Yes | Yes | |
| Persistent mapping | Yes | **No** | WebGPU requires explicit mapping |

**Blade Shadow Buffer Model**: For `Memory::Upload` and `Memory::Shared`, blade maintains CPU-side shadow memory. Changes are synced via `queue.write_buffer()` before submit. This avoids WebGPU's complex async buffer mapping.

### 4.3 Buffer Address

| Feature | Vulkan | WebGPU | Notes |
|---------|:------:|:------:|-------|
| Buffer device address | Yes | **No** | Pointers not exposed in WebGPU |

---

## 5. Texture Features

### 5.1 Texture Types

| Type | Vulkan | WebGPU | Notes |
|------|:------:|:------:|-------|
| 1D | Yes | Yes | |
| 2D | Yes | Yes | |
| 3D | Yes | Yes | |
| Cube | Yes | Yes | |
| Array | Yes | Yes | |
| Multisampled | Yes | Yes | 1, 4 samples typically |

### 5.2 Texture Formats

| Format Category | Vulkan | WebGPU | Notes |
|-----------------|:------:|:------:|-------|
| R8/RG8/RGBA8 | Yes | Yes | |
| R16/RG16/RGBA16 | Yes | Yes | |
| R32/RG32/RGBA32 Float | Yes | Yes | |
| Depth24/32 | Yes | Yes | |
| Depth24Stencil8 | Yes | Yes | |
| BC1-7 (S3TC/DXT) | Yes | Yes | Widely supported |
| ASTC | Extension | Extension | Mobile-focused |
| ETC2 | Extension | Extension | Mobile-focused |

### 5.3 Texture Operations

| Operation | Vulkan | WebGPU | Notes |
|-----------|:------:|:------:|-------|
| Sampling | Yes | Yes | |
| Storage read/write | Yes | Yes | |
| Copy texture→texture | Yes | Yes | |
| Mipmap generation | Manual | Manual | No auto-gen in either |
| Resolve MSAA | Yes | Yes | |

---

## 6. Synchronization

| Mechanism | Vulkan | WebGPU | Notes |
|-----------|:------:|:------:|-------|
| Timeline semaphores | Yes | **No** | WebGPU uses submission index |
| Binary semaphores | Yes | **No** | |
| Fences | Yes | Implicit | `device.poll()` with timeout |
| Barriers | Explicit | Implicit | WebGPU tracks internally |
| Events | Yes | **No** | |

**Blade Abstraction**: `SyncPoint` maps to timeline semaphore (Vulkan) or submission index (WebGPU).

---

## 7. Advanced Features

### 7.1 Blending

| Feature | Vulkan | WebGPU | Notes |
|---------|:------:|:------:|-------|
| Standard blend ops | Yes | Yes | |
| Dual-source blending | Yes | Extension | Chrome 130+ with `dual-source-blending` feature |
| Logic operations | Yes | **No** | |

**Dual-Source Blending** use cases:
- Sub-pixel font rendering (ClearType-style)
- Colored glass with reflection

**Blade Status**: `capabilities().dual_source_blending` returns `false` for WebGPU backend.

### 7.2 GPU Timing / Profiling

| Feature | Vulkan | WebGPU | Notes |
|---------|:------:|:------:|-------|
| Timestamp queries | Yes | Extension | `timestamp-query` feature |
| Pipeline statistics | Yes | **No** | |
| Timestamp resolution | Nanoseconds | 100μs | Browser quantization for security |
| Async readback | Direct | Required | WebGPU requires async `mapAsync` |

**Blade Status**:
- Vulkan: Full GPU timing
- WebGPU: Infrastructure exists, WASM readback not implemented (issue `blade-9rv`)

**Workaround**: Use browser DevTools, Perfetto tracing (`gpu.dawn`), or WebGPU Inspector extension.

### 7.3 Indirect Operations

| Feature | Vulkan | WebGPU | Notes |
|---------|:------:|:------:|-------|
| Indirect draw | Yes | Yes | |
| Indirect indexed draw | Yes | Yes | |
| Indirect dispatch | Yes | Yes | |
| Multi-draw indirect | Extension | **No** | Single indirect call only |
| Draw count | Extension | **No** | |

### 7.4 Queries

| Query Type | Vulkan | WebGPU | Notes |
|------------|:------:|:------:|-------|
| Occlusion | Yes | Yes | |
| Timestamp | Yes | Extension | |
| Pipeline statistics | Yes | **No** | |

---

## 8. Platform & Runtime

### 8.1 Initialization

| Aspect | Vulkan | WebGPU | Notes |
|--------|:------:|:------:|-------|
| Sync init | Yes | Native only | `Context::init()` |
| Async init | No | WASM required | `Context::init_async()` |

### 8.2 Surface / Presentation

| Feature | Vulkan | WebGPU | Notes |
|---------|:------:|:------:|-------|
| Windowed | Yes | Yes | |
| Fullscreen | Yes | Yes | |
| VSync modes | All | Fifo/Mailbox | Limited present modes |
| HDR | Extension | Future | Not yet standardized |

### 8.3 Browser Support (WebGPU)

| Browser | Platform | Status |
|---------|----------|--------|
| Chrome | Windows/macOS/ChromeOS | Stable (113+) |
| Chrome | Android | Stable (121+) |
| Chrome | Linux | Behind flags |
| Firefox | Windows/macOS | Stable (141+) |
| Firefox | Linux | 2026 expected |
| Safari | macOS/iOS 26+ | Stable |

---

## 9. Blade Backend Capabilities Summary

```rust
// Vulkan
Capabilities {
    ray_query: ShaderVisibility::all(),  // VERTEX | FRAGMENT | COMPUTE
    sample_count_mask: 0b1111,           // 1, 2, 4, 8
    dual_source_blending: true,
}

// WebGPU
Capabilities {
    ray_query: ShaderVisibility::empty(), // None
    sample_count_mask: 0b0101,            // 1, 4
    dual_source_blending: false,
}
```

---

## 10. Migration Guide: Vulkan → WebGPU

### 10.1 Remove Ray Tracing

Replace hardware ray tracing with:
- Compute shader path tracing
- Screen-space techniques (SSR, SSAO)
- Precomputed lighting (lightmaps, probes)

### 10.2 Replace Bindless

```rust
// Vulkan: Single draw with material index
pe.bind(0, &material_array);
pe.draw_indirect(...);

// WebGPU: Multiple draws with bind group switches
for material in materials {
    pe.bind(0, &material.bind_group);
    pe.draw(...);
}
```

Consider batching materials by texture to reduce bind group switches.

### 10.3 Replace Push Constants

```rust
// Vulkan: Push constants (fast, no allocation)
pe.bind(0, &transform);

// WebGPU: Uniform buffer (blade handles this transparently)
pe.bind(0, &transform); // Same API, uses triple-buffered UBO internally
```

### 10.4 Handle Dual-Source Blending

```rust
// Check capability
if context.capabilities().dual_source_blending {
    // Use Src1/OneMinusSrc1 blend factors
} else {
    // Fallback: two-pass rendering or different technique
}
```

### 10.5 GPU Timing

```rust
// Vulkan: Direct timing
let times = context.timing_results();

// WebGPU (WASM): Use browser tools
// - Chrome DevTools Performance tab
// - Perfetto gpu.dawn category
// - WebGPU Inspector extension
```

---

## 11. Future WebGPU Extensions

Extensions in development or proposed:

| Extension | Status | Impact |
|-----------|--------|--------|
| Subgroups | Proposal | Better compute perf |
| Ray tracing | Proposal ([#535](https://github.com/gpuweb/gpuweb/issues/535)) | Would close critical gap |
| Bindless | Under discussion | Would enable modern rendering |
| Dual-source blending | Chrome 130+ | Font rendering |
| Timestamp queries | Available | Already in blade |

---

## 12. References

- [W3C WebGPU Specification](https://www.w3.org/TR/webgpu/)
- [WebGPU Explainer](https://gpuweb.github.io/gpuweb/explainer/)
- [MDN WebGPU API](https://developer.mozilla.org/en-US/docs/Web/API/WebGPU_API)
- [Browser Implementation Status](https://github.com/gpuweb/gpuweb/wiki/Implementation-Status)
- [Ray Tracing Proposal](https://github.com/gpuweb/gpuweb/issues/535)
- [Dual-Source Blending (Chrome)](https://chromestatus.com/feature/5167711051841536)
- [WebGPU Best Practices](https://toji.dev/webgpu-best-practices/)

---

*Document version: 2026-01-07 | blade-graphics v0.7.0*
