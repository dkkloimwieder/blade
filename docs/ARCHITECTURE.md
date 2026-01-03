# Blade Graphics Library - Architecture Summary

> **Sharp and simple graphics library** - A lean, multi-layered rendering solution for Rust

---

## 1. Executive Overview

**Blade** is a Rust graphics library that provides a complete, layered rendering stack. It starts with low-level GPU abstractions and scales up to a full-featured engine with ray-tracing and physics support.

| Attribute | Details |
|-----------|---------|
| Repository | https://github.com/kvark/blade |
| Version | 0.3.0 (root), 0.7.0 (blade-graphics) |
| License | MIT |
| Rust Edition | 2021 (MSRV 1.65) |
| Author | Dzmitry Malyshau (kvark) |

### Supported Platforms

| Backend | Platforms | Features |
|---------|-----------|----------|
| **Vulkan** | Linux, Windows, Android, FreeBSD | Full features including ray tracing |
| **Metal** | macOS, iOS | Core graphics features |
| **OpenGL ES 3** | WebAssembly, fallback | Basic graphics features |

### Key Design Principles

1. **Layered Architecture**: Use only the abstraction level you need
2. **Ergonomic API**: Focus on developer experience with clean Rust idioms
3. **Modern GPU Features**: Dynamic rendering, timeline semaphores, ray tracing
4. **Cross-Platform**: Single API compiles to multiple backends

---

## 2. Crate Architecture

```
blade (v0.3.0) - Full engine with physics
├── blade-graphics (v0.7.0) - Low-level GPU abstraction
├── blade-render (v0.4.0) - Ray-traced renderer [Vulkan only]
├── blade-asset (v0.2.0) - Asset pipeline framework
├── blade-egui (v0.6.0) - egui UI integration
├── blade-helpers (v0.1.0) - Camera, HUD utilities
├── blade-macros (v0.3.0) - Derive macros
└── blade-util (v0.3.0) - Utility functions
```

### Crate Descriptions

| Crate | Path | Purpose |
|-------|------|---------|
| `blade-graphics` | `blade-graphics/` | Core GPU abstraction with Vulkan/Metal/GLES backends |
| `blade-render` | `blade-render/` | Ray-traced renderer with ReSTIR, denoising |
| `blade-asset` | `blade-asset/` | Task-parallel asset loading (GLTF, textures) |
| `blade-egui` | `blade-egui/` | Integration layer for egui immediate-mode UI |
| `blade-helpers` | `blade-helpers/` | High-level utilities (camera controls, HUD) |
| `blade-macros` | `blade-macros/` | `#[derive(ShaderData)]`, `#[derive(Vertex)]` macros |
| `blade-util` | `blade-util/` | General-purpose utility functions |
| `blade` (root) | `src/` | Complete engine with Rapier3D physics |

### Dependency Graph

```
blade
├── blade-graphics ──────────────────────────────────┐
├── blade-render ─────┬── blade-graphics             │
│                     └── blade-asset                │
├── blade-asset ──────────────────────────────────── ├── naga (shader compiler)
├── blade-egui ───────┬── blade-graphics             │
│                     └── egui                       │
├── blade-helpers ────┬── blade-graphics             │
│                     └── rapier3d                   │
└── blade-macros ─────────────────────────────────── ├── syn, quote (proc-macro)
```

---

## 3. Rendering Pipeline Deep-Dive

### 3.1 Backend Abstraction

**File**: `blade-graphics/src/lib.rs:51-69`

The backend is selected at compile time via conditional compilation:

```rust
#[cfg_attr(all(not(vulkan), not(gles), any(target_os = "ios", target_os = "macos")),
    path = "metal/mod.rs")]
#[cfg_attr(all(not(gles), any(vulkan, windows, target_os = "linux", ...)),
    path = "vulkan/mod.rs")]
#[cfg_attr(any(gles, target_arch = "wasm32"), path = "gles/mod.rs")]
mod hal;
```

### 3.2 Context Initialization

**File**: `blade-graphics/src/vulkan/mod.rs:151-163`

```rust
pub struct Context {
    memory: Mutex<MemoryManager>,        // GPU memory allocator
    device: Device,                       // Vulkan device wrapper
    queue_family_index: u32,              // Graphics queue family
    queue: Mutex<Queue>,                  // Submission queue with timeline semaphore
    physical_device: vk::PhysicalDevice,  // GPU handle
    naga_flags: naga::back::spv::WriterFlags, // Shader compilation flags
    instance: Instance,                   // Vulkan instance
    entry: ash::Entry,                    // Vulkan loader
}
```

**Usage**:
```rust
let context = unsafe {
    gpu::Context::init(gpu::ContextDesc {
        presentation: true,
        validation: true,
        timing: true,
        ..Default::default()
    }).unwrap()
};
```

### 3.3 Pipeline Types

#### Compute Pipeline

**File**: `blade-graphics/src/lib.rs:727-731`

```rust
pub struct ComputePipelineDesc<'a> {
    pub name: &'a str,
    pub data_layouts: &'a [&'a ShaderDataLayout],
    pub compute: ShaderFunction<'a>,
}
```

#### Render Pipeline

**File**: `blade-graphics/src/lib.rs:1062-1072`

```rust
pub struct RenderPipelineDesc<'a> {
    pub name: &'a str,
    pub data_layouts: &'a [&'a ShaderDataLayout],
    pub vertex: ShaderFunction<'a>,
    pub vertex_fetches: &'a [VertexFetchState<'a>],
    pub primitive: PrimitiveState,
    pub depth_stencil: Option<DepthStencilState>,
    pub fragment: Option<ShaderFunction<'a>>,
    pub color_targets: &'a [ColorTargetState],
    pub multisample_state: MultisampleState,
}
```

### 3.4 Command Encoding

**File**: `blade-graphics/src/vulkan/command.rs`

The command encoding follows a hierarchical pattern:

```
CommandEncoder
├── TransferCommandEncoder  (copy operations)
├── ComputeCommandEncoder   (compute dispatches)
│   └── PipelineEncoder     (bound pipeline state)
├── RenderCommandEncoder    (rendering passes)
│   └── PipelineEncoder     (bound pipeline state)
└── AccelerationStructureCommandEncoder (ray tracing builds)
```

**Example Flow** (from `examples/mini/main.rs:117-170`):

```rust
// 1. Create encoder with buffer count for pipelining
let mut command_encoder = context.create_command_encoder(gpu::CommandEncoderDesc {
    name: "main",
    buffer_count: 1,
});

// 2. Start recording
command_encoder.start();

// 3. Transfer pass
if let mut transfer = command_encoder.transfer("copy-data") {
    transfer.copy_buffer_to_texture(src, stride, dst, extent);
}

// 4. Compute pass
if let mut compute = command_encoder.compute("process") {
    if let mut pc = compute.with(&pipeline) {
        pc.bind(0, &shader_data);
        pc.dispatch(groups);
    }
}

// 5. Submit and synchronize
let sync_point = context.submit(&mut command_encoder);
context.wait_for(&sync_point, 1000);
```

### 3.5 Render Pass Structure

**File**: `blade-graphics/src/lib.rs:1091-1119`

```rust
pub enum InitOp {
    Load,                    // Preserve existing content
    Clear(TextureColor),     // Clear to specified color
    DontCare,                // Content undefined
}

pub enum FinishOp {
    Store,                   // Store results
    Discard,                 // Discard results
    ResolveTo(TextureView),  // MSAA resolve
    Ignore,                  // No-op
}

pub struct RenderTarget {
    pub view: TextureView,
    pub init_op: InitOp,
    pub finish_op: FinishOp,
}

pub struct RenderTargetSet<'a> {
    pub colors: &'a [RenderTarget],
    pub depth_stencil: Option<RenderTarget>,
}
```

### 3.6 Shader System

**File**: `blade-graphics/src/shader.rs`

#### Shader Compilation Pipeline

```
WGSL Source
    │
    ▼
naga::front::wgsl::parse_str()  ─── Parse WGSL
    │
    ▼
naga::valid::Validator          ─── Validate module
    │
    ▼
Shader { module, info, source } ─── Stored IR
    │
    ▼
Backend Compilation:
├── Vulkan: naga::back::spv    ─── SPIR-V
├── Metal:  naga::back::msl    ─── MSL
└── GLES:   naga::back::glsl   ─── GLSL ES
```

#### Shader Data Binding

**File**: `blade-graphics/src/lib.rs:637-678`

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShaderBinding {
    Texture,
    TextureArray { count: u32 },
    Sampler,
    Buffer,
    BufferArray { count: u32 },
    AccelerationStructure,
    Plain { size: u32 },  // Up to 256 bytes inline uniform
}

pub trait ShaderData {
    fn layout() -> ShaderDataLayout;
    fn fill(&self, context: PipelineContext);
}
```

**Manual Implementation** (from `examples/mini/main.rs:15-33`):

```rust
impl gpu::ShaderData for Globals {
    fn layout() -> gpu::ShaderDataLayout {
        gpu::ShaderDataLayout {
            bindings: vec![
                ("modulator", gpu::ShaderBinding::Plain { size: 16 }),
                ("demodulator", gpu::ShaderBinding::Buffer),
                ("input", gpu::ShaderBinding::Texture),
                ("output", gpu::ShaderBinding::Texture),
            ],
        }
    }
    fn fill(&self, mut ctx: gpu::PipelineContext) {
        use gpu::ShaderBindable as _;
        self.modulator.bind_to(&mut ctx, 0);
        self.demodulator.bind_to(&mut ctx, 1);
        self.input.bind_to(&mut ctx, 2);
        self.output.bind_to(&mut ctx, 3);
    }
}
```

### 3.7 Synchronization

**File**: `blade-graphics/src/vulkan/mod.rs:77-81`

```rust
struct Queue {
    raw: vk::Queue,
    timeline_semaphore: vk::Semaphore,  // VK_KHR_timeline_semaphore
    last_progress: u64,                  // Monotonic progress counter
}
```

**SyncPoint Pattern**:

```rust
// Submit returns a sync point
let sync_point = context.submit(&mut command_encoder);

// Wait for completion (returns false on timeout)
let completed = context.wait_for(&sync_point, timeout_ms);
```

**Frame Synchronization**:

```rust
// Per-frame semaphores for swapchain
struct InternalFrame {
    acquire_semaphore: vk::Semaphore,  // Image acquisition
    present_semaphore: vk::Semaphore,  // Ready for present
    image: vk::Image,
    view: vk::ImageView,
}
```

---

## 4. Resource Management

### 4.1 Memory Types

**File**: `blade-graphics/src/lib.rs:157-198`

```rust
pub enum Memory {
    Device,                          // GPU-local, fast for GPU
    Shared,                          // CPU-GPU accessible
    Upload,                          // CPU write, GPU read only
    External(ExternalMemorySource),  // Shared with other processes
}

pub enum ExternalMemorySource {
    #[cfg(target_os = "windows")]
    Win32(Option<isize>),
    #[cfg(not(target_os = "windows"))]
    Fd(Option<i32>),
    #[cfg(target_os = "linux")]
    Dma(Option<i32>),
    HostAllocation(usize),
}
```

### 4.2 Buffer Resource

**File**: `blade-graphics/src/lib.rs:200-234`

```rust
pub struct BufferDesc<'a> {
    pub name: &'a str,
    pub size: u64,
    pub memory: Memory,
}

pub struct BufferPiece {
    pub buffer: Buffer,
    pub offset: u64,
}
```

### 4.3 Texture Resources

**File**: `blade-graphics/src/lib.rs:302-461`

```rust
pub enum TextureFormat {
    // Color formats
    R8Unorm, Rg8Unorm, Rgba8Unorm, Rgba8UnormSrgb,
    R16Float, Rg16Float, Rgba16Float,
    R32Float, Rg32Float, Rgba32Float,
    // Depth/Stencil
    Depth32Float, Depth32FloatStencil8Uint,
    // Compressed (BC1-BC7)
    Bc1Unorm, Bc1UnormSrgb, /* ... */ Bc7UnormSrgb,
    // Packed
    Rgb10a2Unorm, Rg11b10Ufloat, Rgb9e5Ufloat,
}

pub enum TextureDimension { D1, D2, D3 }
pub enum ViewDimension { D1, D1Array, D2, D2Array, Cube, CubeArray, D3 }

pub struct TextureDesc<'a> {
    pub name: &'a str,
    pub format: TextureFormat,
    pub size: Extent,
    pub array_layer_count: u32,
    pub mip_level_count: u32,
    pub sample_count: u32,
    pub dimension: TextureDimension,
    pub usage: TextureUsage,
    pub external: Option<ExternalMemorySource>,
}
```

### 4.4 Sampler Configuration

**File**: `blade-graphics/src/lib.rs:472-541`

```rust
pub enum AddressMode {
    ClampToEdge, Repeat, MirrorRepeat, ClampToBorder,
}

pub enum FilterMode { Nearest, Linear }

pub struct SamplerDesc<'a> {
    pub name: &'a str,
    pub address_modes: [AddressMode; 3],
    pub mag_filter: FilterMode,
    pub min_filter: FilterMode,
    pub mipmap_filter: FilterMode,
    pub lod_min_clamp: f32,
    pub lod_max_clamp: Option<f32>,
    pub compare: Option<CompareFunction>,
    pub anisotropy_clamp: u32,
    pub border_color: Option<TextureColor>,
}
```

### 4.5 Resource Arrays (Bindless)

**File**: `blade-graphics/src/lib.rs:236-281`

```rust
pub type ResourceIndex = u32;

pub struct ResourceArray<T, const N: ResourceIndex> {
    data: Vec<T>,
    free_list: Vec<ResourceIndex>,
}

pub type BufferArray<const N: ResourceIndex> = ResourceArray<BufferPiece, N>;
pub type TextureArray<const N: ResourceIndex> = ResourceArray<TextureView, N>;

// Usage
let mut textures: TextureArray<1000> = TextureArray::new();
let index = textures.alloc(my_texture_view);
// Bind entire array to shader
encoder.bind(0, &textures);
```

### 4.6 Acceleration Structures (Ray Tracing)

**File**: `blade-graphics/src/lib.rs:543-612`

```rust
pub enum AccelerationStructureType { TopLevel, BottomLevel }

pub struct AccelerationStructureMesh {
    pub vertex_data: BufferPiece,
    pub vertex_format: VertexFormat,
    pub vertex_stride: u32,
    pub vertex_count: u32,
    pub index_data: BufferPiece,
    pub index_type: Option<IndexType>,
    pub triangle_count: u32,
    pub transform_data: BufferPiece,
    pub is_opaque: bool,
}

pub struct AccelerationStructureInstance {
    pub acceleration_structure_index: u32,
    pub transform: Transform,
    pub mask: u32,
    pub custom_index: u32,
}
```

---

## 5. High-Level Features

### 5.1 Ray-Traced Renderer (blade-render)

**File**: `blade-render/src/render/mod.rs`

Features:
- Hardware ray tracing with VK_KHR_ray_query
- ReSTIR (Reservoir Spatio-Temporal Importance Resampling)
- Temporal denoising
- Environment importance sampling

**Debug Modes**:
```rust
pub enum DebugMode {
    Final, Depth, DiffuseAlbedo, NormalAlbedo,
    GeometryNormals, ShadingNormals, Motion,
    HitConsistency, SampleReuse, Variance, /* ... */
}
```

### 5.2 Physics Integration (blade + blade-helpers)

**File**: `src/lib.rs`

Powered by **Rapier3D** physics engine:

```rust
pub enum DynamicInput {
    Empty,       // Static object
    SetPosition, // Kinematic (position-based)
    SetVelocity, // Kinematic (velocity-based)
    Full,        // Fully dynamic
}

pub struct JointDesc {
    pub parent_anchor: Transform,
    pub child_anchor: Transform,
    pub linear: mint::Vector3<Option<FreedomAxis>>,
    pub angular: mint::Vector3<Option<FreedomAxis>>,
    pub allow_contacts: bool,
    pub is_hard: bool,  // Hard vs soft constraint
}
```

### 5.3 UI Integration (blade-egui)

**File**: `blade-egui/src/lib.rs`

Integrates egui (v0.32) immediate-mode UI:
- Multi-platform support (Vulkan, Metal, WebGL2)
- Screen descriptor management
- Texture upload and management

### 5.4 Asset Pipeline (blade-asset)

**File**: `blade-asset/src/lib.rs`

Task-parallel asset loading:
- Built on **Choir** task scheduler
- GLTF/GLB model loading with `gltf` crate
- Tangent generation
- Async texture loading

---

## 6. API Quick Reference

### 6.1 Core Types

| Type | File | Purpose |
|------|------|---------|
| `Context` | `vulkan/mod.rs:151` | GPU context, device management |
| `CommandEncoder` | `vulkan/command.rs` | Command recording |
| `ComputePipeline` | `vulkan/pipeline.rs` | Compute shader pipeline |
| `RenderPipeline` | `vulkan/pipeline.rs` | Graphics rendering pipeline |
| `Buffer` | `vulkan/mod.rs:166` | GPU buffer |
| `Texture` | `vulkan/mod.rs:193` | GPU texture |
| `TextureView` | `vulkan/mod.rs` | Texture view for binding |
| `Sampler` | `vulkan/mod.rs` | Texture sampler |
| `Shader` | `lib.rs:613` | Compiled shader module |
| `Surface` | `vulkan/surface.rs` | Window surface for presentation |
| `Frame` | `vulkan/mod.rs:117` | Swapchain frame |

### 6.2 Key Traits

| Trait | File | Purpose |
|-------|------|---------|
| `ShaderData` | `lib.rs:675` | Shader uniform/resource binding layout |
| `ShaderBindable` | `lib.rs:648` | Individual resource binding |
| `Vertex` | `lib.rs:697` | Vertex attribute layout |

### 6.3 Limits

**File**: `blade-graphics/src/lib.rs:73-84`

```rust
pub mod limits {
    pub const PASS_COUNT: usize = 100;              // Max passes per encoder
    pub const PLAIN_DATA_SIZE: u32 = 256;           // Max inline uniform bytes
    pub const RESOURCES_IN_GROUP: u32 = 8;          // Max bindings per group
    pub const STORAGE_BUFFER_ALIGNMENT: u64 = 16;   // Storage buffer alignment
    pub const ACCELERATION_STRUCTURE_SCRATCH_ALIGNMENT: u64 = 256;
}
```

---

## 7. Example Usage Patterns

### Minimal Compute Example

```rust
use blade_graphics as gpu;

// 1. Initialize context
let context = unsafe { gpu::Context::init(gpu::ContextDesc::default()).unwrap() };

// 2. Load shader
let shader = context.create_shader(gpu::ShaderDesc {
    source: include_str!("shader.wgsl"),
});

// 3. Create pipeline
let pipeline = context.create_compute_pipeline(gpu::ComputePipelineDesc {
    name: "my-compute",
    data_layouts: &[&MyData::layout()],
    compute: shader.at("main"),
});

// 4. Create resources
let buffer = context.create_buffer(gpu::BufferDesc {
    name: "data",
    size: 1024,
    memory: gpu::Memory::Shared,
});

// 5. Record and submit commands
let mut encoder = context.create_command_encoder(gpu::CommandEncoderDesc {
    name: "main", buffer_count: 1,
});
encoder.start();

if let mut compute = encoder.compute("run") {
    if let mut pc = compute.with(&pipeline) {
        pc.bind(0, &my_data);
        pc.dispatch([8, 8, 1]);
    }
}

let sync = context.submit(&mut encoder);
context.wait_for(&sync, !0);

// 6. Cleanup
context.destroy_buffer(buffer);
context.destroy_command_encoder(&mut encoder);
```

### Render Pass Example

```rust
// Acquire frame
let frame = surface.acquire_frame();

encoder.start();

// Render pass
if let mut pass = encoder.render("main", gpu::RenderTargetSet {
    colors: &[gpu::RenderTarget {
        view: frame.texture_view(),
        init_op: gpu::InitOp::Clear(gpu::TextureColor::OpaqueBlack),
        finish_op: gpu::FinishOp::Store,
    }],
    depth_stencil: None,
}) {
    pass.set_scissor(scissor_rect);
    if let mut enc = pass.with(&render_pipeline) {
        enc.bind(0, &uniforms);
        enc.draw(0, vertex_count, 0, 1);
    }
}

context.submit(&mut encoder);
context.present(frame);
```

---

## 8. File Reference

### Core Graphics (`blade-graphics/src/`)

| File | Lines | Purpose |
|------|-------|---------|
| `lib.rs` | ~1200 | Public API, types, traits |
| `shader.rs` | ~313 | Shader compilation, binding resolution |
| `traits.rs` | ~100 | Shared traits |
| `util.rs` | ~200 | Error reporting, utilities |
| `derive.rs` | ~50 | Trait bounds for derive macros |

### Vulkan Backend (`blade-graphics/src/vulkan/`)

| File | Lines | Purpose |
|------|-------|---------|
| `mod.rs` | ~770 | Context, resource types, submit/wait |
| `init.rs` | ~650 | Instance/device creation |
| `command.rs` | ~1100 | Command encoding |
| `pipeline.rs` | ~760 | Pipeline creation |
| `descriptor.rs` | ~400 | Descriptor set management |
| `resource.rs` | ~600 | Buffer/texture creation |
| `surface.rs` | ~400 | Swapchain management |

### Ray-Traced Renderer (`blade-render/src/`)

| File | Purpose |
|------|---------|
| `render/mod.rs` | Main renderer, configuration |
| `asset_hub.rs` | Asset loading coordination |
| `model/mod.rs` | Model loading, mesh processing |
| `shader.rs` | Renderer shaders |
| `texture/mod.rs` | Texture management |

### Examples (`examples/`)

| Example | Features Demonstrated |
|---------|----------------------|
| `mini/` | Minimal compute shader |
| `init/` | Shader loading, environment sampling |
| `ray-query/` | Hardware ray tracing |
| `particle/` | egui integration, particle effects |
| `scene/` | Full scene editor with physics |
| `vehicle/` | Physics, vehicle control |
| `bunnymark/` | Rendering performance benchmark |

---

*Generated for Blade v0.3.0 / blade-graphics v0.7.0*
