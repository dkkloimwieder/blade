# WebGPU Triangle Example

> Minimal "hello world" for blade-graphics WebGPU rendering

This is the simplest possible render example demonstrating core blade-graphics patterns for WebGPU.

## What This Example Demonstrates

| Pattern | Description |
|---------|-------------|
| **Context initialization** | Sync (native) and async (WASM) paths |
| **Surface creation** | Window surface with format detection |
| **Shader loading** | Platform-aware (embedded for WASM, filesystem for native) |
| **Render pipeline** | Minimal pipeline with no data bindings |
| **Render loop** | Frame acquisition, command encoding, submission |
| **Synchronization** | Waiting for previous frame before reuse |
| **Resource cleanup** | Proper destruction order |

## Running

### Browser (WebGPU)

```bash
RUSTFLAGS="--cfg blade_wgpu" cargo run-wasm -p blade-graphics --example webgpu-triangle
```

Then open http://localhost:8000 in a WebGPU-capable browser.

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

### Native (WebGPU backend via wgpu)

```bash
RUSTFLAGS="--cfg blade_wgpu" cargo run -p blade-graphics --example webgpu-triangle
```

## Code Structure

```
webgpu-triangle/
├── main.rs      # Application setup and render loop
└── shader.wgsl  # Vertex and fragment shaders
```

## Key Patterns Explained

### 1. Platform-Aware Initialization

```rust
// Native: synchronous
#[cfg(not(target_arch = "wasm32"))]
fn new(window: &winit::window::Window) -> Self {
    let context = unsafe { gpu::Context::init(desc).unwrap() };
    // ...
}

// WASM: asynchronous (required by browser WebGPU)
#[cfg(target_arch = "wasm32")]
async fn new_async(window: &winit::window::Window) -> Self {
    let context = gpu::Context::init_async(desc).await.unwrap();
    // ...
}
```

### 2. Shader Loading

```rust
// WASM: embed at compile time (no filesystem)
#[cfg(target_arch = "wasm32")]
let shader_source = include_str!("shader.wgsl");

// Native: load from filesystem (enables live editing)
#[cfg(not(target_arch = "wasm32"))]
let shader_source = std::fs::read_to_string("path/to/shader.wgsl").unwrap();
```

### 3. Minimal Render Pipeline

```rust
let pipeline = context.create_render_pipeline(gpu::RenderPipelineDesc {
    name: "triangle",
    data_layouts: &[],           // No uniform/storage bindings
    vertex: shader.at("vs_main"),
    vertex_fetches: &[],         // No vertex buffers (positions in shader)
    fragment: Some(shader.at("fs_main")),
    primitive: gpu::PrimitiveState {
        topology: gpu::PrimitiveTopology::TriangleList,
        ..Default::default()
    },
    depth_stencil: None,         // No depth testing
    color_targets: &[gpu::ColorTargetState {
        format: surface.info().format,
        blend: None,
        write_mask: gpu::ColorWrites::ALL,
    }],
    multisample_state: gpu::MultisampleState::default(),
});
```

### 4. Render Loop

```rust
fn render(&mut self) {
    // Wait for previous frame (prevents buffer conflicts)
    if let Some(ref sp) = self.prev_sync_point {
        self.context.wait_for(sp, !0);
    }

    // Acquire swapchain frame
    let frame = self.surface.acquire_frame();

    // Start recording commands
    self.command_encoder.start();

    // Begin render pass
    if let mut pass = self.command_encoder.render("triangle", gpu::RenderTargetSet {
        colors: &[gpu::RenderTarget {
            view: frame.texture_view(),
            init_op: gpu::InitOp::Clear(gpu::TextureColor::OpaqueBlack),
            finish_op: gpu::FinishOp::Store,
        }],
        depth_stencil: None,
    }) {
        // Bind pipeline and draw
        if let mut encoder = pass.with(&self.pipeline) {
            encoder.draw(0, 3, 0, 1);  // 3 vertices, 1 instance
        }
    }

    // Present and submit
    self.command_encoder.present(frame);
    self.prev_sync_point = Some(self.context.submit(&mut self.command_encoder));
}
```

### 5. WGSL Shader (Hardcoded Vertices)

```wgsl
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Positions defined in shader - no vertex buffer needed
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 0.5),    // top
        vec2<f32>(-0.5, -0.5),  // bottom-left
        vec2<f32>(0.5, -0.5),   // bottom-right
    );
    // ...
}
```

## Expected Output

A colored triangle with:
- Red vertex at top
- Green vertex at bottom-left
- Blue vertex at bottom-right
- Colors interpolated across the face

## Next Steps

After understanding this example, explore:
- **texture** - Adding textures and samplers
- **mini** - Compute shaders (mipmap generation)
- **bunnymark** - Vertex buffers, instancing, compute physics
