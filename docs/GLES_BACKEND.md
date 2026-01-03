# Blade GLES Backend - Technical Deep Dive

> OpenGL ES 3.x / WebGL2 backend implementation for cross-platform graphics

---

## 1. Overview

The GLES backend provides OpenGL ES 3.x support for platforms where Vulkan/Metal are unavailable, including WebGL2 for browser-based applications.

### File Structure

| File | Lines | Purpose |
|------|-------|---------|
| `mod.rs` | 696 | Core types, Context, Command enum, format mapping |
| `command.rs` | 1,184 | Command recording, execution, binding traits |
| `pipeline.rs` | 378 | Shader compilation (WGSL→GLSL), program linking |
| `resource.rs` | 400 | Buffer, texture, sampler creation |
| `egl.rs` | 896 | Native EGL platform (Linux, Windows, Android) |
| `web.rs` | 153 | WebGL2 platform (WASM) |

**Total**: ~3,700 lines

### Platform Selection

**File**: `mod.rs:3-5`

```rust
#[cfg_attr(not(target_arch = "wasm32"), path = "egl.rs")]
#[cfg_attr(target_arch = "wasm32", path = "web.rs")]
mod platform;
```

---

## 2. Platform Abstraction Layer

### 2.1 EGL Platform (Native)

**File**: `egl.rs`

The EGL platform handles native OpenGL ES contexts on desktop and mobile:

```rust
pub(super) struct ContextInner {
    glow: glow::Context,
    egl: EglContext,
}

pub struct PlatformContext {
    inner: Mutex<ContextInner>,  // Thread-safe access
}

pub struct ContextLock<'a> {
    guard: MutexGuard<'a, ContextInner>,
}

impl<'a> Drop for ContextLock<'a> {
    fn drop(&mut self) {
        self.guard.egl.unmake_current();  // Release context on unlock
    }
}
```

**Key Features**:
- Dynamic EGL library loading (`libEGL.dll`/`.dylib`/`.so`)
- ANGLE support via `EGL_ANGLE_platform_angle`
- Surfaceless rendering via `EGL_MESA_platform_surfaceless`
- EGL debug output when validation enabled
- sRGB colorspace support (EGL 1.5 core or `EGL_KHR_gl_colorspace`)

### 2.2 WebGL Platform (WASM)

**File**: `web.rs`

Simplified platform for browser-based WebGL2:

```rust
pub struct PlatformContext {
    #[allow(unused)]
    webgl2: web_sys::WebGl2RenderingContext,
    glow: glow::Context,
}

impl super::Context {
    pub unsafe fn init(_desc: crate::ContextDesc) -> Result<Self, crate::NotSupportedError> {
        // Find canvas element with id="blade"
        let canvas = web_sys::window()
            .and_then(|win| win.document())
            .expect("Cannot get document")
            .get_element_by_id("blade")
            .expect("Canvas is not found")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("Failed to downcast to canvas type");

        // Create WebGL2 context with options
        let context_options = js_sys::Object::new();
        js_sys::Reflect::set(&context_options, &"antialias".into(), &JsValue::FALSE)?;

        let webgl2 = canvas
            .get_context_with_context_options("webgl2", &context_options)?
            .dyn_into::<web_sys::WebGl2RenderingContext>()?;

        let glow = glow::Context::from_webgl2_context(webgl2.clone());
        // ...
    }
}
```

**Key Differences from EGL**:
- No mutex locking (single-threaded)
- Fixed canvas element ID (`"blade"`)
- No persistent buffer mapping (BUFFER_STORAGE unavailable)
- No debug labels (`glObjectLabel` not supported)

---

## 3. Context and Capabilities

### 3.1 Context Structure

**File**: `mod.rs:36-42`

```rust
pub struct Context {
    platform: platform::PlatformContext,
    capabilities: Capabilities,
    toggles: Toggles,
    limits: Limits,
    device_information: crate::DeviceInformation,
}
```

### 3.2 Capabilities

**File**: `mod.rs:17-23`

```rust
bitflags::bitflags! {
    struct Capabilities: u32 {
        const BUFFER_STORAGE = 1 << 0;        // GL_EXT_buffer_storage
        const DRAW_BUFFERS_INDEXED = 1 << 1;  // Per-MRT blend states
        const DISJOINT_TIMER_QUERY = 1 << 2;  // GPU timing queries
    }
}
```

| Capability | Effect |
|------------|--------|
| `BUFFER_STORAGE` | Enables persistent buffer mapping, explicit shader bindings (ES320) |
| `DRAW_BUFFERS_INDEXED` | Allows different blend states per render target |
| `DISJOINT_TIMER_QUERY` | Enables GPU pass timing |

### 3.3 Toggles and Limits

**File**: `mod.rs:25-28`

```rust
#[derive(Debug, Default)]
struct Toggles {
    scoping: bool,   // Debug scope support (GL_KHR_debug)
    timing: bool,    // Performance timing queries
}

#[derive(Clone, Debug)]
struct Limits {
    uniform_buffer_alignment: u32,  // UBO alignment requirement
}
```

---

## 4. Command Encoding System

### 4.1 Deferred Command Model

Unlike Vulkan's immediate command buffer recording, GLES uses a **deferred command model**:

**File**: `mod.rs:375-385`

```rust
pub struct CommandEncoder {
    name: String,
    commands: Vec<Command>,        // Deferred command list
    plain_data: Vec<u8>,           // Packed uniform data
    string_data: Vec<u8>,          // Debug scope names
    needs_scopes: bool,
    present_frames: Vec<platform::PlatformFrame>,
    limits: Limits,
    timing_datas: Option<Box<[TimingData]>>,
    timings: crate::Timings,
}
```

### 4.2 Command Enum

**File**: `mod.rs:220-368`

The `Command` enum contains 40+ variants organized into categories:

```rust
#[derive(Debug)]
enum Command {
    // === Draw Commands ===
    Draw { topology, start_vertex, vertex_count, instance_count },
    DrawIndexed { topology, index_buf, index_type, index_count, base_vertex, instance_count },
    DrawIndirect { topology, indirect_buf },
    DrawIndexedIndirect { topology, raw_index_buf, index_type, indirect_buf },

    // === Compute Commands ===
    Dispatch([u32; 3]),
    DispatchIndirect { indirect_buf },

    // === Transfer Commands ===
    FillBuffer { dst, size, value },
    CopyBufferToBuffer { src, dst, size },
    CopyTextureToTexture { src, dst, size },
    CopyBufferToTexture { src, dst, bytes_per_row, size },
    CopyTextureToBuffer { src, dst, bytes_per_row, size },

    // === Framebuffer Commands ===
    ResetFramebuffer,
    BlitFramebuffer { from, to },
    BindAttachment { attachment, view },
    InvalidateAttachment(u32),
    SetDrawColorBuffers(u8),
    ClearColor { draw_buffer, color, ty },
    ClearDepthStencil { depth, stencil },

    // === State Commands ===
    SetViewport(crate::Viewport),
    SetScissor(crate::ScissorRect),
    SetStencilFunc { face, function, reference, read_mask },
    SetStencilOps { face, write_mask },
    SetBlendConstant([f32; 4]),

    // === Pipeline Commands ===
    SetProgram(glow::Program),
    UnsetProgram,
    SetAllColorTargets(Option<crate::BlendState>, crate::ColorWrites),
    SetSingleColorTarget(u32, Option<crate::BlendState>, crate::ColorWrites),

    // === Binding Commands ===
    BindUniform { slot, offset, size },
    BindVertex { buffer },
    BindBuffer { target, slot, buffer, size },
    SetVertexAttribute { index, format, offset, stride, instanced },
    DisableVertexAttributes { count },
    BindSampler { slot, sampler },
    BindTexture { slot, texture, target },
    BindImage { slot, binding },
    ResetAllSamplers,

    // === Debug Commands ===
    QueryCounter { query },
    PushScope { name_range },
    PopScope,
    Barrier,
}
```

### 4.3 Command Execution

**File**: `mod.rs:508-555`

Commands execute during `submit()`:

```rust
fn submit(&self, encoder: &mut CommandEncoder) -> SyncPoint {
    let fence = {
        let gl = self.lock();
        encoder.finish(&gl);

        // Create execution context with temporary resources
        let ec = unsafe {
            let framebuf = gl.create_framebuffer().unwrap();
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuf));

            // Upload packed uniform data
            let plain_buffer = gl.create_buffer().unwrap();
            if !encoder.plain_data.is_empty() {
                gl.bind_buffer(glow::UNIFORM_BUFFER, Some(plain_buffer));
                gl.buffer_data_u8_slice(
                    glow::UNIFORM_BUFFER,
                    &encoder.plain_data,
                    glow::STATIC_DRAW,
                );
            }

            ExecutionContext { framebuf, plain_buffer, string_data: ... }
        };

        // Execute all recorded commands
        for command in encoder.commands.iter() {
            unsafe { command.execute(&*gl, &ec) };
        }

        // Cleanup and create fence
        unsafe {
            gl.delete_framebuffer(ec.framebuf);
            gl.delete_buffer(ec.plain_buffer);
            gl.fence_sync(glow::SYNC_GPU_COMMANDS_COMPLETE, 0).unwrap()
        }
    };

    SyncPoint { fence }
}
```

### 4.4 Pass Encoder Hierarchy

```
CommandEncoder
├── transfer(label) → PassEncoder<()>
├── compute(label) → PassEncoder<ComputePipeline>
└── render(label, targets) → PassEncoder<RenderPipeline>
                                 └── with(pipeline) → PipelineEncoder
```

**File**: `command.rs:155-258`

```rust
pub fn render(
    &mut self,
    label: &str,
    targets: crate::RenderTargetSet,
) -> super::PassEncoder<super::RenderPipeline> {
    self.begin_pass(label);

    // Record attachment bindings
    for (i, rt) in targets.colors.iter().enumerate() {
        let attachment = glow::COLOR_ATTACHMENT0 + i as u32;
        self.commands.push(Command::BindAttachment { attachment, view: rt.view });

        if let crate::FinishOp::Discard = rt.finish_op {
            invalidate_attachments.push(attachment);
        }
    }

    // Record viewport and scissor
    self.commands.push(Command::SetDrawColorBuffers(targets.colors.len() as _));
    self.commands.push(Command::SetViewport(...));
    self.commands.push(Command::SetScissor(...));

    // Record clear operations
    for (i, rt) in targets.colors.iter().enumerate() {
        if let crate::InitOp::Clear(color) = rt.init_op {
            self.commands.push(Command::ClearColor { draw_buffer: i as u32, color, ty: ... });
        }
    }

    self.pass(PassKind::Render)
}
```

### 4.5 Pass Cleanup (RAII)

**File**: `command.rs:347-368`

```rust
impl<T> Drop for super::PassEncoder<'_, T> {
    fn drop(&mut self) {
        self.commands.push(Command::UnsetProgram);

        for attachment in self.invalidate_attachments.drain(..) {
            self.commands.push(Command::InvalidateAttachment(attachment));
        }

        match self.kind {
            PassKind::Transfer => {}
            PassKind::Compute => {
                self.commands.push(Command::ResetAllSamplers);
            }
            PassKind::Render => {
                self.commands.push(Command::ResetAllSamplers);
                self.commands.push(Command::ResetFramebuffer);
            }
        }

        if self.has_scope {
            self.commands.push(Command::PopScope);
        }
    }
}
```

---

## 5. Pipeline Creation

### 5.1 Shader Compilation Flow

```
WGSL Source → Naga Parse → Naga Validate → Naga GLSL Backend → GL Compile → GL Link
```

**File**: `pipeline.rs:20-41`

```rust
unsafe fn create_pipeline(
    &self,
    shaders: &[crate::ShaderFunction],
    group_layouts: &[&crate::ShaderDataLayout],
    vertex_fetch_states: &[crate::VertexFetchState],
    name: &str,
    extra_flags: glsl::WriterFlags,
) -> super::PipelineInner {
    let gl = self.lock();

    // Determine binding strategy based on capabilities
    let force_explicit_bindings = self.capabilities.contains(Capabilities::BUFFER_STORAGE);

    let mut naga_options = glsl::Options {
        version: glsl::Version::Embedded {
            version: if force_explicit_bindings { 320 } else { 300 },
            is_webgl: cfg!(target_arch = "wasm32"),
        },
        writer_flags: extra_flags | glsl::WriterFlags::ADJUST_COORDINATE_SPACE,
        binding_map: Default::default(),
        zero_initialize_workgroup_memory: false,
    };
    // ...
}
```

### 5.2 Two Binding Strategies

#### Strategy 1: Explicit Bindings (ES320+ with BUFFER_STORAGE)

**File**: `pipeline.rs:49-100`

```rust
if force_explicit_bindings {
    let mut num_textures = 0u32;
    let mut num_samplers = 0u32;
    let mut num_buffers = 0u32;

    for (group_index, (data_mapping, &layout)) in group_mappings.iter_mut().zip(group_layouts) {
        for (binding_index, (slot_list, &(_, ref binding))) in data_mapping.targets.iter_mut()
            .zip(layout.bindings.iter()).enumerate()
        {
            let target = match *binding {
                crate::ShaderBinding::Texture => { num_textures += 1; num_textures - 1 }
                crate::ShaderBinding::Sampler => { num_samplers += 1; num_samplers - 1 }
                crate::ShaderBinding::Buffer | crate::ShaderBinding::Plain { .. } => {
                    num_buffers += 1; num_buffers - 1
                }
                _ => unimplemented!(),
            };

            // Pre-assign slot in Naga binding map
            let rb = naga::ResourceBinding { group: group_index as u32, binding: binding_index as u32 };
            naga_options.binding_map.insert(rb, target as u8);
            slot_list.push(target);
        }
    }
}
```

#### Strategy 2: Reflection-Based Discovery (ES300/WebGL)

**File**: `pipeline.rs:195-285`

```rust
if !force_explicit_bindings {
    for (sf, &(_, ref reflection)) in shaders.iter().zip(baked_shaders.iter()) {
        // Query texture bindings from compiled shader
        for (glsl_name, mapping) in reflection.texture_mapping.iter() {
            if let Some(ref location) = gl.get_uniform_location(program, glsl_name) {
                let mut slots = [0i32];
                gl.get_uniform_i32(program, location, &mut slots);
                targets.push(slots[0] as u32);
            }
        }

        // Query uniform block bindings
        if let Some(index) = gl.get_uniform_block_index(program, glsl_name) {
            gl.uniform_block_binding(program, index, index);  // Force assignment
            targets.push(index);
        }
    }
}
```

### 5.3 Pipeline Structures

**File**: `mod.rs:123-144`

```rust
struct PipelineInner {
    program: glow::Program,
    group_mappings: Box<[ShaderDataMapping]>,
    vertex_attribute_infos: Box<[VertexAttributeInfo]>,
    color_targets: Box<[(Option<crate::BlendState>, crate::ColorWrites)]>,
}

pub struct ComputePipeline {
    inner: PipelineInner,
    wg_size: [u32; 3],  // Workgroup size from shader
}

pub struct RenderPipeline {
    inner: PipelineInner,
    topology: crate::PrimitiveTopology,
}
```

---

## 6. Resource Management

### 6.1 Buffer Creation

**File**: `resource.rs:36-91`

```rust
pub struct Buffer {
    raw: glow::Buffer,
    size: u64,
    data: *mut u8,  // Host-visible pointer (may be null)
}

fn create_buffer(&self, desc: crate::BufferDesc) -> super::Buffer {
    let raw = unsafe { gl.create_buffer() }.unwrap();
    let mut data = ptr::null_mut();

    // Configure flags based on memory type
    let (storage_flags, map_flags, usage) = match desc.memory {
        crate::Memory::Device => (0, 0, glow::STATIC_DRAW),
        crate::Memory::Shared => (
            glow::MAP_PERSISTENT_BIT | glow::MAP_COHERENT_BIT | glow::MAP_READ_BIT | glow::MAP_WRITE_BIT,
            glow::MAP_READ_BIT | glow::MAP_WRITE_BIT | glow::MAP_PERSISTENT_BIT,
            glow::DYNAMIC_DRAW
        ),
        crate::Memory::Upload => (
            glow::MAP_PERSISTENT_BIT | glow::MAP_COHERENT_BIT | glow::MAP_WRITE_BIT,
            glow::MAP_WRITE_BIT | glow::MAP_PERSISTENT_BIT | glow::MAP_UNSYNCHRONIZED_BIT,
            glow::DYNAMIC_DRAW
        ),
        crate::Memory::External(_) => unimplemented!(),
    };

    unsafe {
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(raw));

        if self.capabilities.contains(Capabilities::BUFFER_STORAGE) {
            // Modern path: persistent mapping
            gl.buffer_storage(glow::ARRAY_BUFFER, desc.size as _, None, storage_flags);
            if map_flags != 0 {
                data = gl.map_buffer_range(glow::ARRAY_BUFFER, 0, desc.size as _, map_flags);
            }
        } else {
            // Fallback: host-allocated buffer (leaked Vec)
            gl.buffer_data_size(glow::ARRAY_BUFFER, desc.size as _, usage);
            let data_vec = vec![0; desc.size as usize];
            data = Vec::leak(data_vec).as_mut_ptr();
        }
    }

    Buffer { raw, size: desc.size, data }
}
```

### 6.2 Buffer Sync (Fallback Path)

**File**: `resource.rs:93-105`

```rust
fn sync_buffer(&self, buffer: super::Buffer) {
    if !self.capabilities.contains(Capabilities::BUFFER_STORAGE) {
        let gl = self.lock();
        unsafe {
            let data = slice::from_raw_parts(buffer.data, buffer.size as usize);
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(buffer.raw));
            gl.buffer_sub_data_u8_slice(glow::ARRAY_BUFFER, 0, data);
        }
    }
}
```

### 6.3 Texture Creation

**File**: `resource.rs:121-263`

```rust
enum TextureInner {
    Renderbuffer { raw: glow::Renderbuffer },
    Texture { raw: glow::Texture, target: BindTarget },
}

fn create_texture(&self, desc: crate::TextureDesc) -> super::Texture {
    let format_desc = describe_texture_format(desc.format);

    let inner = if crate::TextureUsage::TARGET.contains(desc.usage)
        && desc.dimension == crate::TextureDimension::D2
        && desc.array_layer_count == 1
    {
        // Renderbuffer path: more efficient for render targets
        let raw = gl.create_renderbuffer().unwrap();
        gl.bind_renderbuffer(glow::RENDERBUFFER, Some(raw));

        if desc.sample_count <= 1 {
            gl.renderbuffer_storage(glow::RENDERBUFFER, format_desc.internal, ...);
        } else {
            gl.renderbuffer_storage_multisample(glow::RENDERBUFFER, desc.sample_count, ...);
        }

        TextureInner::Renderbuffer { raw }
    } else {
        // Texture path: general textures with sampling
        let raw = gl.create_texture().unwrap();

        let target = match (desc.dimension, desc.array_layer_count, desc.sample_count) {
            (D1, 1, _) => glow::TEXTURE_1D,
            (D1, _, _) => glow::TEXTURE_1D_ARRAY,
            (D2, 1, 1) => glow::TEXTURE_2D,
            (D2, 1, _) => glow::TEXTURE_2D_MULTISAMPLE,
            (D2, _, 1) => glow::TEXTURE_2D_ARRAY,
            (D2, _, _) => glow::TEXTURE_2D_MULTISAMPLE_ARRAY,
            (D3, _, _) => glow::TEXTURE_3D,
        };

        gl.bind_texture(target, Some(raw));
        gl.tex_parameter_i32(target, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
        gl.tex_parameter_i32(target, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);

        // Immutable storage allocation
        match desc.dimension {
            D1 => gl.tex_storage_1d(target, desc.mip_level_count, format_desc.internal, ...),
            D2 => gl.tex_storage_2d(target, desc.mip_level_count, format_desc.internal, ...),
            D3 => gl.tex_storage_3d(target, desc.mip_level_count, format_desc.internal, ...),
        }

        TextureInner::Texture { raw, target }
    };

    Texture { inner, target_size, format: desc.format }
}
```

### 6.4 Texture Format Mapping

**File**: `mod.rs:587-655`

```rust
struct FormatInfo {
    internal: u32,   // GL internal format (e.g., GL_RGBA8)
    external: u32,   // GL external format (e.g., GL_RGBA)
    data_type: u32,  // GL data type (e.g., GL_UNSIGNED_BYTE)
}

fn describe_texture_format(format: crate::TextureFormat) -> FormatInfo {
    match format {
        // Color formats
        Tf::R8Unorm => (glow::R8, glow::RED, glow::UNSIGNED_BYTE),
        Tf::Rg8Unorm => (glow::RG8, glow::RG, glow::UNSIGNED_BYTE),
        Tf::Rgba8Unorm => (glow::RGBA8, glow::RGBA, glow::UNSIGNED_BYTE),
        Tf::Rgba8UnormSrgb => (glow::SRGB8_ALPHA8, glow::RGBA, glow::UNSIGNED_BYTE),
        Tf::Rgba16Float => (glow::RGBA16F, glow::RGBA, glow::HALF_FLOAT),
        Tf::Rgba32Float => (glow::RGBA32F, glow::RGBA, glow::FLOAT),

        // Depth/Stencil
        Tf::Depth32Float => (glow::DEPTH_COMPONENT32F, glow::DEPTH_COMPONENT, glow::FLOAT),
        Tf::Depth32FloatStencil8Uint => (glow::DEPTH32F_STENCIL8, glow::DEPTH_STENCIL, ...),

        // Compressed (BC/S3TC)
        Tf::Bc1Unorm => (glow::COMPRESSED_RGBA_S3TC_DXT1_EXT, glow::RGBA, 0),
        Tf::Bc7Unorm => (glow::COMPRESSED_RGBA_BPTC_UNORM, glow::RGBA, 0),

        // Packed
        Tf::Rgb10a2Unorm => (glow::RGB10_A2, glow::RGBA, glow::UNSIGNED_INT_2_10_10_10_REV),
        Tf::Rg11b10Ufloat => (glow::R11F_G11F_B10F, glow::RGB, ...),
        // ...
    }
}
```

### 6.5 Sampler Creation

**File**: `resource.rs:292-349`

```rust
fn create_sampler(&self, desc: crate::SamplerDesc) -> super::Sampler {
    let raw = gl.create_sampler().unwrap();

    let (min, mag) = map_filter_modes(desc.min_filter, desc.mag_filter, desc.mipmap_filter);

    gl.sampler_parameter_i32(raw, glow::TEXTURE_MIN_FILTER, min as i32);
    gl.sampler_parameter_i32(raw, glow::TEXTURE_MAG_FILTER, mag as i32);

    for (&address_mode, wrap_enum) in desc.address_modes.iter().zip([WRAP_S, WRAP_T, WRAP_R]) {
        gl.sampler_parameter_i32(raw, wrap_enum, map_address_mode(address_mode) as i32);
    }

    if desc.border_color.is_some() {
        gl.sampler_parameter_f32_slice(raw, glow::TEXTURE_BORDER_COLOR, &border);
    }

    gl.sampler_parameter_f32(raw, glow::TEXTURE_MIN_LOD, desc.lod_min_clamp);
    if let Some(clamp) = desc.lod_max_clamp {
        gl.sampler_parameter_f32(raw, glow::TEXTURE_MAX_LOD, clamp);
    }

    if desc.anisotropy_clamp > 1 {
        gl.sampler_parameter_i32(raw, glow::TEXTURE_MAX_ANISOTROPY, desc.anisotropy_clamp as i32);
    }

    if let Some(compare) = desc.compare {
        gl.sampler_parameter_i32(raw, glow::TEXTURE_COMPARE_MODE, glow::COMPARE_REF_TO_TEXTURE);
        gl.sampler_parameter_i32(raw, glow::TEXTURE_COMPARE_FUNC, map_compare_func(compare));
    }

    Sampler { raw }
}
```

---

## 7. Data Binding

### 7.1 ShaderBindable Trait Implementations

**File**: `command.rs:10-81`

```rust
// Plain data (uniforms) - packed into buffer
impl<T: bytemuck::Pod> crate::ShaderBindable for T {
    fn bind_to(&self, ctx: &mut super::PipelineContext, index: u32) {
        let self_slice = bytemuck::bytes_of(self);

        // Align to UBO requirements
        let alignment = ctx.limits.uniform_buffer_alignment as usize;
        let rem = ctx.plain_data.len() % alignment;
        if rem != 0 {
            ctx.plain_data.resize(ctx.plain_data.len() - rem + alignment, 0);
        }

        let offset = ctx.plain_data.len() as u32;
        let size = round_up_uniform_size(self_slice.len() as u32);
        ctx.plain_data.extend_from_slice(self_slice);
        ctx.plain_data.extend((self_slice.len() as u32..size).map(|_| 0));

        for &slot in ctx.targets[index as usize].iter() {
            ctx.commands.push(Command::BindUniform { slot, offset, size });
        }
    }
}

// Textures
impl crate::ShaderBindable for super::TextureView {
    fn bind_to(&self, ctx: &mut super::PipelineContext, index: u32) {
        let (texture, target) = self.inner.as_native();
        for &slot in ctx.targets[index as usize].iter() {
            ctx.commands.push(Command::BindTexture { slot, texture, target });
        }
    }
}

// Samplers
impl crate::ShaderBindable for super::Sampler {
    fn bind_to(&self, ctx: &mut super::PipelineContext, index: u32) {
        for &slot in ctx.targets[index as usize].iter() {
            ctx.commands.push(Command::BindSampler { slot, sampler: self.raw });
        }
    }
}

// Buffers (SSBO)
impl crate::ShaderBindable for crate::BufferPiece {
    fn bind_to(&self, ctx: &mut super::PipelineContext, index: u32) {
        for &slot in ctx.targets[index as usize].iter() {
            ctx.commands.push(Command::BindBuffer {
                target: glow::SHADER_STORAGE_BUFFER,
                slot,
                buffer: (*self).into(),
                size: (self.buffer.size - self.offset) as u32,
            });
        }
    }
}
```

---

## 8. Synchronization

### 8.1 Fence-Based Sync

**File**: `mod.rs:432-436, 557-575`

```rust
#[derive(Clone, Debug)]
pub struct SyncPoint {
    fence: glow::Fence,
}
// TODO: destructor

fn wait_for(&self, sp: &SyncPoint, timeout_ms: u32) -> bool {
    let gl = self.lock();
    let timeout_ns = if timeout_ms == !0 { !0 } else { timeout_ms as u64 * 1_000_000 };

    // WebGL constraint: max 1 second timeout
    let timeout_ns_i32 = timeout_ns.min(MAX_TIMEOUT) as i32;

    let status = unsafe {
        gl.client_wait_sync(sp.fence, glow::SYNC_FLUSH_COMMANDS_BIT, timeout_ns_i32)
    };

    match status {
        glow::ALREADY_SIGNALED | glow::CONDITION_SATISFIED => true,
        _ => false,
    }
}
```

### 8.2 Constants

**File**: `mod.rs:12`

```rust
const MAX_TIMEOUT: u64 = 1_000_000_000; // MAX_CLIENT_WAIT_TIMEOUT_WEBGL (1 second)
```

---

## 9. Timing and Debug Support

### 9.1 GPU Timing

**File**: `command.rs:84-153`

```rust
struct TimingData {
    pass_names: Vec<String>,
    queries: Box<[glow::Query]>,
}

fn begin_pass(&mut self, label: &str) {
    if let Some(ref mut timing_datas) = self.timing_datas {
        let td = timing_datas.first_mut().unwrap();
        let id = td.pass_names.len();
        self.commands.push(Command::QueryCounter { query: td.queries[id] });
        td.pass_names.push(label.to_string());
    }
}

pub(super) fn finish(&mut self, gl: &glow::Context) {
    if let Some(ref mut timing_datas) = self.timing_datas {
        // Record final timestamp
        let td = timing_datas.first_mut().unwrap();
        self.commands.push(Command::QueryCounter { query: td.queries[td.pass_names.len()] });

        // Rotate and resolve previous frame's timings
        timing_datas.rotate_left(1);
        let td = timing_datas.first_mut().unwrap();

        let mut prev = 0;
        gl.get_query_parameter_u64_with_offset(td.queries[0], glow::QUERY_RESULT, &mut prev);

        for (pass_name, &query) in td.pass_names.drain(..).zip(td.queries[1..].iter()) {
            let mut result: u64 = 0;
            gl.get_query_parameter_u64_with_offset(query, glow::QUERY_RESULT, &mut result);
            self.timings.push((pass_name, Duration::from_nanos(result - prev)));
            prev = result;
        }
    }
}
```

### 9.2 Debug Scopes

```rust
if self.needs_scopes {
    self.commands.push(Command::PushScope { name_range: start..end });
}

// Executed as:
gl.push_debug_group(glow::DEBUG_SOURCE_APPLICATION, DEBUG_ID, name);
// ... commands ...
gl.pop_debug_group();
```

---

## 10. Y-Flip Handling

**File**: `mod.rs:671-696`

OpenGL's coordinate system is Y-flipped relative to other APIs. Blade handles this with:

1. **Shader output**: `ADJUST_COORDINATE_SPACE` flag in Naga
2. **Present blit**: Y-flip during framebuffer blit

```rust
unsafe fn present_blit(gl: &glow::Context, source: glow::Framebuffer, size: crate::Extent) {
    gl.disable(glow::SCISSOR_TEST);
    gl.color_mask(true, true, true, true);
    gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, None);
    gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(source));

    // Y-flip: source (0, height) → (width, 0) maps to dest (0, 0) → (width, height)
    gl.blit_framebuffer(
        0, size.height as i32, size.width as i32, 0,  // Source: flipped
        0, 0, size.width as i32, size.height as i32,  // Dest: normal
        glow::COLOR_BUFFER_BIT,
        glow::NEAREST,
    );
}
```

---

## 11. Limitations and Unimplemented Features

### Unimplemented (`unimplemented!()`)

| Feature | Location | Reason |
|---------|----------|--------|
| Acceleration structures | `resource.rs:5-26, 356-368` | No ray tracing in GLES |
| Texture arrays binding | `command.rs:43-47` | Complex implementation |
| Buffer arrays binding | `command.rs:70-73` | Complex implementation |
| `draw_indirect` | `command.rs:540-541` | Not implemented |
| `draw_indexed_indirect` | `command.rs:544-551` | Not implemented |
| `fill_buffer` | `command.rs:699-703` | Not implemented |
| `set_stencil_reference` | `command.rs:313-315, 477-479` | Not implemented |
| `acceleration_structure` pass | `command.rs:160-162` | No ray tracing |
| Barrier command | Execution skipped | Implicit barriers used |

### Assertions/Constraints

| Constraint | Location | Reason |
|------------|----------|--------|
| Single vertex buffer | `command.rs:487` | `assert_eq!(index, 0)` |
| No start_instance | `command.rs:511, 529` | `assert_eq!(start_instance, 0)` |
| MS textures: mip=1 | `resource.rs:227` | OpenGL requirement |

### WebGL-Specific

- No `glObjectLabel` (debug names)
- No BUFFER_STORAGE (persistent mapping)
- Max sync timeout: 1 second
- No compute shaders (WebGL2 limitation)

---

## 12. Comparison: GLES vs Vulkan

| Aspect | GLES Backend | Vulkan Backend |
|--------|--------------|----------------|
| **Command Model** | Deferred (`Vec<Command>`) | Immediate (`vk::CommandBuffer`) |
| **Shader Format** | GLSL (via Naga) | SPIR-V (native) |
| **Binding Model** | Slot-based, per-command | Descriptor sets |
| **Synchronization** | GL fences | Timeline semaphores |
| **Memory Model** | GL allocates | Explicit gpu-alloc |
| **Thread Safety** | Mutex-locked context | Queue-based |
| **Binding Resolution** | Compile-time or reflection | Descriptor set layouts |
| **Ray Tracing** | Not supported | Full support |
| **Indirect Drawing** | Partial | Full support |
| **Multi-draw** | Limited | Full support |
| **Buffer Mapping** | Persistent (ES320) or copy | Always persistent |

---

## 13. Dependencies

```toml
[target.'cfg(any(gles, target_arch = "wasm32"))'.dependencies]
glow = "0.16"              # OpenGL bindings
naga = { features = ["glsl-out"] }  # Shader compilation

# Native only
egl = "0.2"               # EGL bindings
libloading = "0.8"        # Dynamic library loading

# WASM only
wasm-bindgen = "0.2"
web-sys = { features = ["Window", "Document", "HtmlCanvasElement", "WebGl2RenderingContext"] }
js-sys = "0.3"
```

---

*Generated for blade-graphics v0.7.0 GLES backend*
