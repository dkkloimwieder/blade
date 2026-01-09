# Blade WebGPU Backend - Technical Deep Dive

> WebGPU backend via wgpu for cross-platform GPU compute and graphics

---

## 1. Overview

The WebGPU backend provides a modern, portable GPU abstraction using the wgpu library. It runs natively on desktop (via Vulkan/Metal/DX12) and in browsers via WebGPU. Enabled with `RUSTFLAGS="--cfg blade_wgpu"`.

### File Structure

| File | Lines | Purpose |
|------|-------|---------|
| `mod.rs` | 941 | Context, Hub, resource handles, bind group cache |
| `command.rs` | 1,751 | Command recording, deferred execution, pass encoding |
| `pipeline.rs` | 620 | Shader compilation (WGSL→WGSL), pipeline creation |
| `resource.rs` | 432 | Buffer, texture, sampler creation |
| `surface.rs` | 207 | Surface/swapchain management |
| `platform.rs` | 182 | Platform-specific initialization (native/WASM) |

**Total**: ~4,133 lines

### Backend Selection

**File**: `blade-graphics/Cargo.toml:126-136`

```toml
# WebGPU backend dependencies (use blade_wgpu cfg flag)
[target.'cfg(blade_wgpu)'.dependencies]
slotmap = { workspace = true }
wgpu = { workspace = true }
naga = { workspace = true, features = ["wgsl-out"] }

[target.'cfg(all(blade_wgpu, not(target_arch = "wasm32")))'.dependencies]
pollster = { workspace = true }

[target.'cfg(all(blade_wgpu, target_arch = "wasm32"))'.dependencies]
wasm-bindgen-futures = "0.4"
```

---

## 2. Architecture

### 2.1 Handle-Based Resource Management

Unlike Vulkan's raw handles or GLES's OpenGL objects, the WebGPU backend uses **slotmap keys** for Copy-able, type-safe resource handles:

**File**: `mod.rs:26-41`

```rust
new_key_type! {
    pub struct BufferKey;
    pub struct TextureKey;
    pub struct TextureViewKey;
    pub struct SamplerKey;
    pub struct RenderPipelineKey;
    pub struct ComputePipelineKey;
    pub struct BindGroupLayoutKey;
}
```

**Public Handle Types**:

```rust
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Buffer {
    raw: BufferKey,      // Slotmap key (generational, prevents use-after-free)
    size: u64,
    data: *mut u8,       // CPU shadow memory pointer (null for Device memory)
}

pub struct Texture {
    raw: TextureKey,
    format: crate::TextureFormat,
    target_size: [u16; 2],
}

pub struct TextureView {
    raw: TextureViewKey,
    target_size: [u16; 2],
    aspects: crate::TexelAspects,
}

pub struct Sampler {
    raw: SamplerKey,
}
```

### 2.2 The Hub: Central Resource Storage

**File**: `mod.rs:415-422`

The Hub stores actual wgpu resources, accessed via slotmap keys:

```rust
struct Hub {
    buffers: SlotMap<BufferKey, BufferEntry>,
    textures: SlotMap<TextureKey, TextureEntry>,
    texture_views: SlotMap<TextureViewKey, wgpu::TextureView>,
    samplers: SlotMap<SamplerKey, wgpu::Sampler>,
    render_pipelines: SlotMap<RenderPipelineKey, RenderPipelineEntry>,
    compute_pipelines: SlotMap<ComputePipelineKey, ComputePipelineEntry>,
}
```

**Concurrency Model**:
- `RwLock<Hub>` allows concurrent read access during command recording
- Write access only needed for resource creation/destruction
- Hub is `Arc`-wrapped so `Surface` can share access

### 2.3 Context Structure

**File**: `mod.rs:742-758`

```rust
pub struct Context {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    hub: std::sync::Arc<RwLock<Hub>>,
    device_information: crate::DeviceInformation,
    limits: Limits,
    bind_group_cache: RwLock<BindGroupCache>,
    uniform_buffer: RwLock<UniformBuffer>,  // Triple-buffered for performance
    timing_pool: RwLock<TimingQueryPool>,   // GPU timing queries
}
```

---

## 3. Platform Initialization

### 3.1 Native (pollster)

**File**: `platform.rs:109-182`

```rust
#[cfg(not(target_arch = "wasm32"))]
pub fn create_context(desc: &crate::ContextDesc) -> Result<Context, PlatformError> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,  // Vulkan/Metal/DX12
        ..Default::default()
    });

    // Blocking adapter request via pollster
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))?;

    // Request timing feature if available
    let timing_supported = adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY);
    let mut required_features = wgpu::Features::empty();
    if desc.timing && timing_supported {
        required_features |= wgpu::Features::TIMESTAMP_QUERY;
    }

    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("Blade WebGPU Device"),
        required_features,
        required_limits: wgpu::Limits::default(),
        memory_hints: wgpu::MemoryHints::default(),
        experimental_features: wgpu::ExperimentalFeatures::default(),
        trace: wgpu::Trace::Off,
    }))?;

    // Set device lost callback for graceful error handling
    device.set_device_lost_callback(|reason, message| {
        log::error!("WebGPU device lost: {:?} - {}", reason, message);
    });

    Ok(Context { ... })
}
```

### 3.2 WASM (async)

**File**: `platform.rs:26-103`

```rust
#[cfg(target_arch = "wasm32")]
pub async fn create_context(desc: &crate::ContextDesc) -> Result<Context, PlatformError> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,  // Browser WebGPU only
        ..Default::default()
    });

    // Async adapter request (no pollster on WASM)
    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions { ... })
        .await
        .map_err(|e| PlatformError(format!("Adapter request failed: {}", e)))?;

    let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor { ... })
        .await
        .map_err(|e| PlatformError(format!("Device request failed: {}", e)))?;

    Ok(Context { ... })
}
```

**Context Initialization API**:

```rust
// Native
pub unsafe fn init(desc: crate::ContextDesc) -> Result<Self, crate::NotSupportedError>

// WASM
pub async fn init_async(desc: crate::ContextDesc) -> Result<Self, crate::NotSupportedError>
```

---

## 4. Command Encoding

### 4.1 Deferred Command Model

Like the GLES backend, WebGPU uses deferred command recording:

**File**: `command.rs:314-321`

```rust
pub struct CommandEncoder {
    pub(super) name: String,
    pub(super) commands: Vec<Command>,        // Deferred commands
    pub(super) plain_data: Vec<u8>,           // Packed uniform data
    pub(super) present_frames: Vec<Frame>,
    pub(super) limits: Limits,
}
```

### 4.2 Command Enum

**File**: `command.rs:143-243`

```rust
pub(super) enum Command {
    // Transfer commands
    FillBuffer { dst, size, value },
    CopyBufferToBuffer { src, dst, size },
    CopyBufferToTexture { src, bytes_per_row, dst, size },
    CopyTextureToBuffer { src, dst, bytes_per_row, size },
    CopyTextureToTexture { src, dst, size },

    // Render pass commands
    BeginRenderPass { label, color_attachments, depth_attachment },
    EndRenderPass,
    SetRenderPipeline { key },
    SetViewport { viewport },
    SetScissor { rect },
    SetStencilReference { reference },
    SetVertexBuffer { slot, buffer },
    Draw { first_vertex, vertex_count, first_instance, instance_count },
    DrawIndexed { index_buffer, index_format, index_count, base_vertex, ... },
    DrawIndirect { indirect_buffer },
    DrawIndexedIndirect { index_buffer, index_format, indirect_buffer },

    // Compute pass commands
    BeginComputePass { label },
    EndComputePass,
    SetComputePipeline { key },
    Dispatch { groups },
    DispatchIndirect { indirect_buffer },

    // Bind group recording (resolved at submit time)
    RecordBindGroup { group_index, entries },

    // Texture initialization (WebGPU textures are zeroed on creation, so this is a no-op)
    InitTexture,
}
```

### 4.3 Pass Encoder Hierarchy

```
CommandEncoder
├── transfer(label) → TransferCommandEncoder
├── compute(label) → ComputeCommandEncoder
│                      └── with(pipeline) → PipelineEncoder
└── render(label, targets) → RenderCommandEncoder
                               └── with(pipeline) → PipelineEncoder
```

**File**: `command.rs:332-342`

```rust
pub type TransferCommandEncoder<'a> = PassEncoder<'a, ()>;
pub type ComputeCommandEncoder<'a> = PassEncoder<'a, ComputePipeline>;
pub type RenderCommandEncoder<'a> = PassEncoder<'a, RenderPipeline>;

pub struct PipelineEncoder<'a> {
    commands: &'a mut Vec<Command>,
    plain_data: &'a mut Vec<u8>,
    group_mappings: &'a [ShaderDataMapping],
    limits: &'a Limits,
}
```

### 4.4 Render Pass Creation

**File**: `command.rs:397-437`

```rust
pub fn render(
    &mut self,
    _label: &str,
    targets: crate::RenderTargetSet,
) -> RenderCommandEncoder<'_> {
    let color_attachments: Vec<RenderColorAttachment> = targets
        .colors
        .iter()
        .map(|ct| {
            let resolve_target = match &ct.finish_op {
                crate::FinishOp::ResolveTo(view) => Some(view.raw),
                _ => None,
            };
            RenderColorAttachment {
                view_key: ct.view.raw,
                load_op: ct.init_op,
                store_op: ct.finish_op,
                frame_view: None,
                resolve_target,  // MSAA resolve support
            }
        })
        .collect();

    let depth_attachment = targets.depth_stencil.as_ref().map(|ds| RenderDepthAttachment {
        view_key: ds.view.raw,
        depth_load_op: ds.init_op,
        depth_store_op: ds.finish_op,
        stencil_load_op: ds.init_op,
        stencil_store_op: ds.finish_op,
    });

    self.commands.push(Command::BeginRenderPass { color_attachments, depth_attachment });
    self.pass(PassKind::Render)
}
```

---

## 5. Command Execution

### 5.1 Submit Flow

**File**: `command.rs:750-794`

```rust
fn submit(&self, encoder: &mut CommandEncoder) -> SyncPoint {
    // 1. Sync all dirty shadow buffers to GPU
    self.sync_dirty_buffers();

    // 2. Create wgpu command encoder
    let mut cmd_encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some(&encoder.name),
    });

    // 3. Execute recorded commands with bind group cache
    let hub = self.hub.read().unwrap();
    let mut cache = self.bind_group_cache.write().unwrap();
    let mut uniform_buffer = self.uniform_buffer.write().unwrap();
    self.execute_commands(&hub, &mut cache, &mut uniform_buffer, &mut cmd_encoder,
                          &encoder.commands, &encoder.plain_data);

    // 4. Submit to queue
    let submission_index = self.queue.submit(std::iter::once(cmd_encoder.finish()));

    // 5. Present frames and cleanup their views from hub
    {
        let mut hub = self.hub.write().unwrap();
        for frame in encoder.present_frames.drain(..) {
            if let Some(view_key) = frame.view_key {
                hub.texture_views.remove(view_key);
            }
            frame.texture.present();
        }
    }

    SyncPoint { submission_index }
}
```

### 5.2 Render Pass Execution

**File**: `command.rs:1361-1545`

The `execute_render_pass` method:
1. Builds wgpu color/depth attachments from recorded data
2. Creates wgpu render pass
3. Iterates through commands, handling:
   - Pipeline binding
   - Viewport/scissor state
   - Vertex buffer binding
   - Bind group accumulation via `RecordBindGroup`
   - Draw calls (flush bind groups before each draw)

```rust
fn execute_render_pass(&self, hub: &Hub, encoder: &mut wgpu::CommandEncoder,
                       commands: &[Command], color_attachments: &[RenderColorAttachment], ...) {
    // Build wgpu attachments with resolve targets
    let wgpu_color_attachments: Vec<Option<wgpu::RenderPassColorAttachment>> = ...;

    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &wgpu_color_attachments,
        depth_stencil_attachment: wgpu_depth_attachment,
        ...
    });

    for cmd in commands {
        match cmd {
            Command::SetRenderPipeline { key } => { ... }
            Command::RecordBindGroup { group_index, entries } => {
                // Accumulate entries - bind group created at draw time
                state.pending_bind_groups.entry(*group_index)
                    .or_insert_with(Vec::new)
                    .extend(entries.iter().cloned());
            }
            Command::Draw { ... } => {
                state.flush_render_bind_groups(&mut render_pass);  // Create & bind groups
                render_pass.draw(...);
            }
            ...
        }
    }
}
```

---

## 6. Bind Group Caching

### 6.1 Cache Architecture

**File**: `mod.rs:446-610`

```rust
pub(super) struct BindGroupCacheKey {
    pipeline_key: PipelineKey,           // Render or Compute
    group_index: u32,
    bindings: Vec<ResourceBinding>,      // Sorted resource list
}

pub(super) enum ResourceBinding {
    Buffer { binding, key, offset, size },
    TextureView { binding, key },
    Sampler { binding, key },
    PlainData { binding, offset, size },  // Ephemeral, not cached
}

pub(super) struct BindGroupCache {
    groups: HashMap<BindGroupCacheKey, wgpu::BindGroup>,
    deps: DependencyTracker,             // For invalidation
    max_size: usize,                     // LRU capacity (default 1024)
    access_order: Vec<BindGroupCacheKey>,
    hits: u64,
    misses: u64,
}
```

### 6.2 Cache Lookup and Creation

**File**: `mod.rs:356-383`

```rust
pub fn get_or_create<F>(&mut self, key: BindGroupCacheKey, create_fn: F) -> &wgpu::BindGroup
where F: FnOnce() -> wgpu::BindGroup
{
    if self.groups.contains_key(&key) {
        self.hits += 1;
        // Update LRU order
        self.access_order.retain(|k| k != &key);
        self.access_order.push(key.clone());
        return self.groups.get(&key).unwrap();
    }

    self.misses += 1;
    let bind_group = create_fn();

    // Register dependencies for invalidation
    self.deps.register(&key);

    // Evict LRU if over capacity
    while self.groups.len() >= self.max_size {
        self.evict_lru();
    }

    self.access_order.push(key.clone());
    self.groups.insert(key.clone(), bind_group);
    self.groups.get(&key).unwrap()
}
```

### 6.3 Resource Invalidation

**File**: `mod.rs:385-417`

When resources are destroyed, cached bind groups must be invalidated:

```rust
pub fn invalidate_buffer(&mut self, key: BufferKey) {
    if let Some(dependents) = self.deps.get_buffer_dependents(key).cloned() {
        for cache_key in dependents {
            self.groups.remove(&cache_key);
            self.access_order.retain(|k| k != &cache_key);
        }
    }
    self.deps.remove_buffer(key);
}
```

**Critical**: Invalidation must happen BEFORE removing from Hub to prevent dangling references.

---

## 7. Uniform Buffer Management

### 7.1 Triple Buffering

**File**: `mod.rs:436-489`

```rust
const UNIFORM_BUFFER_COUNT: usize = 3;

pub(super) struct UniformBuffer {
    buffers: [Option<wgpu::Buffer>; UNIFORM_BUFFER_COUNT],
    capacity: u64,
    current_index: usize,
}

impl UniformBuffer {
    fn ensure_capacity(&mut self, device: &wgpu::Device, size: u64) -> &wgpu::Buffer {
        // Rotate to next buffer (avoids GPU stalls from write_buffer)
        self.current_index = (self.current_index + 1) % UNIFORM_BUFFER_COUNT;

        // Grow all buffers if needed (power-of-two sizing)
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
        }

        self.buffers[self.current_index].as_ref().unwrap()
    }
}
```

### 7.2 Plain Data Binding

**File**: `command.rs:17-43`

```rust
impl<T: bytemuck::Pod> crate::ShaderBindable for T {
    fn bind_to(&self, ctx: &mut PipelineContext, index: u32) {
        let self_slice = bytemuck::bytes_of(self);

        // Align to UBO requirements
        let alignment = ctx.limits.uniform_buffer_alignment as usize;
        let rem = ctx.plain_data.len() % alignment;
        if rem != 0 {
            ctx.plain_data.resize(ctx.plain_data.len() - rem + alignment, 0);
        }

        let offset = ctx.plain_data.len() as u32;
        let size = round_up_uniform_size(self_slice.len() as u32, ctx.limits.uniform_buffer_alignment);
        ctx.plain_data.extend_from_slice(self_slice);
        ctx.plain_data.extend((self_slice.len() as u32..size).map(|_| 0));

        for slot in ctx.targets[index as usize].iter() {
            ctx.commands.push(Command::RecordBindGroup {
                group_index: slot.group,
                entries: vec![BindGroupEntry::PlainData { binding: slot.binding, offset, size }],
            });
        }
    }
}
```

---

## 8. Pipeline Creation

### 8.1 Shader Compilation Flow

```
WGSL Source → Naga Parse → Fill Bindings → Strip Entry Points → Placeholder Bindings → Re-validate → Naga WGSL Backend → wgpu Compile
```

**File**: `pipeline.rs:299-380`

For multi-entry-point shaders, we strip all entry points except the target one. This prevents binding conflicts when different entry points have different resource requirements.

```rust
fn load_shader(
    &self,
    sf: crate::ShaderFunction,
    group_layouts: &[&crate::ShaderDataLayout],
    group_infos: &mut [crate::ShaderDataInfo],
    vertex_fetch_states: &[crate::VertexFetchState],
) -> CompiledShader {
    let ep_index = sf.entry_point_index();
    let (mut module, module_info) = sf.shader.resolve_constants(sf.constants);

    // Process bindings for the TARGET entry point only
    let target_stage = module.entry_points[ep_index].stage;
    let ep_info = module_info.get_entry_point(ep_index);
    crate::Shader::fill_resource_bindings(
        &mut module, group_infos, target_stage, ep_info, group_layouts,
    );

    // Fill vertex attribute locations
    let attribute_mappings = crate::Shader::fill_vertex_locations(&mut module, ep_index, vertex_fetch_states);

    // Strip all entry points except the target one
    let target_ep = module.entry_points.swap_remove(ep_index);
    module.entry_points.clear();
    module.entry_points.push(target_ep);

    // Assign placeholder bindings to any remaining unbound resource variables
    // WebGPU only allows groups 0-3, so we use group 3 with high binding numbers
    let placeholder_group = 3u32;
    let mut placeholder_binding = 1000u32;
    for (_handle, var) in module.global_variables.iter_mut() {
        let needs_binding = matches!(
            var.space,
            naga::AddressSpace::Uniform | naga::AddressSpace::Storage { .. } | naga::AddressSpace::Handle
        );
        if needs_binding && var.binding.is_none() {
            var.binding = Some(naga::ResourceBinding {
                group: placeholder_group,
                binding: placeholder_binding,
            });
            placeholder_binding += 1;
        }
    }

    // Re-validate the stripped module to get correct ModuleInfo
    let stripped_info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    ).validate(&module).expect("Stripped module validation failed");

    // Emit modified module back to WGSL with @group/@binding annotations
    let wgsl_source = naga::back::wgsl::write_string(
        &module, &stripped_info, naga::back::wgsl::WriterFlags::empty(),
    ).expect("Failed to emit WGSL from modified naga module");

    let wgpu_module = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(sf.entry_point),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Owned(wgsl_source)),
    });

    CompiledShader { module: wgpu_module, entry_point: sf.entry_point.to_string(), ... }
}
```

**Why entry point stripping?** WebGPU compiles the entire shader module as a unit. If a shader has multiple entry points with different resource requirements (e.g., a compute shader with reset/emit/update and a render shader with vs/fs), all variables must have valid bindings even if unused by the target entry point. Stripping removes unused entry points, making their variables dead code.

### 8.2 Error Scope Handling

**File**: `pipeline.rs:5-30`

Pipeline creation uses error scopes for validation feedback:

```rust
#[cfg(not(target_arch = "wasm32"))]
fn with_error_scope<T, F: FnOnce() -> T>(device: &wgpu::Device, name: &str, f: F) -> T {
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let result = f();
    let error = pollster::block_on(scope.pop());
    if let Some(e) = error {
        log::error!("WebGPU pipeline '{}' validation error: {}", name, e);
    }
    result
}

#[cfg(target_arch = "wasm32")]
fn with_error_scope<T, F: FnOnce() -> T>(device: &wgpu::Device, name: &str, f: F) -> T {
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let result = f();
    let name = name.to_string();
    wasm_bindgen_futures::spawn_local(async move {
        if let Some(e) = scope.pop().await {
            log::error!("WebGPU pipeline '{}' validation error: {}", name, e);
        }
    });
    result
}
```

### 8.3 Shader Data Mapping

**File**: `mod.rs:150-165`

```rust
#[derive(Clone, Debug)]
struct BindingSlot {
    group: u32,
    binding: u32,
}

type SlotList = Vec<BindingSlot>;

#[derive(Clone)]
struct ShaderDataMapping {
    /// For each binding in the ShaderDataLayout, the target slot(s)
    targets: Box<[SlotList]>,
}
```

---

## 9. Surface Management

### 9.1 Surface Creation

**File**: `surface.rs:28-87` (native), `surface.rs:93-166` (WASM)

```rust
// Native
pub fn create_surface<I: HasWindowHandle + HasDisplayHandle>(
    &self, window: &I
) -> Result<Surface, crate::NotSupportedError> {
    let surface = unsafe {
        self.instance.create_surface_unsafe(
            wgpu::SurfaceTargetUnsafe::from_window(window)?
        )
    }?;

    let caps = surface.get_capabilities(&self.adapter);
    let format = caps.formats.first().copied()
        .unwrap_or(wgpu::TextureFormat::Bgra8UnormSrgb);

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: 1, height: 1,  // Reconfigured later
        present_mode: wgpu::PresentMode::Fifo,
        ...
    };

    Ok(Surface { raw: surface, config, format: blade_format, hub: self.hub.clone() })
}

// WASM: Auto-discovers canvas with id="blade"
pub fn create_surface<I>(&self, _window: &I) -> Result<Surface, crate::NotSupportedError> {
    let canvas = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(crate::CANVAS_ID))
        .dyn_into::<web_sys::HtmlCanvasElement>()?;

    self.create_surface_from_canvas(canvas)
}
```

### 9.2 Frame Acquisition

**File**: `surface.rs:209-226`

```rust
pub fn acquire_frame(&self) -> Frame {
    let texture = self.raw.get_current_texture().expect("Failed to acquire frame");
    let view = texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

    // Store view in hub so render pass can look it up by key
    let view_key = self.hub.write().unwrap().texture_views.insert(view);

    Frame {
        texture,
        view_key: Some(view_key),
        target_size: [self.config.width as u16, self.config.height as u16],
        format: self.format,
    }
}
```

---

## 10. Synchronization

### 10.1 SyncPoint

**File**: `mod.rs:938-941`

```rust
#[derive(Clone, Debug)]
pub struct SyncPoint {
    submission_index: wgpu::SubmissionIndex,
}
```

### 10.2 Waiting

**File**: `command.rs:786-793`

```rust
fn wait_for(&self, sp: &SyncPoint, timeout_ms: u32) -> bool {
    let timeout = std::time::Duration::from_millis(timeout_ms as u64);
    self.device.poll(wgpu::PollType::Wait {
        submission_index: Some(sp.submission_index.clone()),
        timeout: Some(timeout),
    }).is_ok()
}
```

---

## 11. Buffer Memory Model

### 11.1 Shadow Memory for Upload/Shared Buffers

**File**: `resource.rs:158-220`

```rust
fn create_buffer(&self, desc: crate::BufferDesc) -> Buffer {
    let (shadow, data_ptr) = match desc.memory {
        crate::Memory::Device => {
            // Device-local: no CPU access
            (None, std::ptr::null_mut())
        }
        crate::Memory::Upload | crate::Memory::Shared => {
            // Host-visible: create shadow memory for CPU access
            let mut shadow_data = vec![0u8; desc.size as usize].into_boxed_slice();
            let ptr = shadow_data.as_mut_ptr();
            (Some(shadow_data), ptr)
        }
        crate::Memory::External(_) => {
            panic!("External memory is not supported in WebGPU backend")
        }
    };

    let gpu = self.device.create_buffer(&wgpu::BufferDescriptor { ... });

    Buffer { raw: key, size: desc.size, data: data_ptr }
}
```

### 11.2 Dirty Buffer Sync

**File**: `mod.rs:547-569`

```rust
fn sync_dirty_buffers(&self) {
    let hub = self.hub.read().unwrap();
    for (_key, entry) in hub.buffers.iter() {
        if entry.dirty.load(Ordering::Acquire) {
            if let Some(ref shadow) = entry.shadow {
                self.queue.write_buffer(&entry.gpu, 0, shadow);
            }
            entry.dirty.store(false, Ordering::Release);
        }
    }
}
```

Called automatically before `queue.submit()`.

---

## 12. Capabilities and Limits

**File**: `mod.rs:536-545`

```rust
pub fn capabilities(&self) -> crate::Capabilities {
    crate::Capabilities {
        ray_query: crate::ShaderVisibility::empty(),  // Not supported
        sample_count_mask: 0b0101,                     // 1 and 4 samples
        dual_source_blending: false,                   // Not supported
    }
}
```

**File**: `mod.rs:49-53`

```rust
struct Limits {
    uniform_buffer_alignment: u32,
    timing_supported: bool,
}
```

---

## 13. Current Gaps and Limitations

### 13.1 Unsupported Features

| Feature | Status | Reason |
|---------|--------|--------|
| Acceleration Structures | `panic!()` | Not in WebGPU base spec |
| TextureArray binding | `unimplemented!()` | Limited WebGPU support |
| BufferArray binding | `unimplemented!()` | Limited WebGPU support |
| External memory | `panic!()` | Not in WebGPU spec |
| Dual-source blending | `panic!()` | Not in WebGPU base spec |
| Ray tracing | Not supported | No WebGPU extension |

### 13.2 Partial Implementation

| Feature | Status | Notes |
|---------|--------|-------|
| GPU Timing | Infrastructure only | Feature detection done, async readback needed |
| MSAA Resolve | Implemented | Via `FinishOp::ResolveTo` |
| Compute shaders | Full support | Unlike GLES WebGL2 |

### 13.3 Platform Differences

| Aspect | Native | WASM |
|--------|--------|------|
| Backend | Vulkan/Metal/DX12 | BROWSER_WEBGPU |
| Initialization | Sync (pollster) | Async |
| Error scopes | Blocking pop | Async pop |
| Canvas discovery | N/A | Auto-finds id="blade" |
| Surface format | Adapter preference | Browser-dependent (Firefox: rgba8unorm only) |

---

## 14. Comparison: WebGPU vs Other Backends

| Aspect | WebGPU Backend | Vulkan Backend | GLES Backend |
|--------|---------------|----------------|--------------|
| **Command Model** | Deferred (`Vec<Command>`) | Immediate | Deferred |
| **Shader Format** | WGSL (via Naga) | SPIR-V | GLSL (via Naga) |
| **Binding Model** | Bind groups (cached) | Descriptor sets | Slot-based |
| **Handle Type** | Slotmap keys (Copy) | Raw handles | GL objects |
| **Memory Model** | Shadow buffers + sync | Explicit gpu-alloc | Persistent map or copy |
| **Sync** | SubmissionIndex | Timeline semaphores | GL fences |
| **Ray Tracing** | Not supported | Full support | Not supported |
| **Compute** | Full support | Full support | Not supported (WebGL2) |
| **Platform** | Native + WASM | Native only | Native + WASM |

---

## 15. Dependencies

```toml
[target.'cfg(blade_wgpu)'.dependencies]
slotmap = { workspace = true }         # Handle management
wgpu = { workspace = true }            # WebGPU implementation
naga = { features = ["wgsl-out"] }     # Shader processing

# Native only
pollster = { workspace = true }        # Blocking async

# WASM only
wasm-bindgen-futures = "0.4"           # Async runtime
```

---

## 16. Usage Examples

### 16.1 Native Initialization

```rust
let context = unsafe {
    blade_graphics::Context::init(blade_graphics::ContextDesc {
        validation: true,
        timing: true,
        ..Default::default()
    })
}?;
```

### 16.2 WASM Initialization

```rust
let context = blade_graphics::Context::init_async(blade_graphics::ContextDesc {
    validation: true,
    ..Default::default()
}).await?;
```

### 16.3 Render Loop

```rust
// Acquire frame
let frame = surface.acquire_frame();

// Create command encoder
let mut encoder = context.create_command_encoder(blade_graphics::CommandEncoderDesc {
    name: "main",
});

encoder.start();

// Render pass
{
    let mut pass = encoder.render("main", blade_graphics::RenderTargetSet {
        colors: &[blade_graphics::RenderTarget {
            view: frame.texture_view(),
            init_op: blade_graphics::InitOp::Clear(blade_graphics::TextureColor::TransparentBlack),
            finish_op: blade_graphics::FinishOp::Store,
        }],
        depth_stencil: None,
    });

    let mut pe = pass.with(&pipeline);
    pe.bind(0, &shader_data);
    pe.draw(0, 3, 0, 1);
}

// Present
encoder.present(frame);
let sync = context.submit(&mut encoder);
```

---

## 17. WASM/JS Interop Best Practices

Efficient WASM→JS→GPU data transfer is critical for WebGPU performance. Every `write_buffer` call crosses the WASM/JS boundary.

### 17.1 Minimize write_buffer Calls

**Bad**: Per-object updates
```rust
for object in objects {
    context.sync_buffer(object.uniform_buffer);  // 1000 calls!
}
```

**Good**: Batch into single buffer
```rust
// Pack all transforms into one buffer
let transforms: Vec<Transform> = objects.iter().map(|o| o.transform).collect();
unsafe { ptr::copy_nonoverlapping(transforms.as_ptr(), buffer.data() as *mut Transform, transforms.len()); }
context.sync_buffer(buffer);  // 1 call
```

### 17.2 Use Dirty Region Tracking

Blade supports partial buffer updates via `sync_buffer_range()`:

```rust
// Only sync the modified region
context.sync_buffer_range(buffer, modified_offset, modified_size);
```

This reduces upload bandwidth by 10-100x for sparse updates (e.g., 1000 moving objects out of 10000).

### 17.3 Compute Shaders for Data Transforms

Move CPU→GPU data transforms to compute shaders:

**CPU-side (slow)**:
```rust
for particle in particles {
    particle.position += particle.velocity * dt;  // CPU work
}
context.sync_buffer(particle_buffer);  // Upload ALL particles
```

**GPU-side (fast)**:
```wgsl
@compute @workgroup_size(256)
fn update(@builtin(global_invocation_id) id: vec3<u32>) {
    particles[id.x].position += particles[id.x].velocity * params.dt;
}
```

The bunnymark example demonstrates this pattern - physics runs entirely on GPU.

### 17.4 Frame-Skip for Idle Scenes

Skip rendering when nothing changed:

```rust
if !scene_dirty && !input_received {
    // Skip this frame entirely
    return;
}
```

This is especially important for WASM where RAF callbacks have overhead.

### 17.5 Batch Uniform Updates

Group per-frame uniforms together:

```rust
#[repr(C)]
struct FrameUniforms {
    view_proj: [[f32; 4]; 4],
    time: f32,
    delta_time: f32,
    screen_size: [f32; 2],
}

// Single bind call per frame
pe.bind(0, &FrameUniforms { ... });
```

---

## 18. Bind Group Organization Patterns

Optimal bind group layout minimizes `set_bind_group` calls during rendering.

### 18.1 Frequency-Based Hierarchy

Organize bind groups by update frequency:

| Group | Frequency | Contents | Example |
|-------|-----------|----------|---------|
| 0 | Per-frame | Camera, time, globals | `view_proj`, `time` |
| 1 | Per-material | Textures, samplers, material params | `albedo_texture`, `roughness` |
| 2 | Per-object | Transforms, instance data | `model_matrix` |

```rust
// Group 0: Bound once per frame
pe.bind(0, &FrameData { view_proj, time });

for material in materials {
    // Group 1: Bound once per material
    pe.bind(1, &MaterialData { texture: material.albedo, sampler });

    for object in material.objects {
        // Group 2: Bound per object
        pe.bind(2, &ObjectData { transform: object.model_matrix });
        pe.draw(...);
    }
}
```

### 18.2 Bind Group Caching

Blade automatically caches bind groups based on resource identity:

```rust
// These two calls reuse the same cached bind group:
pe.bind(0, &StaticData { texture: my_texture, sampler: my_sampler });
pe.bind(0, &StaticData { texture: my_texture, sampler: my_sampler });  // Cache hit!
```

**Cache stats** are available via `context.cache_stats()`:
```rust
let (hits, misses, size) = context.cache_stats();
let hit_rate = hits as f64 / (hits + misses) as f64 * 100.0;
log::info!("Bind group cache: {:.1}% hit rate, {} entries", hit_rate, size);
```

### 18.3 Dynamic Offsets vs Separate Bind Groups

**Use dynamic offsets** for frequently-changing uniform data:
- Lower bind group creation overhead
- Blade handles this automatically for `Plain` uniform bindings

**Use separate bind groups** for:
- Texture/sampler combinations (can't use dynamic offsets)
- Data that rarely changes (benefit from caching)

### 18.4 Pipeline Layout Sharing

Pipelines with identical bind group layouts can share bind groups:

```rust
// Both pipelines use same layout - bind groups are interchangeable
let layout = <MyData as gpu::ShaderData>::layout();
let pipeline_a = context.create_render_pipeline(gpu::RenderPipelineDesc {
    data_layouts: &[&layout],
    ...
});
let pipeline_b = context.create_render_pipeline(gpu::RenderPipelineDesc {
    data_layouts: &[&layout],  // Same layout!
    ...
});
```

### 18.5 Example: Optimal Bunnymark Layout

The bunnymark example demonstrates optimal organization:

```rust
// Group 0: Static resources (texture, sampler, instance buffer)
// Cached across all frames - single bind group created once
pe.bind(0, &StaticParams {
    sprite_texture: view,
    sprite_sampler: sampler,
    instances: instance_buf.into(),
});

// Group 1: Per-frame uniforms (MVP, sprite size)
// Recreated each frame but cheap (only uniform data)
pe.bind(1, &FrameParams {
    globals: Globals { mvp_transform, sprite_size, .. },
});

// Single draw call for ALL bunnies (instanced rendering)
pe.draw(0, 4, 0, bunny_count);
```

**Result**: 99%+ cache hit rate, single draw call for 100k+ sprites.

---

## 19. WASM Development Tools

### 19.1 Default: cargo-run-wasm

The default WASM runner, configured via alias in `.cargo/config.toml`:

```bash
RUSTFLAGS="--cfg blade_wgpu" cargo run-wasm --example bunnymark
```

**Pros**: Simple, integrated
**Cons**: No DWARF debug symbol support

### 19.2 Alternative: wasm-server-runner

A cleaner cargo runner integration with auto-reload support.

**Installation:**
```bash
cargo install wasm-server-runner
```

**Configuration** (`.cargo/config.toml`):
```toml
[target.wasm32-unknown-unknown]
runner = "wasm-server-runner"
```

**Custom HTML Template** (`wasm-index.html`):
```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Blade WASM</title>
    <style>
        body { margin: 0px; }
        canvas { width: 100vw; height: 100vh; display: block; }
    </style>
</head>
<body>
    {{ NO_MODULE }}
    <script type="module">
        // {{ MODULE }}
        wasm_bindgen('./api/wasm.wasm');
    </script>
</body>
</html>
```

**Usage:**
```bash
WASM_SERVER_RUNNER_CUSTOM_INDEX_HTML=wasm-index.html \
RUSTFLAGS="--cfg blade_wgpu" \
cargo run --target wasm32-unknown-unknown -p blade-graphics --example webgpu-triangle
```

**Server runs on**: http://127.0.0.1:1334

**Pros**: Cargo runner integration, auto-reload, cleaner CLI
**Cons**: No DWARF support (yet), requires custom HTML for full-screen

### 19.3 Manual Build with DWARF Debug Symbols

For source-level debugging in Chrome DevTools:

```bash
# 1. Build with debug info
RUSTFLAGS="--cfg blade_wgpu" cargo build --target wasm32-unknown-unknown \
  -p blade-graphics --example webgpu-triangle

# 2. Run wasm-bindgen with --keep-debug
wasm-bindgen --keep-debug --web --out-dir ./debug-out \
  target/wasm32-unknown-unknown/debug/examples/webgpu_triangle.wasm

# 3. Create index.html in debug-out/
cat > debug-out/index.html << 'EOF'
<!DOCTYPE html>
<html>
<head><style>body{margin:0}canvas{width:100vw;height:100vh;display:block}</style></head>
<body>
<script type="module">
import init from './webgpu_triangle.js';
init();
</script>
</body>
</html>
EOF

# 4. Serve
python3 -m http.server 8000 -d ./debug-out
```

**Requirements:**
- Chrome with [C/C++ DevTools Support (DWARF)](https://chromewebstore.google.com/detail/pdcpmagijalfljmkmjngeonclgbbannb) extension
- `debug = true` in Cargo.toml profile

**Pros**: Full source-level Rust debugging
**Cons**: Multi-step process, manual HTML

### 19.4 Tool Comparison

| Feature | cargo-run-wasm | wasm-server-runner | Manual |
|---------|---------------|-------------------|--------|
| Setup complexity | Low | Medium | High |
| DWARF debug | No | No | Yes |
| Auto-reload | No | Yes | No |
| Custom HTML | Built-in | Env var | Manual |
| Port | 8000 | 1334 | Any |

---

*Generated for blade-graphics v0.7.0 WebGPU backend*
