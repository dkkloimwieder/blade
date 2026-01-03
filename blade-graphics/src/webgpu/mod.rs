//! WebGPU backend for blade-graphics
//!
//! Uses wgpu with slotmap-based handles for Copy semantics.

mod command;
mod pipeline;
mod platform;
mod resource;
mod surface;

use slotmap::{new_key_type, SlotMap};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;
use std::marker::PhantomData;

pub use command::PipelineContext;
pub use platform::PlatformError;

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
    /// Key for render pipeline resources
    pub struct RenderPipelineKey;
    /// Key for compute pipeline resources
    pub struct ComputePipelineKey;
    /// Key for bind group layout resources
    pub struct BindGroupLayoutKey;
}

//=============================================================================
// Internal Configuration
//=============================================================================

#[derive(Clone, Debug)]
struct Limits {
    uniform_buffer_alignment: u32,
    max_bind_groups: u32,
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
    _phantom: PhantomData<()>,
}

//=============================================================================
// Internal Storage Entry Types
//=============================================================================

/// Internal buffer entry with shadow memory
struct BufferEntry {
    gpu: wgpu::Buffer,
    /// CPU shadow memory for Upload/Shared buffers
    shadow: Option<Box<[u8]>>,
    /// True if shadow memory is dirty and needs sync.
    /// Using AtomicBool allows marking dirty without write lock.
    dirty: AtomicBool,
}

/// Internal texture entry
struct TextureEntry {
    gpu: wgpu::Texture,
    format: crate::TextureFormat,
}

/// Internal render pipeline entry
struct RenderPipelineEntry {
    raw: wgpu::RenderPipeline,
    group_mappings: Box<[ShaderDataMapping]>,
    bind_group_layouts: Vec<wgpu::BindGroupLayout>,
    topology: crate::PrimitiveTopology,
}

/// Internal compute pipeline entry
struct ComputePipelineEntry {
    raw: wgpu::ComputePipeline,
    group_mappings: Box<[ShaderDataMapping]>,
    bind_group_layouts: Vec<wgpu::BindGroupLayout>,
    wg_size: [u32; 3],
}

//=============================================================================
// Shader Data Binding
//=============================================================================

/// Maps a logical binding index to a WebGPU binding slot
#[derive(Clone, Debug)]
struct BindingSlot {
    group: u32,
    binding: u32,
}

/// List of binding slots for one shader data binding
type SlotList = Vec<BindingSlot>;

/// Mapping from ShaderDataLayout to WebGPU bind group structure
struct ShaderDataMapping {
    /// For each binding in the ShaderDataLayout, the target slot(s)
    targets: Box<[SlotList]>,
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
    device_information: crate::DeviceInformation,
    limits: Limits,
}

impl Context {
    /// Get device information
    pub fn info(&self) -> &crate::DeviceInformation {
        &self.device_information
    }

    /// Get capabilities (WebGPU has limited feature set compared to Vulkan)
    pub fn capabilities(&self) -> crate::Capabilities {
        crate::Capabilities {
            // WebGPU doesn't support ray tracing in the base spec
            ray_query: crate::ShaderVisibility::empty(),
            // WebGPU supports 1 and 4 samples typically
            sample_count_mask: 0b0101, // 1 and 4
        }
    }

    /// Mark a buffer as dirty (called when user writes to shadow memory)
    pub fn mark_buffer_dirty(&self, buffer: Buffer) {
        let hub = self.hub.read().unwrap();
        if let Some(entry) = hub.buffers.get(buffer.raw) {
            entry.dirty.store(true, Ordering::Release);
        }
    }

    /// Sync all dirty shadow buffers to GPU.
    /// MUST be called exactly once, immediately before queue.submit()
    fn sync_dirty_buffers(&self) {
        let hub = self.hub.read().unwrap();
        for (_key, entry) in hub.buffers.iter() {
            // Use Acquire ordering to see all writes to shadow memory
            if entry.dirty.load(Ordering::Acquire) {
                if let Some(ref shadow) = entry.shadow {
                    self.queue.write_buffer(&entry.gpu, 0, shadow);
                }
                // Clear dirty flag with Release ordering
                entry.dirty.store(false, Ordering::Release);
            }
        }
    }
}

//=============================================================================
// Surface & Frame
//=============================================================================

pub struct Surface {
    raw: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    format: crate::TextureFormat,
}

impl Surface {
    /// Reconfigure surface after window resize
    pub fn reconfigure(&mut self, ctx: &Context, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.raw.configure(&ctx.device, &self.config);
    }
}

#[derive(Debug)]
pub struct Frame {
    texture: wgpu::SurfaceTexture,
    view: wgpu::TextureView,
    /// Key for the temporary view in hub (if needed for binding)
    view_key: Option<TextureViewKey>,
    target_size: [u16; 2],
    format: crate::TextureFormat,
}

impl Frame {
    pub fn texture(&self) -> Texture {
        Texture {
            raw: TextureKey::default(),
            format: self.format,
            target_size: self.target_size,
        }
    }

    pub fn texture_view(&self) -> TextureView {
        TextureView {
            raw: self.view_key.unwrap_or_default(),
            target_size: self.target_size,
            aspects: crate::TexelAspects::COLOR,
        }
    }
}

//=============================================================================
// Pipeline Types
//=============================================================================

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
    #[allow(dead_code)]
    topology: crate::PrimitiveTopology,
}

//=============================================================================
// Sync Point
//=============================================================================

#[derive(Clone, Debug)]
pub struct SyncPoint {
    submission_index: wgpu::SubmissionIndex,
}
