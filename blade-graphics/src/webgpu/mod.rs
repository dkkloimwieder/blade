//! WebGPU backend for blade-graphics
//!
//! Uses wgpu with slotmap-based handles for Copy semantics.

mod command;
mod pipeline;
mod platform;
mod resource;
mod surface;

use slotmap::{new_key_type, SlotMap};
use std::ops::Range;
use std::sync::{Mutex, RwLock};
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
    /// Whether GPU timing queries are supported
    timing_supported: bool,
}

//=============================================================================
// Constants
//=============================================================================

/// Maximum bind groups cached before LRU eviction
pub(super) const BIND_GROUP_CACHE_SIZE: usize = 1024;

/// WebGPU requires bytes_per_row to be a multiple of 256 for texture copies
pub(super) const BYTES_PER_ROW_ALIGNMENT: u32 = 256;

//=============================================================================
// GPU Timing Infrastructure
//=============================================================================

/// Maximum number of passes that can be timed per frame
const MAX_TIMING_PASSES: u32 = 64;

/// Number of timing frames in the ring buffer (triple buffering)
const TIMING_RING_SIZE: usize = 3;

/// Ring buffer entry for timing data
struct TimingFrame {
    /// Query set for this frame's timestamps (2 queries per pass: begin + end)
    query_set: wgpu::QuerySet,
    /// Buffer to resolve query results into
    resolve_buffer: wgpu::Buffer,
    /// Mappable buffer for CPU readback
    readback_buffer: wgpu::Buffer,
    /// Pass names for this frame (indexed by query pair)
    pass_names: Vec<String>,
    /// Number of passes recorded this frame
    pass_count: u32,
}

impl TimingFrame {
    fn new(device: &wgpu::Device) -> Self {
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("Blade Timing Query Set"),
            ty: wgpu::QueryType::Timestamp,
            count: MAX_TIMING_PASSES * 2, // 2 timestamps per pass (begin + end)
        });

        // Each timestamp is a u64 (8 bytes)
        let buffer_size = (MAX_TIMING_PASSES * 2 * 8) as u64;

        let resolve_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Blade Timing Resolve Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Blade Timing Readback Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self {
            query_set,
            resolve_buffer,
            readback_buffer,
            pass_names: Vec::with_capacity(MAX_TIMING_PASSES as usize),
            pass_count: 0,
        }
    }

    fn reset(&mut self) {
        self.pass_names.clear();
        self.pass_count = 0;
    }
}

/// GPU timing query pool with async readback
pub(super) struct TimingQueryPool {
    /// Ring buffer of timing frames
    frames: Option<[TimingFrame; TIMING_RING_SIZE]>,
    /// Current frame index in ring
    current_frame: usize,
    /// Timestamp period in nanoseconds per tick (from queue)
    timestamp_period: f32,
    /// Collected timing results from previous frame (native only)
    #[cfg(not(target_arch = "wasm32"))]
    results: Vec<(String, std::time::Duration)>,
}

impl TimingQueryPool {
    pub fn new() -> Self {
        Self {
            frames: None,
            current_frame: 0,
            timestamp_period: 1.0, // Default, will be set from queue
            #[cfg(not(target_arch = "wasm32"))]
            results: Vec::new(),
        }
    }

    /// Initialize the query pool (called when timing is enabled)
    pub fn init(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.frames.is_some() {
            return; // Already initialized
        }

        self.frames = Some([
            TimingFrame::new(device),
            TimingFrame::new(device),
            TimingFrame::new(device),
        ]);
        self.timestamp_period = queue.get_timestamp_period();
        log::info!(
            "Initialized GPU timing with period {} ns/tick",
            self.timestamp_period
        );
    }

    /// Get the current frame's query set and allocate a pass slot
    pub fn begin_pass(&mut self, name: &str) -> Option<(u32, &wgpu::QuerySet)> {
        let frames = self.frames.as_mut()?;
        let frame = &mut frames[self.current_frame];

        if frame.pass_count >= MAX_TIMING_PASSES {
            log::warn!("Exceeded max timing passes per frame ({})", MAX_TIMING_PASSES);
            return None;
        }

        let query_index = frame.pass_count * 2; // 2 queries per pass
        frame.pass_names.push(name.to_string());
        frame.pass_count += 1;

        Some((query_index, &frame.query_set))
    }

    /// Resolve queries after command buffer submission
    pub fn resolve(&self, encoder: &mut wgpu::CommandEncoder) {
        let Some(ref frames) = self.frames else { return };
        let frame = &frames[self.current_frame];

        if frame.pass_count == 0 {
            return;
        }

        let query_count = frame.pass_count * 2;

        // Resolve query results to GPU buffer
        encoder.resolve_query_set(&frame.query_set, 0..query_count, &frame.resolve_buffer, 0);

        // Copy to mappable readback buffer
        encoder.copy_buffer_to_buffer(
            &frame.resolve_buffer,
            0,
            &frame.readback_buffer,
            0,
            (query_count * 8) as u64,
        );
    }

    /// Advance to next frame and process previous frame's results
    #[cfg(not(target_arch = "wasm32"))]
    pub fn advance_frame(&mut self) {
        let Some(ref mut frames) = self.frames else { return };

        // Move to next frame
        self.current_frame = (self.current_frame + 1) % TIMING_RING_SIZE;

        // Try to read back results from oldest frame (2 frames ago)
        let readback_index = (self.current_frame + 1) % TIMING_RING_SIZE;
        let readback_frame = &mut frames[readback_index];

        if readback_frame.pass_count == 0 {
            readback_frame.reset();
            return;
        }

        // Map and read the buffer (blocking on native)
        let slice = readback_frame.readback_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });

        // Collect data we need before processing
        let pass_names: Vec<String> = readback_frame.pass_names.clone();
        let period_ns = self.timestamp_period as f64;

        // Poll until mapped (blocking)
        // Note: In production, this should be non-blocking with device.poll()
        if rx.recv().ok().and_then(|r| r.ok()).is_some() {
            let data = slice.get_mapped_range();
            let timestamps: &[u64] = bytemuck::cast_slice(&data);

            // Process timing data directly here
            self.results.clear();
            for (i, name) in pass_names.iter().enumerate() {
                let begin_idx = i * 2;
                let end_idx = begin_idx + 1;
                if end_idx < timestamps.len() {
                    let begin_tick = timestamps[begin_idx];
                    let end_tick = timestamps[end_idx];
                    let duration_ns = ((end_tick - begin_tick) as f64 * period_ns) as u64;
                    self.results.push((name.clone(), std::time::Duration::from_nanos(duration_ns)));
                }
            }

            drop(data);
            readback_frame.readback_buffer.unmap();
        }

        readback_frame.reset();
    }

    /// Advance to next frame (WASM version)
    ///
    /// Note: GPU timing queries on WASM require browser flags to be enabled
    /// (e.g., --enable-dawn-features=allow_unsafe_apis in Chrome).
    /// Even when enabled, async buffer mapping for readback is complex.
    /// For now, we skip readback and timing results will be empty on WASM.
    #[cfg(target_arch = "wasm32")]
    pub fn advance_frame(&mut self) {
        let Some(ref mut frames) = self.frames else { return };

        // Move to next frame
        self.current_frame = (self.current_frame + 1) % TIMING_RING_SIZE;

        // Get index of frame to read back (2 frames ago for triple buffering)
        let readback_index = (self.current_frame + 1) % TIMING_RING_SIZE;
        let readback_frame = &mut frames[readback_index];

        if readback_frame.pass_count == 0 {
            readback_frame.reset();
            return;
        }

        // On WASM, GPU timing queries require special browser flags and async
        // buffer mapping is complex. Skip readback for now.
        // Users should use browser DevTools Performance tab for GPU profiling.
        readback_frame.reset();
    }

    /// Get timing results from the most recently completed frame
    #[cfg(not(target_arch = "wasm32"))]
    pub fn results(&self) -> &[(String, std::time::Duration)] {
        &self.results
    }

    /// Get timing results (WASM version)
    ///
    /// Returns empty - WASM timing readback not implemented.
    /// Use browser DevTools Performance tab for GPU profiling.
    #[cfg(target_arch = "wasm32")]
    pub fn results(&self) -> &[(String, std::time::Duration)] {
        &[] // Timing readback not available on WASM
    }
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
    /// Dirty byte range that needs sync to GPU.
    /// None = clean, Some(range) = dirty region.
    /// Ranges are merged on overlap for efficient uploads.
    dirty_range: Mutex<Option<Range<u64>>>,
}

/// Internal texture entry
struct TextureEntry {
    gpu: wgpu::Texture,
}

/// Internal render pipeline entry
struct RenderPipelineEntry {
    raw: wgpu::RenderPipeline,
    bind_group_layouts: Vec<wgpu::BindGroupLayout>,
}

/// Internal compute pipeline entry
struct ComputePipelineEntry {
    raw: wgpu::ComputePipeline,
    bind_group_layouts: Vec<wgpu::BindGroupLayout>,
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
    /// List of resource bindings
    bindings: Vec<ResourceBinding>,
}

impl BindGroupCacheKey {
    /// Create a new cache key from an iterator of bindings
    pub fn new<I>(pipeline_key: PipelineKey, group_index: u32, bindings_iter: I) -> Self
    where
        I: Iterator<Item = ResourceBinding>,
    {
        Self {
            pipeline_key,
            group_index,
            bindings: bindings_iter.collect(),
        }
    }
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
    /// Uniform data with dynamic offset - keyed by buffer_index and size, NOT offset
    /// The offset is passed as a dynamic offset to set_bind_group
    PlainDataDynamic { binding: u32, buffer_index: usize, size: u32 },
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
            match *binding {
                ResourceBinding::Buffer { key, .. } => {
                    self.buffer_deps
                        .entry(key)
                        .or_default()
                        .insert(cache_key.clone());
                }
                ResourceBinding::TextureView { key, .. } => {
                    self.texture_view_deps
                        .entry(key)
                        .or_default()
                        .insert(cache_key.clone());
                }
                ResourceBinding::Sampler { key, .. } => {
                    self.sampler_deps
                        .entry(key)
                        .or_default()
                        .insert(cache_key.clone());
                }
                ResourceBinding::PlainData { .. } | ResourceBinding::PlainDataDynamic { .. } => {
                    // Ephemeral, not tracked (uniform buffer is internal)
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

/// Bind group cache with dependency tracking
/// Uses FIFO eviction (simpler than LRU, eviction is rare for small caches)
pub(super) struct BindGroupCache {
    /// Map from cache key to bind group
    groups: HashMap<BindGroupCacheKey, wgpu::BindGroup>,
    /// Dependency tracking for invalidation
    deps: DependencyTracker,
    /// Maximum cache size before eviction
    max_size: usize,
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
            hits: 0,
            misses: 0,
        }
    }

    /// Get cached bind group or create a new one
    /// O(1) for cache hits (single hash lookup)
    pub fn get_or_create<F>(&mut self, key: BindGroupCacheKey, create_fn: F) -> &wgpu::BindGroup
    where
        F: FnOnce() -> wgpu::BindGroup,
    {
        // Fast path: check if exists (one hash lookup)
        if self.groups.contains_key(&key) {
            self.hits += 1;
            return self.groups.get(&key).unwrap();
        }

        // Cache miss
        self.misses += 1;

        // Register dependencies
        self.deps.register(&key);

        // Evict if over capacity
        while self.groups.len() >= self.max_size {
            self.evict_one();
        }

        // Insert and return
        self.groups.insert(key.clone(), create_fn());
        self.groups.get(&key).unwrap()
    }

    /// CRITICAL: Called when a buffer is destroyed
    /// Must be called BEFORE removing from Hub to ensure bind groups are dropped first
    pub fn invalidate_buffer(&mut self, key: BufferKey) {
        if let Some(dependents) = self.deps.get_buffer_dependents(key).cloned() {
            for cache_key in dependents {
                self.groups.remove(&cache_key);
            }
        }
        self.deps.remove_buffer(key);
    }

    /// Called when a texture view is destroyed
    pub fn invalidate_texture_view(&mut self, key: TextureViewKey) {
        if let Some(dependents) = self.deps.get_texture_view_dependents(key).cloned() {
            for cache_key in dependents {
                self.groups.remove(&cache_key);
            }
        }
        self.deps.remove_texture_view(key);
    }

    /// Called when a sampler is destroyed
    pub fn invalidate_sampler(&mut self, key: SamplerKey) {
        if let Some(dependents) = self.deps.get_sampler_dependents(key).cloned() {
            for cache_key in dependents {
                self.groups.remove(&cache_key);
            }
        }
        self.deps.remove_sampler(key);
    }

    /// Evict one entry (arbitrary - HashMap iteration order)
    /// Only called when cache is full, which is rare
    fn evict_one(&mut self) {
        if let Some(key) = self.groups.keys().next().cloned() {
            self.groups.remove(&key);
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
    /// Returns (buffer, buffer_index) for use in cache keying
    fn ensure_capacity(&mut self, device: &wgpu::Device, size: u64) -> (&wgpu::Buffer, usize) {
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

        let index = self.current_index;
        (self.buffers[self.current_index].as_ref().unwrap(), index)
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
    /// GPU timing query pool (lazily initialized when timing is requested)
    timing_pool: RwLock<TimingQueryPool>,
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
        platform::create_context(&desc).map_err(crate::NotSupportedError::Platform)
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

    /// Mark a buffer as dirty (entire buffer).
    /// Called when user writes to shadow memory via buffer.data().
    pub fn mark_buffer_dirty(&self, buffer: Buffer) {
        self.mark_buffer_dirty_range(buffer, 0, buffer.size);
    }

    /// Mark a specific range of a buffer as dirty.
    /// More efficient than marking the entire buffer when only a portion changed.
    ///
    /// # Arguments
    /// * `buffer` - The buffer to mark dirty
    /// * `offset` - Byte offset from start of buffer
    /// * `size` - Number of bytes that changed
    ///
    /// # Example
    /// ```ignore
    /// // Write to a specific region of the buffer
    /// unsafe {
    ///     let ptr = buffer.data().add(offset as usize);
    ///     std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
    /// }
    /// // Only sync the modified region
    /// context.sync_buffer_range(buffer, offset, data.len() as u64);
    /// ```
    pub fn sync_buffer_range(&self, buffer: Buffer, offset: u64, size: u64) {
        self.mark_buffer_dirty_range(buffer, offset, size);
    }

    /// Internal: mark a specific range as dirty (will be synced at next submit)
    fn mark_buffer_dirty_range(&self, buffer: Buffer, offset: u64, size: u64) {
        let hub = self.hub.read().unwrap();
        if let Some(entry) = hub.buffers.get(buffer.raw) {
            let new_range = offset..(offset + size);
            let mut dirty = entry.dirty_range.lock().unwrap();
            *dirty = Some(match dirty.take() {
                None => new_range,
                Some(existing) => {
                    // Merge overlapping or adjacent ranges
                    let start = existing.start.min(new_range.start);
                    let end = existing.end.max(new_range.end);
                    start..end
                }
            });
        }
    }

    /// Sync all dirty shadow buffers to GPU.
    /// MUST be called exactly once, immediately before queue.submit()
    /// Only uploads the dirty byte ranges, not entire buffers.
    fn sync_dirty_buffers(&self) {
        let hub = self.hub.read().unwrap();
        for (_key, entry) in hub.buffers.iter() {
            let range = entry.dirty_range.lock().unwrap().take();
            if let Some(range) = range {
                if let Some(ref shadow) = entry.shadow {
                    // Only upload the dirty range
                    let start = range.start as usize;
                    let end = (range.end as usize).min(shadow.len());
                    if start < end {
                        self.queue.write_buffer(&entry.gpu, range.start, &shadow[start..end]);
                    }
                }
            }
        }
    }

    /// Check if GPU timing is supported
    pub fn timing_supported(&self) -> bool {
        self.limits.timing_supported
    }

    /// Get GPU pass timing results from the most recently completed frame.
    ///
    /// Results are available with a 2-frame delay due to triple buffering.
    /// Returns empty slice if timing is not enabled or not yet available.
    pub fn timing_results(&self) -> Vec<(String, std::time::Duration)> {
        let pool = self.timing_pool.read().unwrap();
        pool.results().to_vec()
    }

    /// Get bind group cache statistics: (hits, misses, current_size)
    pub fn cache_stats(&self) -> (u64, u64, usize) {
        let cache = self.bind_group_cache.read().unwrap();
        cache.stats()
    }

    /// Initialize timing pool if timing is enabled.
    /// Called internally during first submit with timing.
    fn ensure_timing_initialized(&self) {
        if self.limits.timing_supported {
            let mut pool = self.timing_pool.write().unwrap();
            pool.init(&self.device, &self.queue);
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
    /// The surface texture, None if acquisition failed (e.g., surface out of date)
    texture: Option<wgpu::SurfaceTexture>,
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
    /// Check if this frame is valid and can be rendered to.
    /// Returns false if frame acquisition failed (e.g., surface out of date).
    /// When false, the caller should skip rendering and reconfigure the surface.
    pub fn is_valid(&self) -> bool {
        self.texture.is_some()
    }

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
