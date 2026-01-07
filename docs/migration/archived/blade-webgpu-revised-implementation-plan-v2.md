# Blade WebGPU Backend: Comprehensive Analysis & Revised Implementation Plan

**Version:** 2.0 (Hardened)  
**Status:** APPROVED for implementation  
**Key Changes from v1:** slotmap for generational safety, RwLock for concurrency, explicit shadow memory lifecycle

---

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

### 2.2 Core Type Definitions (Using slotmap)

**CRITICAL: Using `slotmap` crate for generational safety**

The ABA problem occurs when:
1. Buffer A created at index 0
2. Buffer A destroyed (index 0 now free)
3. Buffer B created at index 0
4. Old handle to A now silently accesses B

`slotmap` solves this with generation counting - accessing a dead key returns `None`.

```rust
// blade-graphics/src/webgpu/mod.rs

use slotmap::{new_key_type, SlotMap};
use std::sync::RwLock;

//=============================================================================
// Slotmap Key Types (Type-Safe, Generational)
//=============================================================================

new_key_type! {
    /// Key for buffer resources - cannot be confused with other resource types
    pub struct BufferKey;
    /// Key for texture resources
    pub struct TextureKey;
    /// Key for texture view resources
    pub struct TextureViewKey;
    /// Key for sampler resources
    pub struct SamplerKey;
    /// Key for bind group layout resources
    pub struct BindGroupLayoutKey;
    /// Key for bind group resources
    pub struct BindGroupKey;
    /// Key for render pipeline resources
    pub struct RenderPipelineKey;
    /// Key for compute pipeline resources
    pub struct ComputePipelineKey;
}

//=============================================================================
// Resource Handle Types (Copy-able, Type-Safe)
//=============================================================================

/// Handle to a GPU buffer - implements Copy!
/// The `raw` field is a generational key that prevents use-after-free.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Buffer {
    raw: BufferKey,
    size: u64,
    /// Pointer to CPU shadow memory for Upload/Shared buffers.
    /// NULL for Device-only buffers.
    data: *mut u8,
}

// SAFETY: Buffer is just a key + metadata. The actual wgpu::Buffer
// lives in the Hub and is accessed via the key.
unsafe impl Send for Buffer {}
unsafe impl Sync for Buffer {}

impl Buffer {
    /// Returns pointer to CPU shadow memory, or null for device-only buffers.
    pub fn data(&self) -> *mut u8 {
        self.data
    }
}

/// Handle to a GPU texture
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Texture {
    raw: TextureKey,
    format: crate::TextureFormat,
    target_size: [u16; 2],
}

/// Handle to a texture view
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct TextureView {
    raw: TextureViewKey,
    target_size: [u16; 2],
    aspects: crate::TexelAspects,
}

/// Handle to a sampler
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Sampler {
    raw: SamplerKey,
}

/// Placeholder for acceleration structures (not supported in WebGPU)
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct AccelerationStructure {
    _phantom: std::marker::PhantomData<()>,
}

//=============================================================================
// Internal Storage Entry Types
//=============================================================================

/// Internal buffer entry with shadow memory
struct BufferEntry {
    gpu: wgpu::Buffer,
    /// CPU shadow memory for Upload/Shared buffers
    shadow: Option<Box<[u8]>>,
    /// True if shadow memory is dirty and needs sync
    dirty: bool,
}

/// Internal texture entry
struct TextureEntry {
    gpu: wgpu::Texture,
    format: crate::TextureFormat,
}

//=============================================================================
// The Hub: Central Resource Storage (RwLock for Concurrency)
//=============================================================================

/// Central storage for all GPU resources.
/// Uses RwLock to allow concurrent read access during command recording.
/// 
/// # Concurrency Model
/// - Command Recording: `read()` access (high frequency, concurrent)
/// - Resource Creation/Destruction: `write()` access (low frequency)
struct Hub {
    buffers: SlotMap<BufferKey, BufferEntry>,
    textures: SlotMap<TextureKey, TextureEntry>,
    texture_views: SlotMap<TextureViewKey, wgpu::TextureView>,
    samplers: SlotMap<SamplerKey, wgpu::Sampler>,
    bind_group_layouts: SlotMap<BindGroupLayoutKey, wgpu::BindGroupLayout>,
    bind_groups: SlotMap<BindGroupKey, wgpu::BindGroup>,
    render_pipelines: SlotMap<RenderPipelineKey, RenderPipelineEntry>,
    compute_pipelines: SlotMap<ComputePipelineKey, ComputePipelineEntry>,
}

impl Hub {
    fn new() -> Self {
        Self {
            buffers: SlotMap::with_key(),
            textures: SlotMap::with_key(),
            texture_views: SlotMap::with_key(),
            samplers: SlotMap::with_key(),
            bind_group_layouts: SlotMap::with_key(),
            bind_groups: SlotMap::with_key(),
            render_pipelines: SlotMap::with_key(),
            compute_pipelines: SlotMap::with_key(),
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
    /// RwLock allows concurrent read access during command recording
    hub: RwLock<Hub>,
    capabilities: crate::Capabilities,
    device_information: crate::DeviceInformation,
    limits: Limits,
    /// Cache for bind groups to avoid recreation
    bind_group_cache: RwLock<BindGroupCache>,
}

impl Context {
    /// Fast read access to a buffer's GPU handle during command recording.
    /// Returns None if the buffer has been destroyed (generational safety).
    #[inline]
    pub(crate) fn get_buffer(&self, key: BufferKey) -> Option<&wgpu::Buffer> {
        // This only takes a read lock - concurrent with other command recording
        let hub = self.hub.read().unwrap();
        hub.buffers.get(key).map(|e| &e.gpu)
    }
    
    /// Fast read access to a texture view during command recording.
    #[inline]
    pub(crate) fn get_texture_view(&self, key: TextureViewKey) -> Option<&wgpu::TextureView> {
        let hub = self.hub.read().unwrap();
        hub.texture_views.get(key)
    }
    
    /// Sync all dirty shadow buffers to GPU.
    /// Called before submit.
    fn sync_dirty_buffers(&self) {
        let hub = self.hub.read().unwrap();
        for (key, entry) in hub.buffers.iter() {
            if entry.dirty {
                if let Some(ref shadow) = entry.shadow {
                    self.queue.write_buffer(&entry.gpu, 0, shadow);
                }
            }
        }
        // Clear dirty flags (needs write lock)
        drop(hub);
        let mut hub = self.hub.write().unwrap();
        for (_, entry) in hub.buffers.iter_mut() {
            entry.dirty = false;
        }
    }
}

#[derive(Clone)]
struct Limits {
    uniform_buffer_alignment: u32,
    max_bind_groups: u32,
}

pub struct PlatformError(String);

impl std::fmt::Debug for PlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PlatformError: {}", self.0)
    }
}
impl std::fmt::Display for PlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for PlatformError {}
```

### 2.3 Shadow Memory Lifecycle

**The Problem:** Blade's API expects users to write to `buffer.data()` and have it "just work". WebGPU doesn't support simultaneous CPU access and GPU usage.

**Solution: Shadow Memory Pattern**

```
┌─────────────────────────────────────────────────────────────────┐
│                      Buffer Creation                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Memory::Device    →  wgpu::Buffer only, data ptr = null        │
│  Memory::Upload    →  wgpu::Buffer + Box<[u8]>, data ptr set    │
│  Memory::Shared    →  (Limited support - see below)              │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                      Write Flow (Upload)                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. User writes to buffer.data() (CPU shadow memory)            │
│  2. Mark buffer as "dirty"                                       │
│  3. On submit(): queue.write_buffer(shadow → gpu)               │
│  4. Clear dirty flag                                             │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                      Read Flow (Shared) - LIMITED                │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  WASM: Synchronous readback is IMPOSSIBLE.                       │
│        → buffer.data() for Shared is effectively write-only     │
│        → Or panic on read attempt with clear error message       │
│                                                                  │
│  Native: Can use pollster::block_on for map_async               │
│          → Expensive, blocks until GPU completes                 │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

```rust
// Buffer creation with shadow memory
impl Context {
    pub fn create_buffer(&self, desc: crate::BufferDesc) -> Buffer {
        let usage = map_buffer_usage(desc.usage);
        
        let gpu_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: desc.name.as_deref(),
            size: desc.size,
            usage,
            mapped_at_creation: false,
        });
        
        // Create shadow memory for Upload/Shared buffers
        let (shadow, data_ptr) = match desc.memory {
            crate::Memory::Device => (None, std::ptr::null_mut()),
            crate::Memory::Upload | crate::Memory::Shared => {
                let mut shadow = vec![0u8; desc.size as usize].into_boxed_slice();
                let ptr = shadow.as_mut_ptr();
                (Some(shadow), ptr)
            }
            crate::Memory::External(_) => {
                panic!("External memory not supported in WebGPU backend")
            }
        };
        
        let entry = BufferEntry {
            gpu: gpu_buffer,
            shadow,
            dirty: false,
        };
        
        let key = self.hub.write().unwrap().buffers.insert(entry);
        
        Buffer {
            raw: key,
            size: desc.size,
            data: data_ptr,
        }
    }
}
```

### 2.4 Bind Group Caching

WebGPU requires binding resources in groups, while Blade binds individually. We cache bind groups to avoid recreation overhead.

```rust
//=============================================================================
// Bind Group Cache
//=============================================================================

use std::collections::HashMap;

/// Key for bind group cache - identifies a unique combination of resources
#[derive(Clone, Hash, PartialEq, Eq)]
struct BindGroupCacheKey {
    layout_key: BindGroupLayoutKey,
    /// Sorted list of (binding_index, resource_key) pairs
    bindings: Vec<(u32, ResourceBinding)>,
}

/// A resource bound to a bind group
#[derive(Clone, Hash, PartialEq, Eq)]
enum ResourceBinding {
    Buffer { key: BufferKey, offset: u64, size: u64 },
    Texture { key: TextureViewKey },
    Sampler { key: SamplerKey },
    /// Uniform data from plain_data buffer
    UniformRange { offset: u32, size: u32 },
}

struct BindGroupCache {
    /// Map from cache key to bind group
    groups: HashMap<BindGroupCacheKey, BindGroupKey>,
    /// Track which bind groups reference each resource (for cleanup)
    resource_refs: HashMap<BufferKey, Vec<BindGroupCacheKey>>,
    /// Maximum cache size before cleanup
    max_size: usize,
}

impl BindGroupCache {
    fn new(max_size: usize) -> Self {
        Self {
            groups: HashMap::new(),
            resource_refs: HashMap::new(),
            max_size,
        }
    }
    
    fn get_or_create(
        &mut self,
        key: BindGroupCacheKey,
        hub: &Hub,
        device: &wgpu::Device,
    ) -> BindGroupKey {
        if let Some(&existing) = self.groups.get(&key) {
            return existing;
        }
        
        // Cache miss - create new bind group
        // ... (bind group creation logic)
        
        // Evict if over capacity (simple LRU would be better)
        if self.groups.len() >= self.max_size {
            self.evict_oldest();
        }
        
        todo!("Create bind group and insert into cache")
    }
    
    /// Invalidate all bind groups referencing a destroyed resource
    fn invalidate_buffer(&mut self, key: BufferKey, hub: &mut Hub) {
        if let Some(refs) = self.resource_refs.remove(&key) {
            for cache_key in refs {
                if let Some(bg_key) = self.groups.remove(&cache_key) {
                    hub.bind_groups.remove(bg_key);
                }
            }
        }
    }
    
    fn evict_oldest(&mut self) {
        // Simple: just clear half the cache
        // Better: track access time and evict LRU
        let to_remove: Vec<_> = self.groups.keys()
            .take(self.groups.len() / 2)
            .cloned()
            .collect();
        for key in to_remove {
            self.groups.remove(&key);
        }
    }
}
```

### 2.5 Command Recording Architecture

Following the GLES pattern exactly, but using slotmap keys:

```rust
// blade-graphics/src/webgpu/mod.rs (continued)

//=============================================================================
// Command Types (using slotmap keys)
//=============================================================================

#[derive(Clone, Debug)]
struct BufferPart {
    key: BufferKey,
    offset: u64,
}

impl From<crate::BufferPiece> for BufferPart {
    fn from(piece: crate::BufferPiece) -> Self {
        Self {
            key: piece.buffer.raw,
            offset: piece.offset,
        }
    }
}

#[derive(Clone, Debug)]
struct TexturePart {
    key: TextureKey,
    format: crate::TextureFormat,
    mip_level: u32,
    array_layer: u32,
    origin: [u32; 3],
}

impl From<crate::TexturePiece> for TexturePart {
    fn from(piece: crate::TexturePiece) -> Self {
        Self {
            key: piece.texture.raw,
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
    SetRenderPipeline { key: RenderPipelineKey },
    SetComputePipeline { key: ComputePipelineKey },
    SetBindGroup { index: u32, key: BindGroupKey, offsets: Vec<u32> },
    SetVertexBuffer { slot: u32, buffer: BufferPart },
    SetIndexBuffer { buffer: BufferPart, format: wgpu::IndexFormat },
    
    // Uniform binding (resolved to bind group at submit)
    BindUniform { group: u32, binding: u32, offset: u32, size: u32 },
    BindTexture { group: u32, binding: u32, view_key: TextureViewKey },
    BindSampler { group: u32, binding: u32, sampler_key: SamplerKey },
    BindBuffer { group: u32, binding: u32, buffer_key: BufferKey, offset: u64, size: u64 },
    
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
    view_key: TextureViewKey,
    resolve_target: Option<TextureViewKey>,
    load_op: wgpu::LoadOp<wgpu::Color>,
    store_op: wgpu::StoreOp,
}

#[derive(Debug)]
struct RenderPassDepthStencilAttachment {
    view_key: TextureViewKey,
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
    /// Reference to context for resource lookups during submission
    context: *const Context,  // Safe: encoder lifetime < context lifetime
}

// SAFETY: CommandEncoder only reads from Context via the pointer,
// and Context is protected by RwLock internally.
unsafe impl Send for CommandEncoder {}

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

pub type TransferCommandEncoder<'a> = PassEncoder<'a, ()>;
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

### 2.6 ShaderData Binding System

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
    /// The bind group layout key
    layout_key: BindGroupLayoutKey,
}

//=============================================================================
// Pipeline Types
//=============================================================================

/// Internal render pipeline entry
struct RenderPipelineEntry {
    raw: wgpu::RenderPipeline,
    group_mappings: Box<[ShaderDataMapping]>,
    bind_group_layouts: Vec<BindGroupLayoutKey>,
    topology: crate::PrimitiveTopology,
}

/// Internal compute pipeline entry
struct ComputePipelineEntry {
    raw: wgpu::ComputePipeline,
    group_mappings: Box<[ShaderDataMapping]>,
    bind_group_layouts: Vec<BindGroupLayoutKey>,
    wg_size: [u32; 3],
}

/// Public handle to a compute pipeline
pub struct ComputePipeline {
    raw: ComputePipelineKey,
    wg_size: [u32; 3],
}

impl ComputePipeline {
    pub fn get_workgroup_size(&self) -> [u32; 3] {
        self.wg_size
    }
}

/// Public handle to a render pipeline
pub struct RenderPipeline {
    raw: RenderPipelineKey,
    topology: crate::PrimitiveTopology,
}

//=============================================================================
// Surface & Frame
//=============================================================================

pub struct Surface {
    raw: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    format: crate::TextureFormat,
}

#[derive(Debug)]
pub struct Frame {
    texture: wgpu::SurfaceTexture,
    view: wgpu::TextureView,
    view_key: TextureViewKey,
    target_size: [u16; 2],
    format: crate::TextureFormat,
}

impl Frame {
    pub fn texture(&self) -> Texture {
        // Note: Surface textures use a special key path
        Texture {
            raw: TextureKey::default(),  // Special null key for surface textures
            format: self.format,
            target_size: self.target_size,
        }
    }
    
    pub fn texture_view(&self) -> TextureView {
        TextureView {
            raw: self.view_key,
            target_size: self.target_size,
            aspects: crate::TexelAspects::COLOR,
        }
    }
}

//=============================================================================
// Sync Point
//=============================================================================

#[derive(Clone, Debug)]
pub struct SyncPoint {
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

## Part 3: Implementation Plan (Revised Timeline)

**Key Change:** Front-load the Arena/Hub setup as it touches everything.

### Phase 1: Foundation - The Hub (Days 1-2)

#### Day 1: Arena & Context (THE HUB)
**This is the critical foundation - everything else depends on it.**

- [ ] Add `slotmap = "1.0"` to Cargo.toml
- [ ] Create `webgpu/mod.rs` with all `new_key_type!` declarations
- [ ] Implement `Hub` struct with all SlotMaps
- [ ] Implement `Context` with `RwLock<Hub>`
- [ ] Implement `create_buffer` / `destroy_buffer` returning Keys
- [ ] Implement `create_texture` / `destroy_texture`
- [ ] Add conditional compilation to `lib.rs`

```rust
// blade-graphics/src/lib.rs addition
#[cfg_attr(
    all(
        any(webgpu, target_arch = "wasm32"),
        not(gles)  // WebGPU takes precedence over GLES on WASM
    ),
    path = "webgpu/mod.rs"
)]
mod hal;
```

**Verification:** Can create and destroy buffers, keys are type-safe, dead keys return None.

#### Day 2: Command Encoder Structure (Moved Up)
**Must verify resource lookups work before implementing complex pipelines.**

- [ ] Implement `CommandEncoder` struct with command Vec
- [ ] Implement `start()` method
- [ ] Implement `transfer()` pass creation
- [ ] Implement basic `set_vertex_buffer` with Hub lookup
- [ ] Test: Can record commands that reference buffers

```rust
// Verification test
let buffer = ctx.create_buffer(desc);
let mut encoder = ctx.create_command_encoder();
let mut transfer = encoder.transfer();
// This must not panic and must resolve the key correctly
transfer.fill_buffer(crate::BufferPiece { buffer, offset: 0 }, 4, 0xFF);
```

### Phase 2: Resource Operations (Days 3-4)

#### Day 3: Complete Resource Creation
- [ ] Implement shadow memory for Upload buffers
- [ ] Implement `create_texture_view` / `destroy_texture_view`
- [ ] Implement `create_sampler` / `destroy_sampler`
- [ ] Surface creation and configuration
- [ ] Platform-specific async initialization (WASM vs native)

#### Day 4: Transfer Commands
- [ ] `fill_buffer` (uses queue.write_buffer for fill pattern)
- [ ] `copy_buffer_to_buffer`
- [ ] `copy_buffer_to_texture`
- [ ] `copy_texture_to_buffer`
- [ ] `copy_texture_to_texture`
- [ ] Shadow buffer sync before submit

### Phase 3: Pipelines & Layouts (Days 5-7)

#### Day 5: Shader & Layout Infrastructure
- [ ] Implement `create_shader()` using wgpu's WGSL support
- [ ] Map Blade's `ShaderDataLayout` to `wgpu::BindGroupLayout`
- [ ] Build `ShaderDataMapping` during pipeline creation
- [ ] Store layouts in Hub

#### Day 6: Pipeline Creation
- [ ] Implement `create_render_pipeline()`
- [ ] Vertex attribute mapping to wgpu format
- [ ] Color target state mapping
- [ ] Depth/stencil state mapping
- [ ] Implement `create_compute_pipeline()`

#### Day 7: Bind Group Cache
- [ ] Implement `BindGroupCache` with proper key structure
- [ ] Cache invalidation on resource destruction
- [ ] LRU eviction for cache size management
- [ ] Integration with command execution

### Phase 4: Binding & Rendering (Days 8-10)

#### Day 8: ShaderBindable Implementations
- [ ] `impl ShaderBindable for T: bytemuck::Pod` (uniforms)
- [ ] `impl ShaderBindable for TextureView`
- [ ] `impl ShaderBindable for Sampler`
- [ ] `impl ShaderBindable for BufferPiece`
- [ ] Dynamic uniform buffer allocation in plain_data

#### Day 9: PipelineEncoder & Pass Recording
- [ ] Implement `with()` for binding pipeline to pass
- [ ] `bind()` method for ShaderData
- [ ] Vertex buffer binding
- [ ] Index buffer binding
- [ ] Viewport and scissor state

#### Day 10: Draw Commands & Submission
- [ ] `draw()`
- [ ] `draw_indexed()`
- [ ] Implement `submit()` - execute recorded commands
- [ ] Create bind groups from cache during submit
- [ ] Frame presentation

### Phase 5: Integration & Testing (Days 11-14)

#### Day 11: Compute Pipeline
- [ ] Compute pass recording
- [ ] `dispatch()`
- [ ] Storage buffer bindings
- [ ] Compute pipeline encoder

#### Day 12: Full Command Execution
- [ ] Complete command execution loop
- [ ] All render pass state (stencil reference, blend constants)
- [ ] Debug groups and labels
- [ ] Error handling and validation messages

#### Day 13: WASM Testing
- [ ] Test async initialization in browser
- [ ] Surface creation with canvas
- [ ] Frame presentation loop
- [ ] Verify no deadlocks (no pollster in WASM paths)

#### Day 14: Integration Testing
- [ ] Test with GPUI basic shapes
- [ ] Test with gpui-component examples
- [ ] Performance profiling
- [ ] Fix any coordinate system issues

---

## Part 4: Critical Implementation Details

### 4.1 Command Execution Pattern (Using RwLock)

```rust
impl Context {
    pub fn submit(&self, encoder: &mut CommandEncoder) -> SyncPoint {
        // 1. Sync dirty shadow buffers BEFORE creating command encoder
        self.sync_dirty_buffers();
        
        // 2. Take read lock for resource lookups
        let hub = self.hub.read().unwrap();
        
        // 3. Create wgpu command encoder
        let mut cmd_encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some(&encoder.name),
            }
        );
        
        // 4. Create uniform buffer for plain data (if any)
        let uniform_buffer = if !encoder.plain_data.is_empty() {
            Some(self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Plain Data Buffer"),
                contents: &encoder.plain_data,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }))
        } else {
            None
        };
        
        // 5. Execute recorded commands
        let mut current_render_pass: Option<wgpu::RenderPass> = None;
        let mut current_compute_pass: Option<wgpu::ComputePass> = None;
        
        // Track pending bindings for bind group creation
        let mut pending_bindings: Vec<PendingBinding> = Vec::new();
        let mut bind_group_cache = self.bind_group_cache.write().unwrap();
        
        for command in &encoder.commands {
            match command {
                Command::BeginRenderPass { color_attachments, depth_stencil, label } => {
                    // Build color attachments - keys MUST resolve or we have a bug
                    let color_views: Vec<Option<wgpu::RenderPassColorAttachment>> = 
                        color_attachments.iter()
                        .map(|att| {
                            let view = hub.texture_views.get(att.view_key)
                                .expect("Invalid texture view key in render pass");
                            Some(wgpu::RenderPassColorAttachment {
                                view,
                                resolve_target: att.resolve_target.map(|key| {
                                    hub.texture_views.get(key)
                                        .expect("Invalid resolve target key")
                                }),
                                ops: wgpu::Operations {
                                    load: att.load_op.clone(),
                                    store: att.store_op,
                                },
                            })
                        })
                        .collect();
                    
                    let depth_attachment = depth_stencil.as_ref().map(|ds| {
                        let view = hub.texture_views.get(ds.view_key)
                            .expect("Invalid depth texture view key");
                        wgpu::RenderPassDepthStencilAttachment {
                            view,
                            depth_ops: Some(wgpu::Operations {
                                load: ds.depth_load_op.clone(),
                                store: ds.depth_store_op,
                            }),
                            stencil_ops: Some(wgpu::Operations {
                                load: ds.stencil_load_op.clone(),
                                store: ds.stencil_store_op,
                            }),
                        }
                    });
                    
                    current_render_pass = Some(cmd_encoder.begin_render_pass(
                        &wgpu::RenderPassDescriptor {
                            label: Some(label),
                            color_attachments: &color_views,
                            depth_stencil_attachment: depth_attachment,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        }
                    ));
                }
                
                Command::SetRenderPipeline { key } => {
                    if let Some(ref mut pass) = current_render_pass {
                        let entry = hub.render_pipelines.get(*key)
                            .expect("Invalid render pipeline key");
                        pass.set_pipeline(&entry.raw);
                    }
                }
                
                Command::SetVertexBuffer { slot, buffer } => {
                    if let Some(ref mut pass) = current_render_pass {
                        let gpu_buffer = hub.buffers.get(buffer.key)
                            .expect("Invalid buffer key in set_vertex_buffer");
                        pass.set_vertex_buffer(
                            *slot,
                            gpu_buffer.gpu.slice(buffer.offset..)
                        );
                    }
                }
                
                Command::Draw { vertex_count, instance_count, first_vertex, first_instance } => {
                    if let Some(ref mut pass) = current_render_pass {
                        // Flush pending bindings to bind groups
                        self.flush_bindings(
                            &mut pending_bindings, 
                            &hub, 
                            &mut bind_group_cache,
                            uniform_buffer.as_ref(),
                        );
                        
                        pass.draw(
                            *first_vertex..(*first_vertex + *vertex_count),
                            *first_instance..(*first_instance + *instance_count)
                        );
                    }
                }
                
                Command::EndRenderPass => {
                    current_render_pass = None;
                }
                
                // ... handle other commands similarly
                
                _ => {}
            }
        }
        
        // 6. Finish and submit
        drop(hub);  // Release read lock before submit
        let submission_index = self.queue.submit(std::iter::once(cmd_encoder.finish()));
        
        // 7. Present any frames
        for frame in encoder.present_frames.drain(..) {
            frame.texture.present();
        }
        
        // 8. Clear encoder for reuse
        encoder.commands.clear();
        encoder.plain_data.clear();
        
        SyncPoint { submission_index }
    }
    
    pub fn wait_for(&self, sp: &SyncPoint, _timeout_ms: u32) -> bool {
        self.device.poll(wgpu::Maintain::WaitForSubmissionIndex(sp.submission_index));
        true  // wgpu poll blocks until complete
    }
    
    fn flush_bindings(
        &self,
        pending: &mut Vec<PendingBinding>,
        hub: &Hub,
        cache: &mut BindGroupCache,
        uniform_buffer: Option<&wgpu::Buffer>,
    ) {
        // Build bind groups from pending bindings using cache
        // This is where BindGroupCache::get_or_create is called
        pending.clear();
    }
}

struct PendingBinding {
    group: u32,
    binding: u32,
    resource: ResourceBinding,
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

## Part 5: Dependencies & Configuration

### 5.1 Cargo.toml Additions

```toml
[dependencies]
# Required for generational handle safety
slotmap = "1.0"

# wgpu with WebGPU backend support
wgpu = { version = "28", features = ["webgpu"] }

# For native async initialization
pollster = { version = "0.3", optional = true }

# For uniform data conversion
bytemuck = { version = "1.14", features = ["derive"] }

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
pollster = "0.3"

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen-futures = "0.4"
web-sys = { version = "0.3", features = ["HtmlCanvasElement"] }
```

### 5.2 Feature Flags

```toml
[features]
default = ["webgpu"]
webgpu = []
# When targeting WASM, webgpu is automatically enabled
```

---

## Part 6: Verification Checklist

### Trait Compliance
- [ ] `Buffer: Clone + Copy + Debug + Hash + PartialEq + Eq + Send + Sync`
- [ ] `Texture: Clone + Copy + Debug + Hash + PartialEq + Eq + Send + Sync`
- [ ] `TextureView: Clone + Copy + Debug + Hash + PartialEq + Eq + Send + Sync`
- [ ] `Sampler: Clone + Copy + Debug + Hash + PartialEq + Eq + Send + Sync`
- [ ] `ResourceDevice` trait implemented with `#[hidden_trait::expose]`
- [ ] `ShaderDevice` trait implemented
- [ ] `CommandDevice` trait implemented
- [ ] All encoder traits implemented

### Generational Safety (slotmap)
- [ ] Dead keys return `None` from Hub lookups
- [ ] Type-safe keys (BufferKey cannot be used as TextureKey)
- [ ] No ABA problem: reused slots have new generation

### Concurrency (RwLock)
- [ ] Command recording uses `read()` only
- [ ] Resource creation/destruction uses `write()`
- [ ] No deadlocks between read and write locks
- [ ] Shadow buffer sync uses proper lock ordering

### Shadow Memory
- [ ] Upload buffers have CPU shadow memory
- [ ] Dirty tracking for modified buffers
- [ ] Sync to GPU before submit
- [ ] Shared buffer limitations documented/enforced

### Bind Group Cache
- [ ] Cache key includes all bound resources
- [ ] Invalidation on resource destruction
- [ ] LRU eviction for size management
- [ ] No stale bind groups after resource destruction

### API Compatibility
- [ ] `CommandEncoder::transfer()` returns `TransferCommandEncoder`
- [ ] `CommandEncoder::compute()` returns `ComputeCommandEncoder`
- [ ] `CommandEncoder::render()` returns `RenderCommandEncoder`
- [ ] `PassEncoder::with()` returns `PipelineEncoder`
- [ ] `PipelineEncoder::bind()` accepts `ShaderData`
- [ ] All `ShaderBindable` types work correctly

### WASM Compatibility
- [ ] No `pollster::block_on` in WASM code paths
- [ ] Async initialization via `wasm_bindgen_futures`
- [ ] Surface creation works with canvas element
- [ ] Frame presentation works in browser
- [ ] No thread blocking operations

---

## Part 7: Risk Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| ABA Problem (use-after-free) | **Eliminated** | Critical | Using slotmap with generational indices |
| Lock Contention | Low | Medium | RwLock allows concurrent reads; write operations are infrequent |
| Bind Group Explosion | Medium | Medium | LRU cache with size limit, invalidation on destroy |
| Shadow Memory Overhead | Low | Low | Only allocated for Upload/Shared buffers |
| WASM Async Complexity | Medium | High | Explicit async paths, no pollster in WASM |
| ShaderData Mapping Errors | Medium | High | Extensive validation, clear error messages |
| Stale Resources After Destroy | **Eliminated** | Critical | slotmap returns None for dead keys |

---

## Part 8: Summary

### Key Architectural Decisions

1. **slotmap for Resource Handles**
   - Generational indices prevent use-after-free
   - Type-safe keys prevent cross-resource confusion
   - O(1) lookup with minimal overhead

2. **RwLock for Concurrency**
   - Read access for command recording (high frequency)
   - Write access for resource management (low frequency)
   - No serialization of command recording across threads

3. **Shadow Memory for Upload Buffers**
   - CPU-side buffer for immediate writes
   - Sync to GPU on submit
   - Dirty tracking to avoid unnecessary copies

4. **Bind Group Caching**
   - Avoid recreation overhead
   - Proper invalidation on resource destruction
   - LRU eviction for memory management

### Implementation Order

```
Day 1:  Hub + slotmap + RwLock (CRITICAL FOUNDATION)
Day 2:  CommandEncoder + resource lookup verification
Day 3:  Complete resource creation + shadow memory
Day 4:  Transfer commands
Day 5:  Shader & layout infrastructure
Day 6:  Pipeline creation
Day 7:  Bind group cache
Day 8:  ShaderBindable implementations
Day 9:  PipelineEncoder & pass recording
Day 10: Draw commands & submission
Day 11: Compute pipeline
Day 12: Full command execution
Day 13: WASM testing
Day 14: Integration testing
```

**Status:** ✅ **APPROVED FOR IMPLEMENTATION**

The architecture is sound, with proper safety guarantees from slotmap and good concurrency characteristics from RwLock. The shadow memory pattern handles the wgpu/Blade API mismatch cleanly.
