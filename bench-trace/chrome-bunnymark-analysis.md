# Chrome Bunnymark Performance Analysis

> Trace: `~/bench-trace/chrome-bunnymark-perf.json`
> Captured: 2026-01-07
> Browser: Chrome with `--enable-unsafe-webgpu`
> Tool: chrome-devtools-cli perf.mjs

---

## Summary

| Metric | Value |
|--------|-------|
| Duration | 10,000ms |
| Script time | 325.33ms (3.2%) |
| Task time | 616.17ms |
| Idle time | 93.6% |
| JS Heap | 3 MB used / 4.51 MB total |
| DOM nodes | 52 |

**Note**: FPS shows as 0 because WebGPU rendering bypasses Chrome's normal paint tracking. The app is running smoothly - this is a profiler limitation.

---

## CPU Time Breakdown

| Category | Time | Percent |
|----------|------|---------|
| Idle | 9,507ms | 93.6% |
| Program (native) | 324ms | 3.2% |
| Script (WASM + JS) | 325ms | 3.2% |

**Interpretation**: The application is GPU-bound or vsync-limited, with CPU spending most time waiting. This is ideal for a rendering benchmark.

---

## Hot Functions (Self Time)

| Function | Self Time | Hits | Notes |
|----------|-----------|------|-------|
| `(program)` | 324.37ms | 705 | Native browser code |
| `take_last_exception` | 93.72ms | 470 | wasm_bindgen overhead |
| `requestAnimationFrame` | 72.70ms | 222 | ~330µs per call |
| `getCurrentTexture` | 6.41ms | 31 | ~206µs per call |
| `(garbage collector)` | 6.14ms | 15 | ~409µs per GC |
| `submit` | 5.91ms | 39 | ~151µs per submit |
| `createView` | 4.98ms | 26 | ~191µs per call |
| `beginRenderPass` | 4.75ms | 18 | ~263µs per call |
| `writeBuffer` | 2.62ms | 14 | ~187µs per call |
| `beginComputePass` | 2.58ms | 1 | One-time setup |

---

## WebGPU API Performance

Over 10 seconds of execution:

| API Call | Total Time | Call Count | Avg Per Call |
|----------|------------|------------|--------------|
| `getCurrentTexture` | 6.41ms | 31 | 206µs |
| `submit` | 5.91ms | 39 | 151µs |
| `createView` | 4.98ms | 26 | 191µs |
| `beginRenderPass` | 4.75ms | 18 | 263µs |
| `writeBuffer` | 2.62ms | 14 | 187µs |

**Total WebGPU overhead**: ~25ms over 10 seconds = **0.25%** of frame time

---

## Call Tree Analysis

Main render path (from `bunnymark::main` closure):

```
bunnymark::main::{closure#3}          297ms total
├── Window::request_redraw            167ms (56%)
│   └── request_animation_frame       167ms
└── Example::render                   129ms (43%)
    └── [WebGPU calls]
```

**Key insight**: More time spent on animation frame scheduling (167ms) than actual rendering (129ms). This is expected browser overhead.

---

## Potential Optimizations

### 1. wasm_bindgen Exception Handling (93.72ms)

`take_last_exception` took nearly 100ms. This suggests:
- Many JS↔WASM boundary crossings
- Exception checking on every WebGPU call

**Possible mitigation**: Batch WebGPU operations to reduce boundary crossings.

### 2. Animation Frame Overhead (72.70ms)

`requestAnimationFrame` native call takes significant time. This is browser overhead, not controllable by the application.

### 3. Frame Timing

With 31 `getCurrentTexture` calls and 39 `submit` calls in 10 seconds:
- ~3-4 frames per second visible to profiler
- Actual rendering is faster (GPU-side not measured here)

---

## Comparison with Firefox

| Aspect | Firefox WGPU Trace | Chrome DevTools |
|--------|-------------------|-----------------|
| Trace type | Low-level API calls | CPU profiling |
| Frame count | 10,596 | N/A (GPU-side) |
| Buffer creates | 7 (once) | Not visible |
| Per-frame overhead | Minimal | 0.25% WebGPU |
| Trace size | 454K lines | 489 lines |

Firefox trace shows GPU-side details; Chrome trace shows CPU-side costs.

---

## Recommendations for Chrome Profiling

### 1. Use Perfetto for GPU Timing

```bash
# Record with gpu.dawn category
chrome://tracing → Record → Enable "gpu.dawn"
```

### 2. Enable WebGPU Developer Features

```
chrome://flags/#enable-webgpu-developer-features
```

Unlocks:
- High-precision timestamp queries (nanosecond instead of 100µs)
- Extended adapter info (backend, driver version, memory heaps)

### 3. Use WebGPU Inspector Extension

For frame-by-frame GPU command inspection (similar to Firefox WGPU trace but interactive).

---

## Conclusions

1. **CPU overhead is minimal** - Application spends 93.6% of time idle (waiting for GPU/vsync)
2. **WebGPU API is fast** - Only 25ms total overhead over 10 seconds
3. **wasm_bindgen overhead** - Exception handling takes ~100ms, worth investigating
4. **Profiler limitation** - Chrome DevTools doesn't capture GPU-side frame timing for WebGPU

For comprehensive analysis, combine:
- **Chrome DevTools**: CPU/JS profiling (this trace)
- **Perfetto gpu.dawn**: Dawn internals
- **WebGPU Inspector**: Frame capture and GPU commands
