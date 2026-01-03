# Blade WebGPU Backend: Comprehensive Analysis & Revised Implementation Plan

## Part 1: The wgpu Copy Trait Issue

### 1.1 Problem Statement

Blade's trait system requires resource types to implement `Copy`:

```rust
// From blade-graphics/src/traits.rs
pub trait ResourceDevice {
    type Buffer: Send + Sync + Clone + Copy + Debug + Hash + PartialEq;
    type Texture: Send + Sync + Clone + Copy + Debug + Hash + PartialEq;
    type TextureView: Send + Sync + Clone + Copy + Debug + Hash + PartialEq;
    type Sampler: Send + Sync + Clone + Copy + Debug + Hash + PartialEq;
    type AccelerationStructure: Send + Sync + Clone + Copy + Debug + Hash + PartialEq;
    // ...
}
```

**wgpu types do NOT implement `Copy`:**

| Type | Clone | Copy | Debug | Hash | PartialEq | Send | Sync |
|------|-------|------|-------|------|-----------|------|------|
| `wgpu::Buffer` | ✅ | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `wgpu::Texture` | ✅ | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `wgpu::TextureView` | ✅ | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `wgpu::Sampler` | ✅ | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ |

### 1.2 Why wgpu Types Are Not Copy

From the wgpu changelog (version 23.0.0):

> "All types in the wgpu API are now Clone. This is implemented with internal reference counting, so cloning for instance a Buffer does copies only the "handle" of the GPU buffer, not the underlying resource."

Key points:
1. **Internal Arc reference counting**: wgpu types wrap resources in `Arc` internally
2. **Clone is cheap**: Just increments a reference count, doesn't copy GPU memory
3. **Copy would be semantically incorrect**: `Copy` implies bitwise copying is valid and equivalent to the original, but reference-counted types need proper reference management
4. **Matches WebGPU JavaScript API**: The web API allows objects to be cloned and shared freely

### 1.3 Why Blade Requires Copy

Looking at the GLES backend:

```rust
// blade-graphics/src/gles/mod.rs
#[derive(Clone, Copy, Debug, Hash, PartialEq)]
pub struct Buffer {
    raw: glow::Buffer,  // glow::Buffer is just a NonZeroU32
    size: u64,
    data: *mut u8,
}
```

`glow::Buffer` is defined as:
```rust
pub type Buffer = NonZeroU32;  // Just a 32-bit handle ID - trivially Copy
```

The GLES backend's resource types are lightweight handle structs that ARE `Copy` because:
1. OpenGL uses integer IDs (GLuint) for all resources
2. No reference counting needed at the Rust level
3. Resource lifetime managed by OpenGL itself

### 1.4 Solution Options Analysis

#### Option A: Handle/Index Pattern (RECOMMENDED)

Create lightweight `Copy` wrapper types that store indices into a resource registry:

```rust
// Our Copy-able handle type
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Buffer {
    id: BufferId,     // u32 index into storage
    size: u64,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct BufferId(u32);

// Storage lives in Context
struct ResourceStorage {
    buffers: SlotMap<BufferId, wgpu::Buffer>,
    textures: SlotMap<TextureId, wgpu::Texture>,
    // ...
}
```

**Pros:**
- ✅ No changes to Blade's public API or trait requirements
- ✅ Well-established pattern in graphics programming (Vulkan, Metal use handles)
- ✅ Minimal memory overhead (just u32 + metadata per resource)
- ✅ Matches how glow works conceptually
- ✅ Easy resource tracking and lifetime management

**Cons:**
- ⚠️ Indirect access requires storage lookup
- ⚠️ Context must outlive all resource handles
- ⚠️ Extra complexity in implementation

#### Option B: Modify Blade's Traits

Remove `Copy` requirement from Blade's traits:

```rust
pub trait ResourceDevice {
    type Buffer: Send + Sync + Clone + Debug + Hash + PartialEq;  // No Copy
    // ...
}
```

**Pros:**
- ✅ Direct use of wgpu types
- ✅ Simpler implementation

**Cons:**
- ❌ Requires changes to Blade core
- ❌ May break existing backends (GLES, Vulkan, Metal)
- ❌ Changes user-facing API
- ❌ Unlikely to be accepted upstream

#### Option C: Use wgpu-core Id Types

wgpu-core has internal `Id<T>` types that ARE Copy:

```rust
// wgpu-core/src/id.rs
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id<T: Marker>(RawId, PhantomData<T>);  // RawId is NonZeroU64
```

**Pros:**
- ✅ Native wgpu types that are Copy
- ✅ Efficient (just u64 handles)

**Cons:**
- ❌ Requires using wgpu-core's lower-level API
- ❌ More complex initialization and resource management
- ❌ Less stable API surface
- ❌ May not work well with WebGPU backend on WASM

### 1.5 Recommended Solution: Handle/Index Pattern

I recommend **Option A** because it:
1. Preserves Blade's existing API contract
2. Is a proven pattern used throughout graphics programming
3. Can be implemented entirely within the WebGPU backend
4. Provides clear resource lifetime management

---

## Part 2: Revised Architecture

### 2.1 Module Structure

```
blade-graphics/src/webgpu/
├── mod.rs              # Types, Context, resource storage
├── command.rs          # CommandEncoder, PassEncoder, PipelineEncoder, PipelineContext
├── pipeline.rs         # Pipeline creation, ShaderDataMapping
├── resource.rs         # ResourceDevice trait implementation
└── platform.rs         # Platform-specific initialization (WASM vs native)
```

### 2.2 Core Type Definitions

```rust
// blade-graphics/src/webgpu/mod.rs

use std::marker::PhantomData;
use std::num::NonZeroU32;

//=============================================================================
// Resource Handle Types (Copy-able)
//=============================================================================

/// Unique identifier for a buffer resource
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct ResourceId(NonZeroU32);

impl ResourceId {
    fn new(index: u32) -> Self {
        Self(NonZeroU32::new(index + 1).unwrap())
    }
    
    fn index(self) -> usize {
        (self.0.get() - 1) as usize
    }
}

/// Handle to a GPU buffer - implements Copy!
#[derive(Clone, Copy, Debug, Hash, PartialEq)]
pub struct Buffer {
    id: ResourceId,
    size: u64,
    mapped_ptr: *mut u8,  // For Shared/Upload memory
}

unsafe impl Send for Buffer {}
unsafe impl Sync for Buffer {}

impl Buffer {
    pub fn data(&self) -> *mut u8 {
        self.mapped_ptr
    }
}

/// Handle to a GPU texture
#[derive(Clone, Copy, Debug, Hash, PartialEq)]
pub struct Texture {
    id: ResourceId,
    format: crate::TextureFormat,
    target_size: [u16; 2],
}

/// Handle to a texture view
#[derive(Clone, Copy, Debug, Hash, PartialEq)]
pub struct TextureView {
    id: ResourceId,
    target_size: [u16; 2],
    aspects: crate::TexelAspects,
}

/// Handle to a sampler
#[derive(Clone, Copy, Debug, Hash, PartialEq)]
pub struct Sampler {
    id: ResourceId,
}

/// Placeholder for acceleration structures (not supported in WebGPU)
#[derive(Clone, Copy, Debug, Hash, PartialEq)]
pub struct AccelerationStructure {
    _phantom: PhantomData<()>,
}

//=============================================================================
// Resource Storage
//=============================================================================

/// Slot in resource storage - can be occupied or free
enum Slot<T> {
    Occupied(T),
    Free { next_free: Option<u32> },
}

/// Simple slot-based resource storage
struct ResourceVec<T> {
    slots: Vec<Slot<T>>,
    first_free: Option<u32>,
    generation: u32,  // For debugging/validation
}

impl<T> ResourceVec<T> {
    fn new() -> Self {
        Self {
            slots: Vec::new(),
            first_free: None,
            generation: 0,
        }
    }
    
    fn insert(&mut self, value: T) -> ResourceId {
        let index = if let Some(free_idx) = self.first_free {
            // Reuse a free slot
            let slot = &mut self.slots[free_idx as usize];
            if let Slot::Free { next_free } = slot {
                self.first_free = *next_free;
            }
            *slot = Slot::Occupied(value);
            free_idx
        } else {
            // Allocate new slot
            let idx = self.slots.len() as u32;
            self.slots.push(Slot::Occupied(value));
            idx
        };
        ResourceId::new(index)
    }
    
    fn remove(&mut self, id: ResourceId) -> Option<T> {
        let index = id.index();
        if index >= self.slots.len() {
            return None;
        }
        
        let slot = std::mem::replace(
            &mut self.slots[index],
            Slot::Free { next_free: self.first_free }
        );
        
        if let Slot::Occupied(value) = slot {
            self.first_free = Some(index as u32);
            Some(value)
        } else {
            // Was already free, restore it
            self.slots[index] = slot;
            None
        }
    }
    
    fn get(&self, id: ResourceId) -> Option<&T> {
        self.slots.get(id.index()).and_then(|slot| {
            if let Slot::Occupied(ref value) = slot {
                Some(value)
            } else {
                None
            }
        })
    }
}

/// All GPU resources owned by the context
struct Resources {
    buffers: ResourceVec<wgpu::Buffer>,
    textures: ResourceVec<wgpu::Texture>,
    texture_views: ResourceVec<wgpu::TextureView>,
    samplers: ResourceVec<wgpu::Sampler>,
    bind_group_layouts: ResourceVec<wgpu::BindGroupLayout>,
    bind_groups: ResourceVec<wgpu::BindGroup>,
}

impl Resources {
    fn new() -> Self {
        Self {
            buffers: ResourceVec::new(),
            textures: ResourceVec::new(),
            texture_views: ResourceVec::new(),
            samplers: ResourceVec::new(),
            bind_group_layouts: ResourceVec::new(),
            bind_groups: ResourceVec::new(),
        }
    }
}

//=============================================================================
// Context
//=============================================================================

pub struct Context {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    resources: std::cell::RefCell<Resources>,
    capabilities: crate::Capabilities,
    device_information: crate::DeviceInformation,
    limits: Limits,
}

#[derive(Clone)]
struct Limits {
    uniform_buffer_alignment: u32,
}

pub struct PlatformError(String);

impl std::fmt::Debug for PlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PlatformError: {}", self.0)
    }
}
```

### 2.3 Command Recording Architecture

Following the GLES pattern exactly:

```rust
// blade-graphics/src/webgpu/mod.rs (continued)

//=============================================================================
// Command Types
//=============================================================================

#[derive(Clone, Debug)]
struct BufferPart {
    id: ResourceId,
    offset: u64,
}

impl From<crate::BufferPiece> for BufferPart {
    fn from(piece: crate::BufferPiece) -> Self {
        Self {
            id: piece.buffer.id,
            offset: piece.offset,
        }
    }
}

#[derive(Clone, Debug)]
struct TexturePart {
    id: ResourceId,
    format: crate::TextureFormat,
    mip_level: u32,
    array_layer: u32,
    origin: [u32; 3],
}

impl From<crate::TexturePiece> for TexturePart {
    fn from(piece: crate::TexturePiece) -> Self {
        Self {
            id: piece.texture.id,
            format: piece.texture.format,
            mip_level: piece.mip_level,
            array_layer: piece.array_layer,
            origin: piece.origin,
        }
    }
}

/// Recorded commands - executed at submit time
#[derive(Debug)]
enum Command {
    // Render commands
    SetRenderPipeline { id: ResourceId },
    SetComputePipeline { id: ResourceId },
    SetBindGroup { index: u32, id: ResourceId, offsets: Vec<u32> },
    SetVertexBuffer { slot: u32, buffer: BufferPart },
    SetIndexBuffer { buffer: BufferPart, format: wgpu::IndexFormat },
    
    Draw {
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    },
    DrawIndexed {
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        base_vertex: i32,
        first_instance: u32,
    },
    
    Dispatch { x: u32, y: u32, z: u32 },
    
    // Transfer commands
    CopyBufferToBuffer {
        src: BufferPart,
        dst: BufferPart,
        size: u64,
    },
    CopyBufferToTexture {
        src: BufferPart,
        bytes_per_row: u32,
        dst: TexturePart,
        size: crate::Extent,
    },
    CopyTextureToBuffer {
        src: TexturePart,
        dst: BufferPart,
        bytes_per_row: u32,
        size: crate::Extent,
    },
    CopyTextureToTexture {
        src: TexturePart,
        dst: TexturePart,
        size: crate::Extent,
    },
    FillBuffer {
        dst: BufferPart,
        size: u64,
        value: u8,
    },
    
    // State commands
    SetViewport(crate::Viewport),
    SetScissor(crate::ScissorRect),
    SetStencilReference(u32),
    SetBlendConstant([f32; 4]),
    
    // Pass structure
    BeginRenderPass {
        color_attachments: Vec<RenderPassColorAttachment>,
        depth_stencil: Option<RenderPassDepthStencilAttachment>,
        label: String,
    },
    EndRenderPass,
    BeginComputePass { label: String },
    EndComputePass,
    
    // Timing
    PushDebugGroup { label: String },
    PopDebugGroup,
}

#[derive(Debug)]
struct RenderPassColorAttachment {
    view_id: ResourceId,
    resolve_target: Option<ResourceId>,
    load_op: wgpu::LoadOp<wgpu::Color>,
    store_op: wgpu::StoreOp,
}

#[derive(Debug)]
struct RenderPassDepthStencilAttachment {
    view_id: ResourceId,
    depth_load_op: wgpu::LoadOp<f32>,
    depth_store_op: wgpu::StoreOp,
    stencil_load_op: wgpu::LoadOp<u32>,
    stencil_store_op: wgpu::StoreOp,
}

//=============================================================================
// Command Encoder
//=============================================================================

pub struct CommandEncoder {
    name: String,
    commands: Vec<Command>,
    plain_data: Vec<u8>,           // Inline uniform data
    present_frames: Vec<Frame>,
    limits: Limits,
}

enum PassKind {
    Transfer,
    Compute,
    Render,
}

pub struct PassEncoder<'a, P> {
    commands: &'a mut Vec<Command>,
    plain_data: &'a mut Vec<u8>,
    kind: PassKind,
    pipeline: std::marker::PhantomData<P>,
    limits: &'a Limits,
}

pub type ComputeCommandEncoder<'a> = PassEncoder<'a, ComputePipeline>;
pub type RenderCommandEncoder<'a> = PassEncoder<'a, RenderPipeline>;

pub struct PipelineEncoder<'a> {
    commands: &'a mut Vec<Command>,
    plain_data: &'a mut Vec<u8>,
    group_mappings: &'a [ShaderDataMapping],
    limits: &'a Limits,
}

pub struct PipelineContext<'a> {
    commands: &'a mut Vec<Command>,
    plain_data: &'a mut Vec<u8>,
    targets: &'a [BindingSlot],
    limits: &'a Limits,
}
```

### 2.4 ShaderData Binding System

```rust
// blade-graphics/src/webgpu/mod.rs (continued)

//=============================================================================
// Shader Data Binding
//=============================================================================

/// Maps a logical binding index to a WebGPU binding slot
#[derive(Clone, Debug)]
struct BindingSlot {
    group: u32,
    binding: u32,
}

/// Mapping from ShaderDataLayout to WebGPU bind group structure
struct ShaderDataMapping {
    /// For each binding in the ShaderDataLayout, the target slot(s)
    targets: Box<[Vec<BindingSlot>]>,
    /// The bind group layout
    layout_id: ResourceId,
}

//=============================================================================
// Pipeline Types
//=============================================================================

struct PipelineInner {
    raw: wgpu::RenderPipeline,  // or ComputePipeline
    group_mappings: Box<[ShaderDataMapping]>,
    bind_group_layouts: Vec<ResourceId>,
}

pub struct ComputePipeline {
    inner: PipelineInner,
    wg_size: [u32; 3],
}

impl ComputePipeline {
    pub fn get_workgroup_size(&self) -> [u32; 3] {
        self.wg_size
    }
}

pub struct RenderPipeline {
    inner: PipelineInner,
    topology: crate::PrimitiveTopology,
}

//=============================================================================
// Surface & Frame
//=============================================================================

pub struct Surface {
    raw: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
}

#[derive(Debug)]
pub struct Frame {
    texture: wgpu::SurfaceTexture,
    view: wgpu::TextureView,
    view_id: ResourceId,
}

impl Frame {
    pub fn texture(&self) -> Texture {
        // Create a temporary texture handle for the surface texture
        Texture {
            id: ResourceId::new(0),  // Special ID for surface textures
            format: crate::TextureFormat::Bgra8UnormSrgb,  // Will be set from config
            target_size: [0, 0],  // Will be set from config
        }
    }
    
    pub fn texture_view(&self) -> TextureView {
        TextureView {
            id: self.view_id,
            target_size: [0, 0],
            aspects: crate::TexelAspects::COLOR,
        }
    }
}

//=============================================================================
// Sync Point
//=============================================================================

#[derive(Clone, Debug)]
pub struct SyncPoint {
    // wgpu handles synchronization internally
    // We just track submission for waiting
    submission_index: wgpu::SubmissionIndex,
}
```

### 2.5 ShaderBindable Implementations

```rust
// blade-graphics/src/webgpu/command.rs

//=============================================================================
// ShaderBindable Implementations
//=============================================================================

/// Bind plain data (uniforms) - works for any Pod type
impl<T: bytemuck::Pod> crate::ShaderBindable for T {
    fn bind_to(&self, ctx: &mut super::PipelineContext, index: u32) {
        let self_slice = bytemuck::bytes_of(self);
        let alignment = ctx.limits.uniform_buffer_alignment as usize;
        
        // Align the data
        let rem = ctx.plain_data.len() % alignment;
        if rem != 0 {
            ctx.plain_data.resize(ctx.plain_data.len() - rem + alignment, 0);
        }
        
        let offset = ctx.plain_data.len() as u32;
        let size = round_up_uniform_size(self_slice.len() as u32);
        
        ctx.plain_data.extend_from_slice(self_slice);
        ctx.plain_data.extend((self_slice.len() as u32..size).map(|_| 0));
        
        // Record binding command for each target slot
        for slot in ctx.targets[index as usize].iter() {
            ctx.commands.push(super::Command::BindUniform {
                group: slot.group,
                binding: slot.binding,
                offset,
                size,
            });
        }
    }
}

impl crate::ShaderBindable for super::TextureView {
    fn bind_to(&self, ctx: &mut super::PipelineContext, index: u32) {
        for slot in ctx.targets[index as usize].iter() {
            ctx.commands.push(super::Command::BindTexture {
                group: slot.group,
                binding: slot.binding,
                view_id: self.id,
            });
        }
    }
}

impl crate::ShaderBindable for super::Sampler {
    fn bind_to(&self, ctx: &mut super::PipelineContext, index: u32) {
        for slot in ctx.targets[index as usize].iter() {
            ctx.commands.push(super::Command::BindSampler {
                group: slot.group,
                binding: slot.binding,
                sampler_id: self.id,
            });
        }
    }
}

impl crate::ShaderBindable for crate::BufferPiece {
    fn bind_to(&self, ctx: &mut super::PipelineContext, index: u32) {
        for slot in ctx.targets[index as usize].iter() {
            ctx.commands.push(super::Command::BindBuffer {
                group: slot.group,
                binding: slot.binding,
                buffer_id: self.buffer.id,
                offset: self.offset,
                size: self.buffer.size - self.offset,
            });
        }
    }
}

impl crate::ShaderBindable for super::AccelerationStructure {
    fn bind_to(&self, _ctx: &mut super::PipelineContext, _index: u32) {
        panic!("Acceleration structures not supported in WebGPU backend");
    }
}

// Array types - not fully supported in base WebGPU
impl<'a, const N: crate::ResourceIndex> crate::ShaderBindable for &'a crate::TextureArray<N> {
    fn bind_to(&self, _ctx: &mut super::PipelineContext, _index: u32) {
        unimplemented!("Texture arrays require bindless support");
    }
}

impl<'a, const N: crate::ResourceIndex> crate::ShaderBindable for &'a crate::BufferArray<N> {
    fn bind_to(&self, _ctx: &mut super::PipelineContext, _index: u32) {
        unimplemented!("Buffer arrays require bindless support");
    }
}

fn round_up_uniform_size(size: u32) -> u32 {
    if size & 0xF != 0 {
        (size | 0xF) + 1
    } else {
        size
    }
}
```

---

## Part 3: Implementation Plan

### Phase 1: Foundation (Days 1-3)

#### Day 1: Module Setup & Core Types
- [ ] Create `webgpu/mod.rs` with handle types
- [ ] Implement `ResourceVec<T>` storage
- [ ] Define `Command` enum
- [ ] Add conditional compilation to `lib.rs`

```rust
// blade-graphics/src/lib.rs addition
#[cfg_attr(
    all(
        any(webgpu, target_arch = "wasm32"),
        not(gles)  // WebGPU takes precedence
    ),
    path = "webgpu/mod.rs"
)]
mod hal;
```

#### Day 2: Context Initialization
- [ ] Implement async `Context::new()` for WASM
- [ ] Implement sync `Context::new()` for native
- [ ] Surface creation and configuration
- [ ] Device/adapter selection

#### Day 3: Basic Resource Creation
- [ ] Implement `create_buffer` / `destroy_buffer`
- [ ] Implement `create_texture` / `destroy_texture`
- [ ] Implement `create_texture_view` / `destroy_texture_view`
- [ ] Implement `create_sampler` / `destroy_sampler`

### Phase 2: Command System (Days 4-7)

#### Day 4: CommandEncoder Structure
- [ ] Implement `CommandEncoder` struct
- [ ] Implement `start()` method
- [ ] Implement `transfer()` pass creation
- [ ] Basic pass lifecycle

#### Day 5: Transfer Commands
- [ ] `fill_buffer`
- [ ] `copy_buffer_to_buffer`
- [ ] `copy_buffer_to_texture`
- [ ] `copy_texture_to_buffer`
- [ ] `copy_texture_to_texture`

#### Day 6: Render Pass Structure
- [ ] Implement `render()` pass creation
- [ ] Color attachment handling
- [ ] Depth/stencil attachment handling
- [ ] Clear operations

#### Day 7: Command Execution
- [ ] Implement `submit()` - convert Commands to wgpu calls
- [ ] Implement `wait_for()` with SyncPoint
- [ ] Frame presentation

### Phase 3: Pipeline System (Days 8-11)

#### Day 8: Shader Compilation
- [ ] Implement `create_shader()` using wgpu's WGSL support
- [ ] Shader reflection for binding info
- [ ] Error handling and validation

#### Day 9: Pipeline Creation
- [ ] Implement `create_render_pipeline()`
- [ ] Vertex attribute mapping
- [ ] Color target state
- [ ] Depth/stencil state

#### Day 10: ShaderData System
- [ ] Implement `ShaderDataMapping`
- [ ] Build bind group layouts from `ShaderDataLayout`
- [ ] Implement `PipelineContext`

#### Day 11: Binding Implementation
- [ ] Implement `ShaderBindable` for all types
- [ ] Dynamic uniform buffer management
- [ ] Bind group creation during submit

### Phase 4: Rendering (Days 12-14)

#### Day 12: PipelineEncoder
- [ ] Implement `with()` for binding pipeline
- [ ] `bind()` method for ShaderData
- [ ] Vertex buffer binding

#### Day 13: Draw Commands
- [ ] `draw()`
- [ ] `draw_indexed()`
- [ ] Viewport and scissor

#### Day 14: Integration Testing
- [ ] Test with GPUI basic rendering
- [ ] Fix coordinate system issues
- [ ] Performance validation

---

## Part 4: Critical Implementation Details

### 4.1 Command Execution Pattern

```rust
impl Context {
    pub fn submit(&self, encoder: &mut CommandEncoder) -> SyncPoint {
        let resources = self.resources.borrow();
        
        // Create wgpu command encoder
        let mut cmd_encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some(&encoder.name),
            }
        );
        
        // Create uniform buffer for plain data
        let uniform_buffer = if !encoder.plain_data.is_empty() {
            Some(self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Plain Data Buffer"),
                contents: &encoder.plain_data,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }))
        } else {
            None
        };
        
        // Execute recorded commands
        let mut current_render_pass: Option<wgpu::RenderPass> = None;
        let mut current_compute_pass: Option<wgpu::ComputePass> = None;
        
        for command in &encoder.commands {
            match command {
                Command::BeginRenderPass { color_attachments, depth_stencil, label } => {
                    // Convert our attachment types to wgpu types
                    let color_views: Vec<_> = color_attachments.iter()
                        .map(|att| {
                            let view = resources.texture_views.get(att.view_id).unwrap();
                            wgpu::RenderPassColorAttachment {
                                view,
                                resolve_target: att.resolve_target.map(|id| 
                                    resources.texture_views.get(id).unwrap()
                                ),
                                ops: wgpu::Operations {
                                    load: att.load_op,
                                    store: att.store_op,
                                },
                            }
                        })
                        .collect();
                    
                    current_render_pass = Some(cmd_encoder.begin_render_pass(
                        &wgpu::RenderPassDescriptor {
                            label: Some(label),
                            color_attachments: &color_views.iter()
                                .map(Some)
                                .collect::<Vec<_>>(),
                            depth_stencil_attachment: depth_stencil.as_ref().map(|ds| {
                                wgpu::RenderPassDepthStencilAttachment {
                                    view: resources.texture_views.get(ds.view_id).unwrap(),
                                    depth_ops: Some(wgpu::Operations {
                                        load: ds.depth_load_op,
                                        store: ds.depth_store_op,
                                    }),
                                    stencil_ops: Some(wgpu::Operations {
                                        load: ds.stencil_load_op,
                                        store: ds.stencil_store_op,
                                    }),
                                }
                            }),
                            // ...
                        }
                    ));
                }
                
                Command::Draw { vertex_count, instance_count, first_vertex, first_instance } => {
                    if let Some(ref mut pass) = current_render_pass {
                        pass.draw(*first_vertex..*first_vertex + *vertex_count, 
                                  *first_instance..*first_instance + *instance_count);
                    }
                }
                
                // ... handle other commands
                
                _ => {}
            }
        }
        
        // Submit and get submission index
        let submission_index = self.queue.submit(std::iter::once(cmd_encoder.finish()));
        
        // Present any frames
        for frame in encoder.present_frames.drain(..) {
            frame.texture.present();
        }
        
        SyncPoint { submission_index }
    }
    
    pub fn wait_for(&self, sp: &SyncPoint, timeout_ms: u32) -> bool {
        self.device.poll(wgpu::Maintain::WaitForSubmissionIndex(sp.submission_index));
        true  // wgpu poll blocks until complete
    }
}
```

### 4.2 Coordinate System Handling

WebGPU uses the same coordinate system as D3D/Metal:
- NDC: X right, Y up, Z into screen
- Depth range: [0, 1]
- Texture origin: top-left

GLES backend uses `ADJUST_COORDINATE_SPACE` flag in naga. For WebGPU, we need to handle this ourselves if targeting GL-style coordinates.

```rust
// If Blade expects GL-style coordinates, add Y-flip in vertex shader output
// This is handled by naga when targeting GLES, but WebGPU is already in the right space
// So we may not need any adjustment if Blade expects D3D/Metal style

// For presentation, wgpu handles the flip automatically
```

### 4.3 Memory Type Mapping

```rust
fn buffer_usage_from_memory(memory: crate::Memory, _usage: wgpu::BufferUsages) -> wgpu::BufferUsages {
    match memory {
        crate::Memory::Device => wgpu::BufferUsages::empty(),
        crate::Memory::Shared => {
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::MAP_WRITE
        }
        crate::Memory::Upload => {
            wgpu::BufferUsages::MAP_WRITE | wgpu::BufferUsages::COPY_SRC
        }
        crate::Memory::External(_) => {
            panic!("External memory not supported in WebGPU backend")
        }
    }
}
```

### 4.4 Platform-Specific Initialization

```rust
// blade-graphics/src/webgpu/platform.rs

#[cfg(target_arch = "wasm32")]
pub async fn create_context(desc: &crate::ContextDesc) -> Result<Context, PlatformError> {
    use wasm_bindgen_futures::JsFuture;
    
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,
        ..Default::default()
    });
    
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .ok_or_else(|| PlatformError("No suitable adapter found".into()))?;
    
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Blade WebGPU Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        )
        .await
        .map_err(|e| PlatformError(format!("Device request failed: {}", e)))?;
    
    Ok(Context::from_raw(instance, adapter, device, queue))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn create_context(desc: &crate::ContextDesc) -> Result<Context, PlatformError> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .ok_or_else(|| PlatformError("No suitable adapter found".into()))?;
    
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("Blade WebGPU Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
        },
        None,
    ))
    .map_err(|e| PlatformError(format!("Device request failed: {}", e)))?;
    
    Ok(Context::from_raw(instance, adapter, device, queue))
}
```

---

## Part 5: Verification Checklist

### Trait Compliance
- [ ] `Buffer: Clone + Copy + Debug + Hash + PartialEq + Send + Sync`
- [ ] `Texture: Clone + Copy + Debug + Hash + PartialEq + Send + Sync`
- [ ] `TextureView: Clone + Copy + Debug + Hash + PartialEq + Send + Sync`
- [ ] `Sampler: Clone + Copy + Debug + Hash + PartialEq + Send + Sync`
- [ ] `ResourceDevice` trait implemented
- [ ] `ShaderDevice` trait implemented
- [ ] `CommandDevice` trait implemented
- [ ] All encoder traits implemented

### API Compatibility
- [ ] `CommandEncoder::transfer()` returns correct type
- [ ] `CommandEncoder::compute()` returns correct type
- [ ] `CommandEncoder::render()` returns correct type
- [ ] `PassEncoder::with()` returns `PipelineEncoder`
- [ ] `PipelineEncoder::bind()` accepts `ShaderData`
- [ ] All `ShaderBindable` types work correctly

### WASM Compatibility
- [ ] No `pollster::block_on` in WASM code paths
- [ ] Async initialization works
- [ ] Surface creation works with canvas
- [ ] Frame presentation works

---

## Part 6: Risk Mitigation

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Handle lookup overhead | Low | Profile, optimize ResourceVec if needed |
| Bind group explosion | Medium | Cache bind groups, reuse when possible |
| Plain data buffer size | Medium | Size estimation, dynamic reallocation |
| WASM async complexity | Medium | Thorough testing on web |
| ShaderData mapping errors | High | Extensive validation, good error messages |

---

## Conclusion

The **Handle/Index pattern** solves the Copy trait incompatibility cleanly while maintaining full API compatibility with Blade's existing design. The implementation follows the GLES backend structure closely, making it easier to understand and maintain.

Key success factors:
1. ResourceVec provides efficient O(1) lookup and insertion
2. Command recording pattern matches GLES exactly
3. ShaderData binding system preserves Blade's ergonomic API
4. Platform-specific initialization handles WASM constraints

Total estimated implementation time: **14 working days**
