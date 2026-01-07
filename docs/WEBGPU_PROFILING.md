# WebGPU Performance Profiling Guide

> Focus: Firefox on Linux (primary), Chrome (secondary)

---

## Quick Reference

| Tool | Firefox | Chrome | What It Shows |
|------|---------|--------|---------------|
| WebGPU Inspector | ✅ | ✅ | Frame capture, GPU objects, shaders |
| Browser Profiler | ✅ | ✅ | CPU/JS timing, markers |
| WGPU_TRACE | ✅ | ❌ | Low-level wgpu API trace |
| Perfetto gpu.dawn | ❌ | ✅ | Dawn internals |
| RenderDoc | ✅ | ✅ | GPU capture, shader debug |

---

## 1. Firefox Setup (Linux)

### 1.1 Required about:config Flags

Open `about:config` and set:

```
dom.webgpu.enabled = true
gfx.webgpu.ignore-blocklist = true
gfx.webrender.all = true
```

**Restart Firefox after changes** - prefs don't take effect without restart.

### 1.2 Optional Performance Flags

```
layers.gpu-process.enabled = true
media.gpu-process-decoder = true
```

---

## 2. WebGPU Profiling Extensions

### 2.1 WebGPU Inspector (Recommended)

**Best for**: Frame-by-frame debugging, shader inspection, buffer/texture data

**Installation:**
- **Firefox**: [Firefox Add-Ons Store](https://addons.mozilla.org/firefox/addon/webgpu-inspector/)
- **Chrome**: [Chrome Web Store](https://chromewebstore.google.com/detail/webgpu-inspector/holcbbnljhkpkjkhgkagjkhhpeochfal)
- **Source**: [GitHub](https://github.com/brendan-duncan/webgpu_inspector)

| Feature | Description |
|---------|-------------|
| **Inspect** | Live view of all GPU objects (buffers, textures, pipelines) |
| **Capture** | Records one frame: all commands, render pass outputs, buffer data |
| **Record** | Generates standalone HTML replay file for sharing/bug reports |
| **Shader Edit** | Modify shaders live, see results immediately |
| **Frame Timing** | Plot frame times and memory allocation over multiple frames |
| **Pixel Inspector** | Click on textures to see exact pixel values |

**Usage:**
1. Open DevTools (F12)
2. Find "WebGPU Inspector" tab (may need to click ">>" to find it)
3. Click **Inspect** to see live GPU objects
4. Click **Capture** to record a single frame
5. Expand commands to see bound resources, state, outputs

**Troubleshooting:**
- Close and reopen DevTools if tab is missing
- Refresh page if tools don't activate
- Ensure page has focus when using Capture

### 2.2 WebGPU DevTools

**Chrome**: [Chrome Web Store](https://chromewebstore.google.com/detail/webgpu-devtools/ckabpgjkjmbkfmichbbgcgbelkbbpopi)

Alternative extension with similar capabilities. Useful if WebGPU Inspector has issues.

### 2.3 WebGPUVision (Advanced)

**Source**: [GitHub](https://github.com/WonderInteractive/WebGPUVision)

A more advanced tool with unique features:
- **Live remote debugging** across networks
- **AI-assisted call graph optimization** (experimental)
- **Multi-frame recording** with compression
- **Real-time shader editing**

Currently Windows-only, Linux/macOS support planned. Pre-release software.

### 2.4 What Extensions Show

- Every GPU command in order
- Render pass output images (what was drawn)
- Buffer contents at each draw call
- Texture data with pixel inspection
- Pipeline state (blend, depth, etc.)
- Memory allocation tracking

---

## 3. WGPU API Tracing (Firefox)

**Best for**: Low-level debugging, reproducing bugs, comparing to native

### 3.1 Setup (One-Time)

Create a dedicated Firefox profile with WebGPU enabled:

```bash
# Create profile directory
mkdir -p ~/firefox-wgputrace-profile

# Launch Firefox profile manager
firefox -ProfileManager --no-remote
```

1. Click **Create Profile**
2. Name it (e.g., `wgputrace`)
3. Set directory to `~/firefox-wgputrace-profile`
4. Finish and launch that profile
5. Go to `about:config`, set `dom.webgpu.enabled` = `true`
6. Close Firefox

### 3.2 Capture Trace

```bash
# Create trace output directory
mkdir -p ~/wgpu-trace

# Launch Firefox with tracing (use YOUR profile path)
MOZ_DISABLE_GPU_SANDBOX=1 WGPU_TRACE=~/wgpu-trace firefox --profile ~/firefox-wgputrace-profile --no-remote http://localhost:8000
```

Navigate to your WebGPU page. The trace records automatically.

**Important**:
- Trace directory must be in home dir (not /tmp) - GPU process sandbox restrictions
- WebGPU must be enabled in that profile
- Works with Firefox stable (tested on 146), not just Nightly

### 3.3 Trace Contents

```
~/wgpu-trace/0/
├── trace.ron           # API call sequence (can be 100K+ lines)
├── data*.bin           # Buffer data snapshots
└── data*.wgsl          # Shader source
```

### 3.4 Replay Trace

Clone wgpu and build the player:

```bash
git clone https://github.com/gfx-rs/wgpu
cd wgpu/player

# For RenderDoc capture (no window):
cargo run -- /tmp/wgpu-trace

# For visual replay (with window):
cargo run --features winit -- /tmp/wgpu-trace
```

### 3.5 If Trace Is Incomplete

If Firefox crashed mid-trace, the file may be truncated:

```bash
# Add closing bracket if missing
echo "]" >> ~/wgpu-trace/0/trace.ron
```

### 3.6 Analyzing trace.ron

The trace file is RON (Rusty Object Notation) format containing every wgpu API call:

```ron
[
    CreateBuffer(Id(0,1,Empty), BufferDescriptor { label: Some("vertex"), size: 65536, usage: VERTEX | COPY_DST, ... }),
    CreateShaderModule(Id(0,1,Empty), ShaderModuleDescriptor { label: None, source: File("data0.wgsl"), ... }),
    CreateRenderPipeline(Id(0,1,Empty), RenderPipelineDescriptor { ... }),
    Submit(1, [
        RunRenderPass { base: BasePass { commands: [...] }, target_colors: [...] }
    ]),
    ...
]
```

**Key things to look for:**

| Pattern | What It Means |
|---------|---------------|
| Many `CreateBuffer` calls per frame | Resource churn - consider caching |
| Large `WriteBuffer` calls | Data uploads - minimize per-frame |
| Multiple `Submit` calls per frame | Consider batching commands |
| Complex `RenderPipeline` descriptors | Shader complexity analysis |

**Quick analysis commands:**

```bash
# Count API calls by type
grep -oE "^    [A-Za-z]+" ~/wgpu-trace/0/trace.ron | sort | uniq -c | sort -rn

# Find all buffer creations
grep "CreateBuffer" ~/wgpu-trace/0/trace.ron | head -20

# Count submits (frames)
grep -c "Submit" ~/wgpu-trace/0/trace.ron

# Find render pass configurations
grep -A5 "RunRenderPass" ~/wgpu-trace/0/trace.ron | head -50
```

**Shader analysis:**

Shaders are saved as `data*.wgsl` files. Review for:
- Unused uniforms
- Complex math that could be simplified
- Loop unrolling opportunities

---

## 4. Firefox Profiler

**Best for**: JavaScript/CPU profiling, finding where time is spent in your app

### 4.1 Enable

1. Go to [profiler.firefox.com](https://profiler.firefox.com/)
2. Click **Enable Profiler** (installs extension)
3. Or: Press **Ctrl+Shift+1** to toggle

### 4.2 Capture

1. Click the profiler icon in toolbar
2. Select **Graphics** preset (for GPU work)
3. Click **Start Recording**
4. Interact with your WebGPU app
5. Click **Capture**

### 4.3 Analyze

- **Timeline**: See all threads, find gaps/stalls
- **Marker Chart**: CSS animations, DOM events, GPU markers
- **Call Tree**: Where CPU time is spent
- **Flame Graph**: Visual call stack

### 4.4 GPU Markers

Firefox adds markers for graphics operations. Look for:
- `CompositorBridgeParent` events
- `WebRender` operations
- Frame boundaries

---

## 5. RenderDoc (Advanced)

**Best for**: Shader debugging, draw call inspection, GPU state

### 5.1 Firefox Integration

Since Firefox renders WebGPU to textures (not swap chain), special setup needed:

```bash
# Wayland workaround (if using Wayland)
export WAYLAND_DISPLAY=""

# Or force X11
export WINIT_UNIX_BACKEND=x11
```

### 5.2 Capture from wgpu Trace

1. Capture WGPU_TRACE as above
2. Replay WITHOUT winit (adds RenderDoc markers automatically):

```bash
cd wgpu/player
cargo run -- /tmp/wgpu-trace
```

RenderDoc sees the begin/end frame markers.

### 5.3 Capture from Running Firefox

More complex - requires building wgpu player and using its trace replay.

---

## 6. Chrome Setup (Secondary)

### 6.1 Required Flags for Linux

**Command line** (most reliable for performance):

```bash
google-chrome \
  --enable-unsafe-webgpu \
  --enable-features=Vulkan,VulkanFromANGLE \
  --use-angle=vulkan \
  --enable-dawn-features=allow_unsafe_apis
```

**Key flags explained:**

| Flag | Purpose |
|------|---------|
| `--enable-unsafe-webgpu` | Enable WebGPU API |
| `--enable-features=Vulkan,VulkanFromANGLE` | Use Vulkan backend (faster than OpenGL) |
| `--use-angle=vulkan` | Force ANGLE to use Vulkan |
| `--enable-dawn-features=allow_unsafe_apis` | Enable timestamp queries |

**Performance note**: The Vulkan flags significantly improve performance on Linux. Without them, Chrome may fall back to OpenGL which is slower.

Or in `chrome://flags`:
- `#enable-unsafe-webgpu` → Enabled
- `#enable-vulkan` → Enabled

### 6.2 Perfetto Tracing

1. Go to [ui.perfetto.dev](https://ui.perfetto.dev/)
2. Click **Record new trace**
3. Select **Chrome** as target
4. Under categories, enable **gpu.dawn**
5. Click **Start Recording**
6. Switch to your WebGPU tab
7. Stop and analyze

**What gpu.dawn shows**:
- Dawn command encoding
- Buffer/texture uploads
- GPU process communication
- Frame timing from browser perspective

### 6.3 Chrome WebGPU Developer Features

Enable at `chrome://flags/#enable-webgpu-developer-features` for advanced profiling.

**Features unlocked:**

| Feature | Description |
|---------|-------------|
| **High-precision timestamps** | Nanosecond GPU timing (removes 100μs quantization) |
| **Extended GPUAdapterInfo** | `backend`, `type`, `driver`, `vkDriverVersion`, `memoryHeaps` |
| **strictMath shaders** | Precise math without NaN/Infinity optimizations |
| **Zero-copy video info** | Check if video textures use direct GPU access |

**Get adapter details:**
```javascript
const adapter = await navigator.gpu.requestAdapter();
console.log(adapter.info.backend);      // "vulkan", "d3d12", etc.
console.log(adapter.info.type);         // "discrete GPU", "integrated GPU"
console.log(adapter.info.memoryHeaps);  // Memory heap sizes
```

**Warning:** Development only - exposes privacy-sensitive info.

### 6.4 Chrome DevTools Performance

1. Open DevTools (F12)
2. Go to **Performance** tab
3. Click record, interact, stop
4. Look at flame graph for JavaScript
5. Note: GPU work is async, may not show directly

---

## 7. WGPU Trace Analysis with RenderDoc

### 7.1 Build the wgpu Player

```bash
git clone https://github.com/gfx-rs/wgpu
cd wgpu/player

# For RenderDoc capture (recommended):
cargo build --release

# For visual replay with window:
cargo build --release --features winit
```

### 7.2 Replay in RenderDoc

1. **Open RenderDoc**
2. **Launch Application** tab:
   - Executable: `wgpu/target/release/player`
   - Arguments: `~/wgpu-trace/0`
   - Working Directory: `wgpu/player`
3. Click **Launch**
4. Press **F12** to capture a frame
5. Double-click the capture thumbnail to analyze

**Important:** Build WITHOUT `winit` feature for RenderDoc - it adds frame markers automatically.

### 7.3 Wayland Workaround

RenderDoc may fail on Wayland. Force X11:

```bash
export WAYLAND_DISPLAY=""
export WINIT_UNIX_BACKEND=x11
```

### 7.4 What RenderDoc Shows

- Draw call list with timing
- Shader source and disassembly
- Buffer/texture contents at each draw
- Pipeline state (blend, depth, rasterizer)
- GPU counters (vendor-specific)

---

## 8. Application Code Changes

### 8.1 No Code Changes Needed

All profiling methods work without modifying your blade/wgpu code:
- WebGPU Inspector hooks the API
- WGPU_TRACE is runtime env var
- Firefox Profiler is external

### 8.2 Optional: Add Debug Labels

For better trace readability, use descriptive pass labels:

```rust
// In blade - labels appear in traces
encoder.render("shadow-pass", targets)
encoder.compute("particle-update")
```

These labels appear in WebGPU Inspector and WGPU traces.

### 8.3 Future: Timestamp Queries

When `blade-9rv` is implemented, you'll be able to get per-pass GPU timing from within the app. Until then, use external tools.

---

## 8. Debugging Workflow

### 8.1 Performance Issue

1. **WebGPU Inspector** → Capture frame → Check command count
2. **Firefox Profiler** → Graphics preset → Find JavaScript bottlenecks
3. **WGPU_TRACE** → Replay in isolation → Compare to native

### 8.2 Visual Bug

1. **WebGPU Inspector** → Capture frame → Inspect render pass outputs
2. Check buffer contents at each draw call
3. Edit shaders live to test fixes

### 8.3 Crash

1. **WGPU_TRACE** → Capture trace before crash
2. Replay to reproduce outside browser
3. Check driver versions, update Mesa

---

## 9. Common Issues

### 9.1 Firefox WebGPU Not Working

```
# Check GPU info
about:support → Graphics section

# Reset prefs if broken
about:config → Reset dom.webgpu.*
```

### 9.2 Blank Canvas on Wayland

Set in about:config:
```
dom.webgpu.allow-present-without-readback = false
```

### 9.3 Poor Performance

Check:
- Driver version (`glxinfo | grep "OpenGL version"`)
- GPU process enabled (`about:support`)
- WebRender active (should say "WebRender" not "Basic")

### 9.4 Random Crashes

Update drivers:
```bash
# Mesa (AMD/Intel)
sudo apt update && sudo apt upgrade mesa-vulkan-drivers

# NVIDIA
# Use latest proprietary driver
```

---

## 10. Summary: Recommended Tools

| What You Want | Tool | Setup |
|---------------|------|-------|
| See GPU commands | WebGPU Inspector | Install extension |
| Debug shaders | WebGPU Inspector | Capture → Edit |
| Find JS bottlenecks | Firefox Profiler | Graphics preset |
| Low-level trace | WGPU_TRACE | Env var + sandbox disable |
| GPU timing | External tools | Until blade-9rv done |

**Recommended workflow**:
1. Install WebGPU Inspector extension
2. Capture a frame
3. Look at command sequence and timing
4. If deeper analysis needed, use WGPU_TRACE

---

## 11. Running Blade Bunnymark (WASM Demo)

### 11.1 Build and Serve

```bash
# Build with WebGPU backend (required for WASM)
RUSTFLAGS="--cfg blade_wgpu" cargo run-wasm --example bunnymark

# Server starts at http://localhost:8000
```

**Important:** The `blade_wgpu` flag is required. Without it, the GLES backend is used which lacks required features.

### 11.2 Modify Bunny Count

Edit `examples/bunnymark/main.rs` line ~695:

```rust
// Pre-populate with bunnies
for _ in 0..11 {  // ~10,000 bunnies (exponential growth)
    ex.increase();
}
```

| Iterations | Approximate Bunnies |
|------------|---------------------|
| 6 | ~1,300 |
| 9 | ~5,000 |
| 11 | ~11,000 |
| 13 | ~25,000 |

Each `increase()` call adds `64 + current_count/2` bunnies.

### 11.3 Automated Chrome Profiling (RECOMMENDED)

**Use the chrome-devtools-cli perf.mjs script with `--gpu` flag:**

```bash
cd ~/.claude/skills/chrome-devtools-cli
node scripts/perf.mjs http://localhost:8000 --gpu --duration 5000
```

**CRITICAL:** The `--gpu` flag is REQUIRED for WebGPU profiling. It automatically enables:
- `--enable-unsafe-webgpu`
- `--enable-features=Vulkan,VulkanFromANGLE`
- `--use-angle=vulkan`
- `--enable-dawn-features=allow_unsafe_apis`

Without `--gpu`, Chrome won't have WebGPU enabled and profiling will fail.

**Options:**
- `--duration <ms>` - Profile duration (default: 5000)
- `--iterations <n>` - Run benchmark N times with statistics
- `--click "<selector>"` - Click element before profiling
- `--output <file>` - Save JSON results

**Examples:**

```bash
# Profile for 10 seconds
node scripts/perf.mjs http://localhost:8000 --gpu --duration 10000

# Run 5 iterations for benchmarking
node scripts/perf.mjs http://localhost:8000 --gpu --iterations 5

# Save results to file
node scripts/perf.mjs http://localhost:8000 --gpu --output /tmp/bunnymark-perf.json
```

### 11.4 Manual Chrome Launch (Alternative)

**With extensions enabled** (for WebGPU Inspector):

```bash
google-chrome \
  --enable-unsafe-webgpu \
  --enable-features=Vulkan,VulkanFromANGLE \
  --use-angle=vulkan \
  --enable-dawn-features=allow_unsafe_apis \
  --user-data-dir=/tmp/chrome-webgpu-profile \
  http://localhost:8000
```

**Key flags:**
- `--user-data-dir=/tmp/...` - Isolated profile (won't affect your main Chrome)
- Extensions enabled by default (no `--disable-extensions`)
- Vulkan backend for best performance on Linux

### 11.5 Capture with WebGPU Inspector

1. Install WebGPU Inspector from Chrome Web Store
2. Navigate to `http://localhost:8000`
3. Open DevTools (F12)
4. Find **WebGPU Inspector** tab (click `>>` if hidden)
5. Click **Capture** for single frame, or **Record** for multi-frame HTML export

### 11.6 Export Recording

1. Click **Record** (not Capture)
2. Let the demo run for a few seconds
3. Click **Stop**
4. Save the generated HTML file

The HTML file is self-contained and can be opened in any browser to replay the captured frames.

### 11.7 Launch Firefox for Profiling

```bash
# Standard launch (WebGPU must be enabled in about:config)
firefox http://localhost:8000

# With WGPU API tracing
MOZ_DISABLE_GPU_SANDBOX=1 WGPU_TRACE=~/wgpu-trace \
  firefox --profile ~/firefox-wgputrace-profile --no-remote \
  http://localhost:8000
```

---

## 12. Blade Bunnymark Analysis

See `bench-trace/webgpu-inspector-analysis.md` for detailed analysis of captured frames.

**Key findings from captures (1K to 11K bunnies):**

| Metric | Scales With Instance Count? |
|--------|---------------------------|
| Draw calls | No (always 1) |
| Compute dispatches | No (always 1) |
| Workgroups | Yes (linear) |
| Pipeline binds | No (always 2) |
| setBindGroup | No (always 4) |
| File size | No (~4.7 MB) |

The demo demonstrates efficient instanced rendering - 8× more instances requires only changes to dispatch/draw parameters, not command structure.

---

## References

- [wgpu Debugging Wiki](https://github.com/gfx-rs/wgpu/wiki/Debugging-wgpu-Applications)
- [WebGPU Inspector](https://github.com/brendan-duncan/webgpu_inspector)
- [Firefox Profiler](https://profiler.firefox.com/)
- [Chrome WebGPU Troubleshooting](https://developer.chrome.com/docs/web-platform/webgpu/troubleshooting-tips)
- [Perfetto](https://ui.perfetto.dev/)
