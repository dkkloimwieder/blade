# WebGPU Triangle Example

> Interactive "hello world" for blade-graphics WebGPU rendering

This example demonstrates core blade-graphics patterns for WebGPU with interactive color selection.

## What This Example Demonstrates

| Pattern | Description |
|---------|-------------|
| **Context initialization** | Sync (native) and async (WASM) paths |
| **Surface creation** | Window surface with format detection |
| **Shader loading** | Platform-aware (embedded for WASM, filesystem for native) |
| **Uniform bindings** | ShaderData trait implementation for color tint |
| **Render pipeline** | Pipeline with data layouts for uniform binding |
| **Render loop** | Frame acquisition, command encoding, submission |
| **Synchronization** | Waiting for previous frame before reuse |
| **Interactive controls** | Keyboard (0-5) + HTML dropdown (WASM) |
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

### 3. Uniform Bindings (ShaderData Trait)

```rust
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ColorTint {
    rgba: [f32; 4],  // RGB + alpha for blend strength
}

struct TriangleParams {
    color_tint: ColorTint,
}

impl gpu::ShaderData for TriangleParams {
    fn layout() -> gpu::ShaderDataLayout {
        gpu::ShaderDataLayout {
            bindings: vec![("uniforms", gpu::ShaderBinding::Plain { size: 16 })],
        }
    }

    fn fill(&self, mut ctx: gpu::PipelineContext) {
        use gpu::ShaderBindable as _;
        self.color_tint.bind_to(&mut ctx, 0);
    }
}
```

### 4. Render Pipeline with Data Layouts

```rust
let pipeline = context.create_render_pipeline(gpu::RenderPipelineDesc {
    name: "triangle",
    data_layouts: &[&TriangleParams::layout()],  // Uniform binding
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

### 5. Render Loop with Uniform Binding

```rust
fn render(&mut self) {
    // Wait for previous frame (prevents buffer conflicts)
    if let Some(ref sp) = self.prev_sync_point {
        self.context.wait_for(sp, !0);
    }

    let frame = self.surface.acquire_frame();
    self.command_encoder.start();

    // Prepare shader params with current color
    let params = TriangleParams { color_tint: self.current_color };

    if let mut pass = self.command_encoder.render("triangle", gpu::RenderTargetSet {
        colors: &[gpu::RenderTarget {
            view: frame.texture_view(),
            init_op: gpu::InitOp::Clear(gpu::TextureColor::OpaqueBlack),
            finish_op: gpu::FinishOp::Store,
        }],
        depth_stencil: None,
    }) {
        if let mut encoder = pass.with(&self.pipeline) {
            encoder.bind(0, &params);  // Bind uniforms at group 0
            encoder.draw(0, 3, 0, 1);
        }
    }

    self.command_encoder.present(frame);
    self.prev_sync_point = Some(self.context.submit(&mut self.command_encoder));
}
```

### 6. WGSL Shader with Color Tint

```wgsl
struct Uniforms {
    color_tint: vec4<f32>,
}

var<uniform> uniforms: Uniforms;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Mix vertex color with tint based on tint alpha
    let vertex_color = vec4<f32>(input.color, 1.0);
    let tinted = mix(vertex_color.rgb, uniforms.color_tint.rgb, uniforms.color_tint.a);
    return vec4<f32>(tinted, 1.0);
}
```

## Controls

| Key | Color |
|-----|-------|
| 0 | Original (vertex colors) |
| 1 | Red |
| 2 | Green |
| 3 | Blue |
| 4 | Yellow |
| 5 | Purple |

**WASM**: Also has an HTML dropdown in the top-left corner.

## Expected Output

A colored triangle with:
- Red vertex at top, Green at bottom-left, Blue at bottom-right
- Colors interpolated across the face
- Interactive color tinting via keyboard or dropdown

## Next Steps

After understanding this example, explore:
- **webgpu-texture** - Adding textures and samplers
- **webgpu-mandelbrot** - Compute shaders with interactivity
- **bunnymark** - Vertex buffers, instancing, compute physics
