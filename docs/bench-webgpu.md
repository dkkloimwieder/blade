Benchmarking and Optimization Methodologies for Rust-Based WebGPU Applications in WebAssembly EnvironmentsExecutive SummaryThe convergence of systems programming and the open web has reached a pivotal juncture with the stabilization of WebGPU and the maturation of WebAssembly (Wasm). This pairing promises a high-performance execution environment capable of delivering fidelity previously reserved for native desktop applications. However, the architectural reality of running Rust-based graphics applications in a browser environment introduces a complex set of performance characteristics that differ fundamentally from native execution.This report provides an exhaustive analysis of the benchmarking methodologies and optimization strategies necessary to navigate this landscape. It deconstructs the wgpu abstraction stack, the wasm-bindgen interoperability layer, and the browser-level security constraints that obfuscate performance metrics. By examining the limitations of high-resolution timing due to Spectre mitigations, the overhead of the Wasm-to-JavaScript boundary, and the intricacies of GPU command submission in an asynchronous event loop, this document establishes a rigorous protocol for performance analysis. Furthermore, it details advanced optimization patterns—such as the use of Staging Belts for memory management, Render Bundles for CPU-bound draw call reduction, and SIMD vectorization—to bridge the performance gap between native and web targets.1. Introduction: The Convergence of Rust, Wasm, and WebGPUThe trajectory of web-based graphics has historically been defined by the capabilities of WebGL, a standard derived from OpenGL ES. While revolutionary for its time, WebGL relied on a global state machine model that incurred significant CPU overhead for driver validation and state management. As modern GPUs evolved toward explicit, low-overhead architectures (represented by Vulkan, Metal, and Direct3D 12), the disparity between web and native graphics performance widened. WebGPU addresses this by exposing a modern, explicit API that aligns with contemporary hardware architectures, promising a dramatic reduction in driver overhead and unlocking the potential for general-purpose GPU compute (GPGPU) on the web.1Simultaneously, WebAssembly has emerged as the standard for high-performance logic on the web, allowing languages like Rust to execute with near-native performance. The wgpu crate in the Rust ecosystem serves as the idiomatic bridge between these technologies, offering a safe, portable abstraction that targets WebGPU on the web and native APIs on desktop platforms.3However, the promise of "write once, run anywhere" does not equate to "profile once, optimize everywhere." The execution environment of a Rust application running in Wasm is fundamentally different from a native binary. The application runs inside a sandboxed virtual machine, communicates with the GPU via a browser-mediated IPC (Inter-Process Communication) layer, and is subject to strict security constraints regarding timing and resource access.4 This report aims to equip the domain expert with the theoretical understanding and practical methodologies required to benchmark and optimize specifically for this constrained environment.2. Architectural Analysis of the wgpu StackTo effectively benchmark an application, one must first understand the layers through which a command travels. In the context of wgpu and WebAssembly, this stack is significantly deeper than in native applications.2.1 The Native Execution Path vs. The Web PathIn a native environment, wgpu functions as a wrapper around wgpu-core and wgpu-hal (Hardware Abstraction Layer). When a Rust application issues a draw call, wgpu-core performs validation and state tracking, and wgpu-hal translates the command directly into the underlying API (e.g., Vulkan, Metal, or DX12). The overhead here is entirely within the application process, controlled by the Rust compiler and the efficiency of the wgpu implementation.3On the web, the architecture diverges. When wgpu compiles to the wasm32-unknown-unknown target:wgpu-native becomes a pass-through: The native backend logic is stripped away. Instead, wgpu calls link to functions provided by web-sys and js-sys, which wrap the browser's JavaScript WebGPU API.3The Wasm Boundary: Every API call traverses the boundary between the Wasm linear memory and the JavaScript environment via wasm-bindgen.The Browser Implementation: The browser (e.g., Chrome) receives the call. It performs its own validation of the WebGPU command to ensure the security of the standard.The Browser Backend: The browser's implementation (e.g., Dawn in Chrome, or wgpu-core in Firefox) translates the command to the native OS API.This "double validation" model—where both the Rust wgpu crate and the browser perform checks—creates a unique performance profile. A benchmark running on native wgpu bypasses the browser's IPC and validation layers entirely, making it an unreliable predictor of web performance.4 The implication is that benchmarking must occur in situ within the browser environment to capture the true cost of command submission.2.2 The Role of wasm-bindgen and IPC OverheadThe interoperability layer, wasm-bindgen, is responsible for marshaling data between Rust and JavaScript. Since WebAssembly currently lacks direct access to the DOM or Web API objects, Rust holds opaque "handles" (indices into a JavaScript table) representing WebGPU objects like GPUDevice or GPUBuffer.6Every interaction with these handles incurs overhead. While simple integer passing is fast, complex operations involving structs or strings require serialization or memory copying. For example, setting a pipeline object involves looking up the JS object corresponding to the Rust handle and invoking a method on it. In high-frequency scenarios—such as issuing thousands of individual draw commands per frame—the cumulative cost of these boundary crossings can exceed the cost of the GPU work itself. This phenomenon, often referred to as being "CPU-bound by the API," is a critical bottleneck in Wasm graphics applications.7 Benchmarking strategies must therefore explicitly isolate the "encoding time" (CPU) from the "execution time" (GPU).2.3 Browser Compositing and PresentationUnlike a native window where the swap chain interacts directly with the OS compositor, a WebGPU canvas is part of the browser's DOM. The browser's compositor manages the presentation, potentially introducing latency or frame pacing behaviors distinct from a native exclusive-fullscreen application. The requestAnimationFrame loop, which drives the rendering cycle, is synchronized with the browser's repaint cycle, typically capped at the display refresh rate (V-Sync). Benchmarking frame rates uncapped is therefore difficult without specific browser flags, complicating the measurement of raw throughput.93. CPU Benchmarking MethodologiesAccurately measuring the performance of Rust code running in WebAssembly requires navigating the limitations of the browser's timing APIs and the lack of direct hardware instrumentation.3.1 The Precision Crisis: performance.now() and SpectreIn native Rust, std::time::Instant provides access to high-resolution monotonic clocks (e.g., CLOCK_MONOTONIC on Linux, QPC on Windows). In the Wasm target, std::time::Instant maps to the browser's performance.now() API.9Following the discovery of transient execution vulnerabilities (Spectre and Meltdown), browser vendors introduced security mitigations to prevent timing attacks. A key mitigation was the reduction of timer precision.Quantization: performance.now() does not return the exact microsecond. It is typically rounded to the nearest 100 microseconds (0.1ms), though this varies by browser and isolation context.10Jitter: Some implementations inject random noise into the timestamp to further obfuscate precise timing intervals.Implication for Benchmarking: This quantization renders micro-benchmarking of individual functions impossible using standard timing. A function taking 40µs might be reported as 0µs or 100µs randomly. To obtain statistically significant data, benchmarks must employ aggregation strategies:Batch Execution: Run the target function $N$ times (where $N$ is large, e.g., 10,000) inside a single timing block such that the total duration significantly exceeds the quantization threshold (e.g., >50ms).Statistical Smoothing: Collect thousands of frame time samples and analyze the distribution (P99, P50) rather than relying on instantaneous measurements, which are plagued by jitter.93.2 Instrumentation Profiling with PuffinGiven the opacity of the browser execution stack, instrumentation profiling is often more valuable than sampling profiling. The puffin crate has emerged as a standard in the Rust graphics ecosystem for this purpose.12Puffin operates by manually defining scopes within the code:Rustfn update() {
    puffin::profile_function!();
    //... logic...
    {
        puffin::profile_scope!("physics_step");
        physics.step();
    }
}
In a Wasm environment, Puffin collects these timings in thread-local storage. Since Wasm execution is single-threaded (in the main event loop context), this data capture is efficient. The data can then be visualized in several ways:In-Game Overlay: Using puffin_egui, the flamegraph can be rendered directly onto the WebGPU canvas. This is particularly powerful for WebAssembly, as it provides real-time feedback without needing to connect to external developer tools.14Remote Viewing: puffin_http allows the Wasm application to stream profiling data over a WebSocket to a native viewer application (puffin_viewer). This decouples the visualization overhead from the browser performance.12Overhead Considerations: The overhead of a Puffin scope is approximately 50-200 nanoseconds.13 While negligible for coarse-grained systems (e.g., "Physics", "Render"), it is significant enough that it should not be placed inside tight inner loops (e.g., per-entity iteration) during critical performance testing.3.3 Leveraging Chrome DevTools and DWARFWhile instrumentation provides logic-level insight, it misses system-level events (e.g., Garbage Collection, JIT compilation). The Chrome DevTools Performance tab fills this gap. Historically, profiling Wasm in DevTools was difficult because the call stack would only show opaque references like wasm-function.Source Maps and DWARF:Modern browser tooling now supports DWARF debugging information. By configuring the Rust build to include debug symbols even in release mode, developers can see original function names in the browser profiler.Configuration:In Cargo.toml:Ini, TOML[profile.release]
debug = true  # Includes DWARF info
lto = "fat"   # Maintains optimization
This setup enables the DevTools to map the compiled Wasm instructions back to the Rust source code, allowing developers to identify exactly which Rust functions are triggering CPU bottlenecks or excessive wasm-bindgen glue execution.163.4 Benchmarking "Headless" LogicFor logic that does not depend on the GPU (e.g., physics, AI), it is possible to benchmark Wasm outside the browser using runtimes like wasmtime or Node.js. However, care must be taken. The JIT compilers in Node.js (V8) and standalone runtimes (Cranelift in Wasmtime) have different performance characteristics. A benchmark run in wasmtime may not perfectly predict performance in Chrome's V8, although they typically correlate well for compute-heavy tasks.44. GPU Benchmarking MethodologiesProfiling the GPU on the web is significantly more restricted than on native platforms due to the inability to interface with driver-level tools like NVIDIA Nsight or AMD RGP. Benchmarking relies on API-level queries and careful inference.4.1 Timestamp Queries: The Gold StandardThe WebGPU specification includes timestamp-query features, which allow the application to request that the GPU write the current timestamp (in nanoseconds) to a query set at specific points in the command stream (e.g., start and end of a compute pass).18Mechanism:Feature Request: The device must be initialized with wgpu::Features::TIMESTAMP_QUERY.20Recording: command_encoder.write_timestamp(&query_set, index) is called to record a time marker.Resolution: resolve_query_set writes the timestamps to a buffer.Readback: The buffer is mapped to the host, and the difference between start and end timestamps is calculated.The Security Barrier:Crucially, timestamp queries are disabled by default in browsers to prevent timing attacks that could infer data based on execution duration.To use them for benchmarking, the developer must launch the browser with specific flags. In Chrome/Chromium, the flag is --disable-dawn-features=disallow_unsafe_apis or enabling "WebGPU Developer Features" in chrome://flags.10Even when enabled, browsers may apply quantization (rounding to 100µs) unless specifically disabled via flags. This makes them unsuitable for measuring very short passes without the correct flag configuration.104.2 The wgpu-profiler EcosystemThe wgpu-profiler crate encapsulates the complexity of managing query sets and resolving buffers. It provides a macro-based API (wgpu_profiler!("scope", encoder, device,...)) that automatically injects the necessary timestamp writes.22Integration Strategy:Conditional Compilation: Since requesting the TIMESTAMP_QUERY feature will cause device creation to fail on standard user browsers, wgpu-profiler usage should be guarded by a feature flag (e.g., cargo run --features profile-gpu).Visualization: The crate can export data to the Chrome Tracing JSON format, allowing the GPU timeline to be viewed in chrome://tracing or the Perfetto UI. It also integrates with Puffin/Tracy, allowing GPU blocks to appear alongside CPU blocks in the timeline, which is critical for visualizing CPU-GPU parallelism and pipeline bubbles.124.3 Pipeline Statistics QueriesBeyond time, WebGPU supports pipeline-statistics-query, which can report metrics like "vertex shader invocations," "clipper invocations," and "fragment shader invocations".24Utility: These metrics are invaluable for identifying bottlenecks. For example, a high ratio of vertex invocations to primitives generated might indicate inefficient geometry processing.Availability: Like timestamps, these are often restricted or require specific feature flags depending on the browser and backend implementation.4.4 Disjoint Timer AnomaliesIn native graphics programming, "disjoint" timer queries occur when the GPU is interrupted (e.g., by a context switch). The WebGPU spec handles this by invalidating query results if the context was lost or if timing reliability cannot be guaranteed. Benchmarking harnesses must be robust to resolve_query_set failing or returning invalid data, treating such frames as outliers to be discarded.255. Memory Architecture and Data Transfer OptimizationIn high-performance WebGPU applications, the bottleneck is frequently not the GPU's compute capability, but the bandwidth of moving data from the CPU (Wasm) to the GPU.5.1 The "Double Copy" PenaltyThe concept of "Zero-Copy" data transfer is a Holy Grail that is rarely achievable in the current Wasm ecosystem.The Architecture: Wasm operates in a linear memory space. JavaScript (and the browser's WebGPU implementation) cannot simply "take ownership" of a pointer to Wasm memory because the Wasm memory can grow (reallocate), invalidating pointers.The Transfer Path:Rust: Data is prepared in a Vec<u8> or struct in Wasm memory.Bindgen: A Uint8Array view is created over this Wasm memory.Browser API: queue.write_buffer is called with this view.IPC Copy: The browser implementation (e.g., Chrome's GPU process) creates a copy of this data to send it safely to the GPU command queue, isolating it from potential modification by the Wasm thread.6This implies that for every byte uploaded to the GPU, at least one CPU-side copy occurs within the browser engine, in addition to the eventual DMA transfer to the GPU.5.2 Staging Belts: Mitigating Allocation ChurnTo optimize transfer, the wgpu::util::StagingBelt pattern is highly recommended over naive queue.write_buffer calls for dynamic data.28Mechanism:A Staging Belt maintains a ring of pre-allocated wgpu::Buffers mapped with MAP_WRITE.Allocation: Instead of the browser allocating a new temporary buffer for every write, the application requests a slice of the existing mapped staging buffer.Write: The application writes data into this mapped slice. In Wasm, this is a write into a JS-backed ArrayBuffer.Command: A copy_buffer_to_buffer command is encoded to move data from the staging buffer to the destination GPU buffer.Reuse: Once the frame is submitted and processing is complete, the staging buffer space is recalled and reused.Analysis of Snippet 30 (Memory Leak):Research highlights a potential pitfall with Staging Belts in Wasm. If the application allocates new chunks in the belt faster than the GPU processes and releases them—or if the "recall" mechanism isn't triggered correctly—memory usage can spike. In the "Game of Life" example referenced, the application failed to reuse chunks effectively, causing the browser to consume gigabytes of memory. This underscores the need for explicit resource management: ensuring belt.finish() is called and that the returned futures are polled to completion to free the chunks.305.3 Mapped at Creation for Static DataFor immutable data (textures, static meshes), the most efficient method is mapped_at_creation: true.Optimization: When a buffer is created with this flag, wgpu returns a mapped range immediately. The data is written once. Upon unmap(), the buffer is ready for GPU use.Advantage: This bypasses the need for a separate staging buffer and the associated copy command, reducing startup time and memory footprint.325.4 Data Alignment and PaddingWebGPU enforces strict alignment rules (e.g., std140 layout). A vec3<f32> in WGSL takes up 16 bytes (aligned to 16 bytes), not 12.Performance Hit: If Rust structs are not padded to match this alignment, the runtime or the developer must perform a field-by-field copy to a correctly padded buffer before upload.Solution: Use crates like encase or crevice to define structs. These crates utilize Rust's const generics to ensure that the memory layout in Wasm perfectly matches the GPU expectation, allowing for memcpy operations rather than element-wise copying.6. Advanced Optimization Strategies6.1 Reducing Draw Call Overhead: Render BundlesOne of the most significant bottlenecks in Wasm graphics is the CPU overhead of recording commands. A typical frame might involve thousands of set_pipeline, set_bind_group, and draw calls. In native code, these are fast. In Wasm, each is an FFI call through wasm-bindgen.7Render Bundles (wgpu::RenderBundle):Render Bundles allow a sequence of render commands to be recorded once and replayed multiple times.Benchmark Evidence: In the "Bunnymark" benchmark 7, switching to a batching strategy or using Render Bundles significantly increased the number of sprites possible.Best Practice: For static geometry, record the draw commands into a Bundle during initialization. In the render loop, execute the bundle with a single render_pass.execute_bundles() call. This collapses thousands of Wasm-to-JS transitions into one.6.2 Bind Group FrequencyChanging Bind Groups is expensive. It breaks the command stream and requires driver validation.Strategy: Organize data by update frequency.Group 0: Global data (Camera, Time) - Changed once per frame.Group 1: Material data (Textures) - Changed per material.Group 2: Object data (Transforms) - Changed per object.Optimization: Use "Dynamic Uniform Buffers" or "Storage Buffers" for Object data. Instead of rebinding Group 2 for every object, bind one large buffer containing all object transforms, and pass the dynamic offset or an index to the draw call. This drastically reduces the number of set_bind_group calls.336.3 SIMD Vectorization in WasmWhile Wasm is 32-bit (currently), it supports the fixed-width SIMD proposal (128-bit).Impact: Physics calculations, frustum culling, and matrix multiplication performed on the CPU can see 2x-4x speedups by using SIMD.34Implementation: Enable the simd128 target feature in .cargo/config. Use crates like glam or ultraviolet which map Rust vector types to Wasm SIMD intrinsics.Benchmark Note: Benchmarks comparing Wasm to Native must account for SIMD. If the Native build uses AVX2 (256-bit) and Wasm uses SIMD128, a performance discrepancy is expected purely due to hardware instruction width, not just Wasm overhead.366.4 Async Compute and Work SubmissionWebGPU submission is asynchronous. queue.submit() returns immediately.Pacing: To avoid flooding the GPU command queue (which can lead to latency accumulation and input lag), applications should manage the "frame in flight" count.Callbacks: Use queue.on_submitted_work_done() (or strictly, the Poll mechanism in wgpu) to track when the GPU has finished a frame.Wasm Specific: In Wasm, device.poll() is a no-op because the browser manages the event loop. Synchronization must rely on awaiting the futures returned by mapping buffers or submission callbacks. Do not attempt to spin-loop (busy wait) in Wasm; it will freeze the browser tab.377. Comparative Analysis: Native vs. WebTo interpret benchmark results, one must establish a baseline. The following comparison table summarizes the expected performance characteristics based on the research.FeatureNative (Rust + Vulkan/Metal/DX12)Web (Rust + Wasm + WebGPU)Implications for BenchmarkingCPU ExecutionNative Speed (AVX/SSE enabled)~1.5x - 2.0x Slower (Wasm Overhead) 34Logic-heavy frames will be CPU bound on Web.GPU ExecutionNative Hardware SpeedIdentical (Native Hardware Speed) 4Shader code runs at same speed.Draw Call CostLowHigh (JS/Wasm Boundary) 7Web benchmarks punish high draw counts.Memory UploadDirect Memory AccessDouble Copy (Wasm->JS->GPU) 6Bandwidth benchmarks will show Web bottleneck.Timing PrecisionNanoseconds (Instant)100µs + Jitter (performance.now) 10Micro-benchmarks require aggregation on Web.Integer Math64-bit Native64-bit Emulated (Slow) 34Avoid u64 in hot loops in Wasm.MultithreadingFull OS ThreadsWeb Workers (SharedArrayBuffer)Synchronization overhead is higher on Web.Case Study: The "Bunnymark" ScenarioThe "Bunnymark" is a standard stress test involving rendering tens of thousands of sprites.Native Performance: Can easily handle 100k+ sprites using individual draw calls.Web Performance: Performance collapses at a lower count if using individual draw calls. The bottleneck is not the GPU (fill rate); it is the CPU time required to cross the Wasm-JS boundary 100,000 times per frame.Optimization: Switching to instanced rendering (drawing 100k instances in 1 call) restores performance parity. The GPU workload remains high, but the CPU-API overhead vanishes. This illustrates that WebGPU benchmarking is largely an exercise in identifying API-bound CPU bottlenecks.7Case Study: Physics Simulation (Bullet)Research on porting the Bullet physics engine to Wasm 38 highlights the compute potential.Findings: Wasm achieved up to 9.88x speedup over asm.js/JavaScript equivalents but trailed native.Key Optimization: Multithreading via Web Workers provided significant scalability.Relevance to WebGPU: Offloading physics to a separate Web Worker prevents the physics loop from blocking the main rendering thread. However, synchronizing the transform data from the Physics Worker to the Render Worker/Main Thread involves SharedArrayBuffer usage, which must be carefully managed to avoid locking contention.8. Tooling and Configuration Guide8.1 Recommended Cargo ProfileTo ensure benchmarks are representative of production potential while maintaining debuggability:Ini, TOML[package]
name = "my-wgpu-app"
version = "0.1.0"
edition = "2021"

[dependencies]
wgpu = "0.19" # Or latest
wasm-bindgen = "0.2"
puffin = "0.19"
puffin_egui = "0.24"

[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
debug = true  # CRITICAL for source maps [17]
8.2 Browser Flags for BenchmarkingTo enable precise profiling features that are otherwise blocked:Chrome/Edge:--enable-dawn-features=allow_unsafe_apis: Enables Timestamp Queries.--disable-dawn-features=disallow_unsafe_apis: (Alternative syntax depending on version).--enable-webgpu-developer-features: Enables access to detailed adapter info.Firefox:dom.webgpu.enabled: true.dom.webgpu.workers.enabled: true (if testing threads).8.3 The Benchmarking Loop ProtocolA robust benchmark implementation should follow this state machine:Warm-up: Run the rendering loop for 60-100 frames to allow the browser's JIT (Just-In-Time) compiler to optimize the wasm-bindgen glue code and to allow GPU caches to fill.Capture Phase:Reset aggregators.Enable Puffin scopes.Record performance.now() at start/end of frame.Issue Timestamp Queries (if enabled).Cooldown/Report:Stop capture after N frames.Calculate Mean, P99, and Standard Deviation.Discard frames where disjoint timer events occurred.Log results to console or display on overlay.9. ConclusionBenchmarking Rust and WebAssembly in the WebGPU era demands a sophisticated understanding of the underlying abstraction layers. The notion that "WebGPU makes the web as fast as native" is nuanced: while the GPU hardware executes instructions at native speed, the pathway to the GPU is fraught with CPU-side obstacles—specifically the Wasm-JS boundary, memory copy overheads, and security-driven timing limitations.The research indicates that successful optimization relies on:Minimizing API Surface Area: Using Render Bundles, Instancing, and large Storage Buffers to reduce the frequency of Wasm-to-JS calls.Intelligent Memory Management: Using Staging Belts and mapped_at_creation to circumvent the "double copy" penalty.Instrumented Observability: Relying on internal instrumentation (Puffin) and developer-flag-enabled Timestamp Queries rather than the coarse and noisy performance.now().By adhering to these methodologies, developers can construct high-fidelity benchmarks that isolate the true bottlenecks, enabling the creation of web applications that genuinely rival their native counterparts in performance and capability.

## 10. Empirical Analysis: Bunnymark WebGPU Trace

This section documents findings from Chrome DevTools performance trace analysis of the blade bunnymark example running with the WebGPU backend.

### 10.1 Trace Overview

**Trace file**: `bench-trace/Trace-webgpu-260104-3.json`

| Metric | Value |
|--------|-------|
| Total events | 72,340 |
| Duration | 5.18 seconds |
| DrawFrame events | 268 |
| GPUTask events | 748 |
| RequestAnimationFrame | 274 |

### 10.2 Frame Timing Analysis

**Key Finding: Zero frame drops, stable 60fps**

| Metric | Value |
|--------|-------|
| Average frame time | 16.63ms |
| Maximum frame time | 16.71ms |
| Minimum frame time | 0ms |
| Frames over 20ms | 0 |
| Frames over 33ms | 0 |

The bunnymark achieves perfect V-Sync pacing with no observable jank or frame drops throughout the 5-second capture.

### 10.3 GPU Task Performance

| Metric | Value |
|--------|-------|
| GPU task count | 748 |
| Maximum duration | 37.7ms |
| Average duration | 4.1ms |
| Minimum duration | 0.001ms |

### 10.4 GPU Memory Usage

| Metric | Value |
|--------|-------|
| Minimum | 0 MB (startup) |
| Maximum | 28.8 MB |
| Average | 21.4 MB |

Memory usage remains stable throughout the trace, indicating no memory leaks in the WebGPU resource management.

### 10.5 WASM Compilation (Startup Cost)

| Event | Count | Total Time |
|-------|-------|------------|
| v8.wasm.compileConsume | 22 | 72.1ms |
| v8.wasm.compileConsumeDone | 1 | 9.6ms |

WASM compilation is a one-time startup cost. The streaming compilation (22 events) processes the module incrementally.

### 10.6 V8 Profiler Overhead

**Critical Finding: 145ms blocking task is DevTools overhead, not application code**

A 145ms blocking event appears in the trace with this call stack:
```
RunTask (146.5ms)
└─ RunMicrotasks (146.4ms)
   └─ v8.callFunction (145.6ms)
      └─ FunctionCall (145.6ms)
         └─ v8::Debugger::AsyncTaskRun (145.1ms)
            └─ V8Console::runTask (145.1ms)
```

This is V8 debugger/console instrumentation overhead, **not** application code. The event occurs once and does not recur during normal rendering. This is a common artifact when profiling with DevTools and should not be interpreted as a performance bug.

Additionally, the trace contains significant profiler overhead:
- 9,244 V8.StackGuard events
- 9,244 V8.HandleInterrupts events
- 546 ProfileChunk events
- 1,219 v8::Debugger::AsyncTaskCanceled events

### 10.7 Memory Pressure Events

| Event | Count | Total Time |
|-------|-------|------------|
| V8.ExternalMemoryPressure | 19 | 10.2ms |
| V8.GC_MC_FINISH_SWEEP_ARRAY_BUFFERS | 30 | 0.1ms |

The V8.ExternalMemoryPressure events indicate the browser's garbage collector is managing external resources (GPU buffers), but the total time (10.2ms over 5 seconds) is negligible.

### 10.8 CPU Profile Analysis (Call Stack)

**Critical: The trace JSON must be analyzed for CPU profile samples, not just timeline events.**

Use `scripts/analyze-trace.py` to extract call stack data:

```bash
python3 scripts/analyze-trace.py bench-trace/Trace-webgpu-260104-3.json
```

The CPU profiler reveals the **actual** bottleneck path:

```
CALL TREE (showing cumulative time)
────────────────────────────────────────────────────────────────
js-to-wasm                          (19.7%)
└─ winit event loop                 (15.6%)
   └─ bunnymark::render             (12.2%)
      └─ Context::submit            (11.3%)
         └─ sync_dirty_buffers      (7.1%)
            └─ write_buffer         (7.4%)
               └─ Uint8Array::new_from_slice (5.8%)
                  └─ __wbg_new_from_slice    (5.6% SELF) ← BOTTLENECK
```

**Bottleneck Summary from CPU profiler:**

| Category | Total % | Root Cause |
|----------|---------|------------|
| JS→WASM Call | 19.7% | wasm-bindgen call overhead |
| GPU Buffer Upload | 17.8% | write_buffer + JS array creation |
| WASM→JS Memory Copy | 17.2% | Uint8Array::new_from_slice |
| WASM→JS Call | 16.1% | Boundary crossing overhead |
| Dirty Buffer Sync | 7.1% | sync_dirty_buffers pattern |

**Key Finding**: The `Uint8Array::new_from_slice` function accounts for ~6% of CPU time as self-time, but ~17% when including the calling write_buffer path. This is the WASM→JS memory copy that occurs on every buffer upload.

### 10.9 Shadow Memory Analysis

The blade WebGPU backend uses a "shadow memory" pattern for Upload/Shared buffers:

```rust
// blade-graphics/src/webgpu/mod.rs:803-815
fn sync_dirty_buffers(&self) {
    for entry in hub.buffers.iter() {
        if entry.dirty.load(Ordering::Acquire) {
            self.queue.write_buffer(&entry.gpu, 0, shadow);
        }
    }
}
```

**Data flow causing double-copy:**
```
Rust data → [CPU copy] → shadow buffer → [WASM→JS copy] → Uint8Array → GPU
```

For bunnymark's instance buffer (100k × 32 bytes = 3.2MB), this results in 6.4MB of memory bandwidth per frame.

### 10.10 Conclusions

1. **Frame timing is excellent**: Stable 60fps with zero frame drops
2. **The 145ms blocking task is DevTools overhead**: V8Console::runTask, not application code
3. **WASM→JS copy IS the bottleneck**: 17% of CPU time in write_buffer path
4. **Uint8Array::new_from_slice is the hottest function**: 5.6% self-time
5. **The double-copy pattern is measurable overhead**: 7.1% in sync_dirty_buffers

### 10.11 Optimization Priorities

Based on CPU profile analysis:

1. **P0**: Avoid `write_buffer` for large dynamic buffers - use mapped buffers or compute shaders
2. **P1**: Batch dirty buffer syncs to reduce WASM→JS boundary crossings
3. **P2**: Cache PlainData bind groups to reduce JS object allocation
4. **P3**: Move per-frame updates to GPU (compute shader for positions)
