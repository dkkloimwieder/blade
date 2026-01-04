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

pub use command::{
    CommandEncoder, ComputeCommandEncoder, PassEncoder, PipelineContext, PipelineEncoder,
    RenderCommandEncoder, TransferCommandEncoder,
};
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
#[derive(Clone)]
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
// Bind Group Cache with Leak Prevention
//=============================================================================

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

/// Key for bind group cache - identifies a unique combination of resources
#[derive(Clone, Debug)]
pub(super) struct BindGroupCacheKey {
    /// Pipeline key (render or compute) for layout lookup
    pipeline_key: PipelineKey,
    /// Group index within the pipeline
    group_index: u32,
    /// Sorted list of resource bindings
    bindings: Vec<ResourceBinding>,
}

impl PartialEq for BindGroupCacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.pipeline_key == other.pipeline_key
            && self.group_index == other.group_index
            && self.bindings == other.bindings
    }
}

impl Eq for BindGroupCacheKey {}

impl Hash for BindGroupCacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.pipeline_key.hash(state);
        self.group_index.hash(state);
        self.bindings.hash(state);
    }
}

/// Identifies either a render or compute pipeline
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(super) enum PipelineKey {
    Render(RenderPipelineKey),
    Compute(ComputePipelineKey),
}

/// A resource bound to a bind group
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(super) enum ResourceBinding {
    Buffer { binding: u32, key: BufferKey, offset: u64, size: u64 },
    TextureView { binding: u32, key: TextureViewKey },
    Sampler { binding: u32, key: SamplerKey },
    /// Uniform data from plain_data buffer (ephemeral, keyed by content hash)
    PlainData { binding: u32, offset: u32, size: u32 },
}

/// Tracks which BindGroups depend on which resources
struct DependencyTracker {
    /// Buffer -> Set of BindGroupCacheKeys that use it
    buffer_deps: HashMap<BufferKey, HashSet<BindGroupCacheKey>>,
    /// TextureView -> Set of BindGroupCacheKeys that use it
    texture_view_deps: HashMap<TextureViewKey, HashSet<BindGroupCacheKey>>,
    /// Sampler -> Set of BindGroupCacheKeys that use it
    sampler_deps: HashMap<SamplerKey, HashSet<BindGroupCacheKey>>,
}

impl DependencyTracker {
    fn new() -> Self {
        Self {
            buffer_deps: HashMap::new(),
            texture_view_deps: HashMap::new(),
            sampler_deps: HashMap::new(),
        }
    }

    /// Register that a bind group uses these resources
    fn register(&mut self, cache_key: &BindGroupCacheKey) {
        for binding in &cache_key.bindings {
            match binding {
                ResourceBinding::Buffer { key, .. } => {
                    self.buffer_deps
                        .entry(*key)
                        .or_default()
                        .insert(cache_key.clone());
                }
                ResourceBinding::TextureView { key, .. } => {
                    self.texture_view_deps
                        .entry(*key)
                        .or_default()
                        .insert(cache_key.clone());
                }
                ResourceBinding::Sampler { key, .. } => {
                    self.sampler_deps
                        .entry(*key)
                        .or_default()
                        .insert(cache_key.clone());
                }
                ResourceBinding::PlainData { .. } => {
                    // Ephemeral, not tracked
                }
            }
        }
    }

    /// Get all bind groups that depend on a buffer
    fn get_buffer_dependents(&self, key: BufferKey) -> Option<&HashSet<BindGroupCacheKey>> {
        self.buffer_deps.get(&key)
    }

    /// Remove tracking for a buffer (called after invalidation)
    fn remove_buffer(&mut self, key: BufferKey) {
        self.buffer_deps.remove(&key);
    }

    /// Get all bind groups that depend on a texture view
    fn get_texture_view_dependents(&self, key: TextureViewKey) -> Option<&HashSet<BindGroupCacheKey>> {
        self.texture_view_deps.get(&key)
    }

    fn remove_texture_view(&mut self, key: TextureViewKey) {
        self.texture_view_deps.remove(&key);
    }

    fn get_sampler_dependents(&self, key: SamplerKey) -> Option<&HashSet<BindGroupCacheKey>> {
        self.sampler_deps.get(&key)
    }

    fn remove_sampler(&mut self, key: SamplerKey) {
        self.sampler_deps.remove(&key);
    }
}

/// LRU cache for bind groups with dependency tracking
pub(super) struct BindGroupCache {
    /// Map from cache key to wgpu bind group
    groups: HashMap<BindGroupCacheKey, wgpu::BindGroup>,
    /// Dependency tracking for invalidation
    deps: DependencyTracker,
    /// Maximum cache size before LRU eviction
    max_size: usize,
    /// Access order for LRU (most recent at back)
    access_order: Vec<BindGroupCacheKey>,
    /// Cache stats
    hits: u64,
    misses: u64,
}

impl BindGroupCache {
    fn new(max_size: usize) -> Self {
        Self {
            groups: HashMap::new(),
            deps: DependencyTracker::new(),
            max_size,
            access_order: Vec::new(),
            hits: 0,
            misses: 0,
        }
    }

    /// Get cached bind group or create a new one
    pub fn get_or_create<F>(&mut self, key: BindGroupCacheKey, create_fn: F) -> &wgpu::BindGroup
    where
        F: FnOnce() -> wgpu::BindGroup,
    {
        if self.groups.contains_key(&key) {
            // Cache hit - update LRU order
            self.hits += 1;
            self.access_order.retain(|k| k != &key);
            self.access_order.push(key.clone());
            return self.groups.get(&key).unwrap();
        }

        // Cache miss - create new bind group
        self.misses += 1;
        let bind_group = create_fn();

        // Register dependencies BEFORE inserting
        self.deps.register(&key);

        // Evict if over capacity
        while self.groups.len() >= self.max_size {
            self.evict_lru();
        }

        self.access_order.push(key.clone());
        self.groups.insert(key.clone(), bind_group);
        self.groups.get(&key).unwrap()
    }

    /// CRITICAL: Called when a buffer is destroyed
    /// Must be called BEFORE removing from Hub to ensure bind groups are dropped first
    pub fn invalidate_buffer(&mut self, key: BufferKey) {
        if let Some(dependents) = self.deps.get_buffer_dependents(key).cloned() {
            for cache_key in dependents {
                self.groups.remove(&cache_key);
                self.access_order.retain(|k| k != &cache_key);
            }
        }
        self.deps.remove_buffer(key);
    }

    /// Called when a texture view is destroyed
    pub fn invalidate_texture_view(&mut self, key: TextureViewKey) {
        if let Some(dependents) = self.deps.get_texture_view_dependents(key).cloned() {
            for cache_key in dependents {
                self.groups.remove(&cache_key);
                self.access_order.retain(|k| k != &cache_key);
            }
        }
        self.deps.remove_texture_view(key);
    }

    /// Called when a sampler is destroyed
    pub fn invalidate_sampler(&mut self, key: SamplerKey) {
        if let Some(dependents) = self.deps.get_sampler_dependents(key).cloned() {
            for cache_key in dependents {
                self.groups.remove(&cache_key);
                self.access_order.retain(|k| k != &cache_key);
            }
        }
        self.deps.remove_sampler(key);
    }

    fn evict_lru(&mut self) {
        if let Some(oldest) = self.access_order.first().cloned() {
            self.groups.remove(&oldest);
            self.access_order.remove(0);
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> (u64, u64, usize) {
        (self.hits, self.misses, self.groups.len())
    }
}

//=============================================================================
// Context
//=============================================================================

/// Ring buffer for per-frame uniform data
/// Uses triple buffering to avoid GPU/CPU contention with write_buffer
const UNIFORM_BUFFER_COUNT: usize = 3;

pub(super) struct UniformBuffer {
    /// Ring of GPU buffers for triple buffering
    buffers: [Option<wgpu::Buffer>; UNIFORM_BUFFER_COUNT],
    /// Current buffer capacity in bytes (all buffers same size)
    capacity: u64,
    /// Current buffer index in the ring
    current_index: usize,
}

impl UniformBuffer {
    fn new() -> Self {
        Self {
            buffers: [None, None, None],
            capacity: 0,
            current_index: 0,
        }
    }

    /// Get buffer for current frame, creating/resizing if needed
    /// Uses ring buffer to avoid GPU stalls from write_buffer
    fn ensure_capacity(&mut self, device: &wgpu::Device, size: u64) -> &wgpu::Buffer {
        // Rotate to next buffer in ring
        self.current_index = (self.current_index + 1) % UNIFORM_BUFFER_COUNT;

        // Grow all buffers if needed
        if self.capacity < size {
            let new_capacity = (size.max(256)).next_power_of_two();
            for i in 0..UNIFORM_BUFFER_COUNT {
                self.buffers[i] = Some(device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&format!("Uniform Buffer Ring {}", i)),
                    size: new_capacity,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
            }
            self.capacity = new_capacity;
            log::debug!("Created uniform buffer ring with capacity {} bytes each", new_capacity);
        } else if self.buffers[self.current_index].is_none() {
            // Create just this buffer if ring not fully initialized
            self.buffers[self.current_index] = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("Uniform Buffer Ring {}", self.current_index)),
                size: self.capacity,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }

        self.buffers[self.current_index].as_ref().unwrap()
    }
}

pub struct Context {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    /// RwLock allows concurrent read access during command recording
    /// Arc-wrapped so Surface can share access for frame view management
    hub: std::sync::Arc<RwLock<Hub>>,
    device_information: crate::DeviceInformation,
    limits: Limits,
    /// Cache for bind groups to avoid recreation
    bind_group_cache: RwLock<BindGroupCache>,
    /// Reusable buffer for per-frame uniform data (avoids create_buffer_init every frame)
    uniform_buffer: RwLock<UniformBuffer>,
}

impl Context {
    /// Initialize a new WebGPU context (native).
    ///
    /// # Safety
    ///
    /// This is marked unsafe for API consistency with other backends.
    /// WebGPU itself is safe.
    #[cfg(not(target_arch = "wasm32"))]
    pub unsafe fn init(desc: crate::ContextDesc) -> Result<Self, crate::NotSupportedError> {
        platform::create_context(&desc).map_err(|e| crate::NotSupportedError::Platform(e))
    }

    /// Initialize a new WebGPU context asynchronously (WASM).
    ///
    /// On WASM, WebGPU initialization is inherently async due to browser APIs.
    /// Use this method instead of `init()` for WASM targets.
    #[cfg(target_arch = "wasm32")]
    pub async fn init_async(desc: crate::ContextDesc) -> Result<Self, crate::NotSupportedError> {
        platform::create_context(&desc)
            .await
            .map_err(|e| crate::NotSupportedError::Platform(e))
    }

    /// Get device information
    pub fn device_information(&self) -> &crate::DeviceInformation {
        &self.device_information
    }

    /// Get capabilities (WebGPU has limited feature set compared to Vulkan)
    pub fn capabilities(&self) -> crate::Capabilities {
        crate::Capabilities {
            // WebGPU doesn't support ray tracing in the base spec
            ray_query: crate::ShaderVisibility::empty(),
            // WebGPU supports 1 and 4 samples typically
            sample_count_mask: 0b0101, // 1 and 4
            // WebGPU doesn't support dual-source blending in the base spec
            dual_source_blending: false,
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
    /// Shared hub reference for frame view management
    hub: std::sync::Arc<RwLock<Hub>>,
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
    /// Key for the frame's texture view in the hub
    view_key: Option<TextureViewKey>,
    target_size: [u16; 2],
    format: crate::TextureFormat,
}

// SAFETY: On WASM, WebGPU is inherently single-threaded, so Send+Sync are safe.
// On native with wgpu, these types are already Send+Sync.
#[cfg(target_arch = "wasm32")]
unsafe impl Send for Frame {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for Frame {}

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
    group_mappings: Box<[ShaderDataMapping]>,
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
    group_mappings: Box<[ShaderDataMapping]>,
}

//=============================================================================
// Sync Point
//=============================================================================

#[derive(Clone, Debug)]
pub struct SyncPoint {
    submission_index: wgpu::SubmissionIndex,
}
