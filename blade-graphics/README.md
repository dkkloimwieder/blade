# Blade Graphics

[![Docs](https://docs.rs/blade-graphics/badge.svg)](https://docs.rs/blade-graphics)
[![Crates.io](https://img.shields.io/crates/v/blade-graphics.svg?maxAge=2592000)](https://crates.io/crates/blade-graphics)

Blade-graphics is a lean and mean [GPU abstraction](https://youtu.be/63dnzjw4azI?t=623) aimed at ergonomics and fun. See [motivation](etc/motivation.md), [FAQ](etc/FAQ.md), and [performance](etc/performance.md) for details.

## Examples

![ray-query example](etc/ray-query.gif)
![particles example](etc/particles.png)

## Platforms

The backend is selected automatically based on the host platform:
- *Vulkan* on desktop Linux, Windows, and Android
- *Metal* on desktop macOS, and iOS
- *OpenGL ES3* on the Web (legacy)
- *WebGPU* on the Web (recommended) - via `--cfg blade_wgpu`

| Feature | Vulkan | Metal | GLES | WebGPU |
| ------- | ------ | ----- | ---- | ------ |
| compute | :white_check_mark: | :white_check_mark: | | :white_check_mark: |
| ray tracing | :white_check_mark: | | | |

### WebGPU (Recommended for Web)

The WebGPU backend provides modern GPU access in browsers with compute shader support:

```bash
RUSTFLAGS="--cfg blade_wgpu" cargo run-wasm --example bunnymark
```

Features:
- Full compute shader support (unlike GLES/WebGL2)
- Indirect draw for GPU-driven rendering
- Modern binding model with bind group caching
- Async initialization for WASM

See [docs/WEBGPU.md](../docs/WEBGPU.md) for technical details.

#### WebGPU Examples

These examples demonstrate WebGPU patterns and run in browsers:

| Example | Pattern | Command |
|---------|---------|---------|
| [webgpu-triangle](examples/webgpu-triangle/) | Minimal setup, vertex colors | `cargo run-wasm --example webgpu-triangle` |
| [webgpu-texture](examples/webgpu-texture/) | Texture creation, sampling, upload | `cargo run-wasm --example webgpu-texture` |
| [webgpu-game-of-life](examples/webgpu-game-of-life/) | Compute ping-pong, storage textures | `cargo run-wasm --example webgpu-game-of-life` |
| [webgpu-post-fx](examples/webgpu-post-fx/) | Render-to-texture, multi-pass | `cargo run-wasm --example webgpu-post-fx` |
| [webgpu-mandelbrot](examples/webgpu-mandelbrot/) | Compute shader visualization | `cargo run-wasm --example webgpu-mandelbrot` |
| [webgpu-sprite-batch](examples/webgpu-sprite-batch/) | Instanced 2D rendering, storage buffer | `cargo run-wasm --example webgpu-sprite-batch` |

All commands require `RUSTFLAGS="--cfg blade_wgpu"` prefix.

#### WASM Event Loop Best Practices

When writing WASM examples with winit, use `ControlFlow::Wait` instead of `ControlFlow::Poll`:

```rust
// ❌ BAD: Hot loop burns 95% of script time on async overhead
target.set_control_flow(winit::event_loop::ControlFlow::Poll);

// ✅ GOOD: Browser throttles to vsync via requestAnimationFrame
target.set_control_flow(winit::event_loop::ControlFlow::Wait);
```

**Why?** On WASM, `Poll` continuously spins the event loop, wasting CPU on scheduler overhead (`cancelAnimationFrame`, `postTask`, `abort`). `Wait` lets the browser handle timing efficiently.

| Pattern | Script Time | CPU Usage |
|---------|-------------|-----------|
| `Poll` + `request_redraw()` | 2400-3900ms | High (hot loop) |
| `Wait` + `request_redraw()` | 100-270ms | Low (browser-throttled) |

Both achieve the same ~120 FPS, but `Wait` uses **8-35x less JavaScript overhead**.

**Platform-specific pattern** (from `examples/particle`):
```rust
// WASM: Use Wait - browser handles timing
#[cfg(target_arch = "wasm32")]
target.set_control_flow(ControlFlow::Wait);

// Native: Use WaitUntil for smooth animation with power efficiency
#[cfg(not(target_arch = "wasm32"))]
target.set_control_flow(ControlFlow::WaitUntil(next_frame_time));
```

See [winit ControlFlow docs](https://docs.rs/winit/latest/winit/event_loop/enum.ControlFlow.html) for details.

### Vulkan

Required instance extensions:
- VK_EXT_debug_utils
- VK_KHR_get_physical_device_properties2
- VK_KHR_get_surface_capabilities2

Required device extensions:
- VK_EXT_inline_uniform_block
- VK_KHR_descriptor_update_template
- VK_KHR_timeline_semaphore
- VK_KHR_dynamic_rendering

Conceptually, Blade requires the baseline Vulkan hardware with a relatively fresh driver.
All of these required extensions are supported in software by the driver on any underlying architecture.

### OpenGL ES (Legacy)

GLES is supported at a basic level but lacks compute shader support. It's enabled for `wasm32-unknown-unknown` target by default, and can also be force-enabled on native:
```bash
RUSTFLAGS="--cfg gles" CARGO_TARGET_DIR=./target-gl cargo run --example bunnymark
```

This path can be activated on all platforms via Angle library.
For example, on macOS it's sufficient to place `libEGL.dylib` and `libGLESv2.dylib` in the working directory.

On Windows, the quotes aren't expected:
```bash
set RUSTFLAGS=--cfg gles
```

### WebGL2 (Legacy)

Without the `blade_wgpu` flag, WASM builds use WebGL2 via GLES:
```bash
cargo run-wasm --example bunnymark
```

Note: WebGL2 lacks compute shader support. Use WebGPU for compute workloads.

### Vulkan Portability

First, ensure to load the environment from the Vulkan SDK:
```bash
cd /opt/VulkanSDK && source setup-env.sh
```

Vulkan backend can be forced on using "vulkan" config flag. Example invocation that produces a vulkan (portability) build into another target folder:
```bash
RUSTFLAGS="--cfg vulkan" CARGO_TARGET_DIR=./target-vk cargo test
```
