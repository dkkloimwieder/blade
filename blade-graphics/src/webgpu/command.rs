//! Command encoding for WebGPU backend
//!
//! Implements deferred command recording pattern following GLES backend.

use super::*;

//=============================================================================
// ShaderBindable Implementations
//=============================================================================

/// Round up uniform size to alignment
fn round_up_uniform_size(size: u32, alignment: u32) -> u32 {
    let mask = alignment - 1;
    (size + mask) & !mask
}

impl<T: bytemuck::Pod> crate::ShaderBindable for T {
    fn bind_to(&self, ctx: &mut PipelineContext, index: u32) {
        let self_slice = bytemuck::bytes_of(self);
        let alignment = ctx.limits.uniform_buffer_alignment as usize;
        let rem = ctx.plain_data.len() % alignment;
        if rem != 0 {
            ctx.plain_data
                .resize(ctx.plain_data.len() - rem + alignment, 0);
        }
        let offset = ctx.plain_data.len() as u32;
        let size = round_up_uniform_size(self_slice.len() as u32, ctx.limits.uniform_buffer_alignment);
        ctx.plain_data.extend_from_slice(self_slice);
        ctx.plain_data
            .extend((self_slice.len() as u32..size).map(|_| 0));

        for slot in ctx.targets[index as usize].iter() {
            ctx.commands.push(Command::RecordBindGroup {
                group_index: slot.group,
                entries: vec![BindGroupEntry::PlainData {
                    binding: slot.binding,
                    offset,
                    size,
                }],
            });
        }
    }
}

impl crate::ShaderBindable for TextureView {
    fn bind_to(&self, ctx: &mut PipelineContext, index: u32) {
        for slot in ctx.targets[index as usize].iter() {
            ctx.commands.push(Command::RecordBindGroup {
                group_index: slot.group,
                entries: vec![BindGroupEntry::Texture {
                    binding: slot.binding,
                    view_key: self.raw,
                }],
            });
        }
    }
}

impl<'a, const N: crate::ResourceIndex> crate::ShaderBindable for &'a crate::TextureArray<N> {
    fn bind_to(&self, _ctx: &mut PipelineContext, _index: u32) {
        unimplemented!("TextureArray binding not supported in WebGPU base spec")
    }
}

impl crate::ShaderBindable for Sampler {
    fn bind_to(&self, ctx: &mut PipelineContext, index: u32) {
        for slot in ctx.targets[index as usize].iter() {
            ctx.commands.push(Command::RecordBindGroup {
                group_index: slot.group,
                entries: vec![BindGroupEntry::Sampler {
                    binding: slot.binding,
                    sampler_key: self.raw,
                }],
            });
        }
    }
}

impl crate::ShaderBindable for crate::BufferPiece {
    fn bind_to(&self, ctx: &mut PipelineContext, index: u32) {
        for slot in ctx.targets[index as usize].iter() {
            ctx.commands.push(Command::RecordBindGroup {
                group_index: slot.group,
                entries: vec![BindGroupEntry::Buffer {
                    binding: slot.binding,
                    buffer_key: self.buffer.raw,
                    offset: self.offset,
                    size: self.buffer.size - self.offset,
                }],
            });
        }
    }
}

impl<'a, const N: crate::ResourceIndex> crate::ShaderBindable for &'a crate::BufferArray<N> {
    fn bind_to(&self, _ctx: &mut PipelineContext, _index: u32) {
        unimplemented!("BufferArray binding not supported in WebGPU base spec")
    }
}

impl crate::ShaderBindable for AccelerationStructure {
    fn bind_to(&self, _ctx: &mut PipelineContext, _index: u32) {
        panic!("AccelerationStructure not supported in WebGPU backend")
    }
}

//=============================================================================
// Command Types
//=============================================================================

#[derive(Clone, Debug)]
pub(super) struct BufferPart {
    pub key: BufferKey,
    pub offset: u64,
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
pub(super) struct TexturePart {
    pub key: TextureKey,
    pub format: crate::TextureFormat,
    pub mip_level: u32,
    pub array_layer: u32,
    pub origin: [u32; 3],
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
pub(super) enum Command {
    // Transfer commands
    FillBuffer {
        dst: BufferPart,
        size: u64,
        value: u8,
    },
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

    // Render pass commands
    BeginRenderPass {
        color_attachments: Vec<RenderColorAttachment>,
        depth_attachment: Option<RenderDepthAttachment>,
    },
    EndRenderPass,
    SetRenderPipeline {
        key: RenderPipelineKey,
    },
    SetViewport {
        viewport: crate::Viewport,
    },
    SetScissor {
        rect: crate::ScissorRect,
    },
    SetStencilReference {
        reference: u32,
    },
    SetVertexBuffer {
        slot: u32,
        buffer: BufferPart,
    },
    SetBindGroup {
        index: u32,
        bind_group_id: u32,  // Index into ephemeral bind groups created at submit
    },
    Draw {
        first_vertex: u32,
        vertex_count: u32,
        first_instance: u32,
        instance_count: u32,
    },
    DrawIndexed {
        index_buffer: BufferPart,
        index_format: crate::IndexType,
        index_count: u32,
        base_vertex: i32,
        first_instance: u32,
        instance_count: u32,
    },
    DrawIndirect {
        indirect_buffer: BufferPart,
    },
    DrawIndexedIndirect {
        index_buffer: BufferPart,
        index_format: crate::IndexType,
        indirect_buffer: BufferPart,
    },

    // Compute pass commands
    BeginComputePass,
    EndComputePass,
    SetComputePipeline {
        key: ComputePipelineKey,
    },
    Dispatch {
        groups: [u32; 3],
    },
    DispatchIndirect {
        indirect_buffer: BufferPart,
    },

    // Bind group recording (stored during bind, resolved at submit)
    RecordBindGroup {
        group_index: u32,
        entries: Vec<BindGroupEntry>,
    },

    // Texture initialization
    InitTexture {
        key: TextureKey,
    },
}

/// Render pass color attachment
#[derive(Debug)]
pub(super) struct RenderColorAttachment {
    pub view_key: TextureViewKey,
    pub load_op: crate::InitOp,
    pub store_op: crate::FinishOp,
    /// For frame targets, store the raw wgpu view
    pub frame_view: Option<std::sync::Arc<wgpu::TextureView>>,
    /// MSAA resolve target (from FinishOp::ResolveTo)
    pub resolve_target: Option<TextureViewKey>,
}

/// Render pass depth attachment
#[derive(Debug)]
pub(super) struct RenderDepthAttachment {
    pub view_key: TextureViewKey,
    pub depth_load_op: crate::InitOp,
    pub depth_store_op: crate::FinishOp,
    pub stencil_load_op: crate::InitOp,
    pub stencil_store_op: crate::FinishOp,
}

/// Bind group entry for deferred binding
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub(super) enum BindGroupEntry {
    Buffer {
        binding: u32,
        buffer_key: BufferKey,
        offset: u64,
        size: u64,
    },
    Texture {
        binding: u32,
        view_key: TextureViewKey,
    },
    Sampler {
        binding: u32,
        sampler_key: SamplerKey,
    },
    PlainData {
        binding: u32,
        offset: u32,
        size: u32,
    },
}

impl BindGroupEntry {
    fn binding_index(&self) -> u32 {
        match self {
            BindGroupEntry::Buffer { binding, .. } => *binding,
            BindGroupEntry::Texture { binding, .. } => *binding,
            BindGroupEntry::Sampler { binding, .. } => *binding,
            BindGroupEntry::PlainData { binding, .. } => *binding,
        }
    }
}

//=============================================================================
// Command Encoder
//=============================================================================

pub struct CommandEncoder {
    pub(super) name: String,
    pub(super) commands: Vec<Command>,
    pub(super) plain_data: Vec<u8>,
    pub(super) present_frames: Vec<Frame>,
    pub(super) limits: Limits,
}

//=============================================================================
// Pass Encoders
//=============================================================================

pub(super) enum PassKind {
    Transfer,
    Compute,
    Render,
}

pub struct PassEncoder<'a, P> {
    commands: &'a mut Vec<Command>,
    plain_data: &'a mut Vec<u8>,
    kind: PassKind,
    pipeline: PhantomData<P>,
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
    pub(super) commands: &'a mut Vec<Command>,
    pub(super) plain_data: &'a mut Vec<u8>,
    pub(super) targets: &'a [SlotList],
    pub(super) limits: &'a Limits,
}

//=============================================================================
// CommandEncoder Implementation
//=============================================================================

impl CommandEncoder {
    /// Create a new command encoder
    pub(super) fn new(name: String, limits: Limits) -> Self {
        Self {
            name,
            commands: Vec::new(),
            plain_data: Vec::new(),
            present_frames: Vec::new(),
            limits,
        }
    }

    /// Helper to create a pass encoder
    fn pass<P>(&mut self, kind: PassKind) -> PassEncoder<'_, P> {
        PassEncoder {
            commands: &mut self.commands,
            plain_data: &mut self.plain_data,
            kind,
            pipeline: PhantomData,
            limits: &self.limits,
        }
    }

    /// Create a transfer pass
    pub fn transfer(&mut self, _label: &str) -> TransferCommandEncoder<'_> {
        self.pass(PassKind::Transfer)
    }

    /// Create a compute pass
    pub fn compute(&mut self, _label: &str) -> ComputeCommandEncoder<'_> {
        self.commands.push(Command::BeginComputePass);
        self.pass(PassKind::Compute)
    }

    /// Create a render pass
    pub fn render(
        &mut self,
        _label: &str,
        targets: crate::RenderTargetSet,
    ) -> RenderCommandEncoder<'_> {
        // Build color attachments
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
                    resolve_target,
                }
            })
            .collect();

        // Build depth attachment if present
        // Note: RenderTarget uses init_op/finish_op for both depth and stencil
        let depth_attachment = targets.depth_stencil.as_ref().map(|ds| RenderDepthAttachment {
            view_key: ds.view.raw,
            depth_load_op: ds.init_op,
            depth_store_op: ds.finish_op,
            stencil_load_op: ds.init_op,
            stencil_store_op: ds.finish_op,
        });

        self.commands.push(Command::BeginRenderPass {
            color_attachments,
            depth_attachment,
        });

        self.pass(PassKind::Render)
    }

    /// Acceleration structure encoder (not supported in WebGPU)
    pub fn acceleration_structure(&mut self, _label: &str) -> TransferCommandEncoder<'_> {
        panic!("Acceleration structures are not supported in WebGPU backend")
    }
}

//=============================================================================
// CommandEncoder Trait Implementation
//=============================================================================

#[hidden_trait::expose]
impl crate::traits::CommandEncoder for CommandEncoder {
    type Texture = Texture;
    type Frame = Frame;

    fn start(&mut self) {
        self.commands.clear();
        self.plain_data.clear();
        self.present_frames.clear();
    }

    fn init_texture(&mut self, texture: Texture) {
        self.commands.push(Command::InitTexture { key: texture.raw });
    }

    fn present(&mut self, frame: Frame) {
        self.present_frames.push(frame);
    }

    fn timings(&self) -> &crate::Timings {
        // Return static empty - WebGPU timing is limited
        static EMPTY: crate::Timings = Vec::new();
        &EMPTY
    }
}

//=============================================================================
// TransferEncoder Implementation
//=============================================================================

#[hidden_trait::expose]
impl crate::traits::TransferEncoder for TransferCommandEncoder<'_> {
    type BufferPiece = crate::BufferPiece;
    type TexturePiece = crate::TexturePiece;

    fn fill_buffer(&mut self, dst: crate::BufferPiece, size: u64, value: u8) {
        self.commands.push(Command::FillBuffer {
            dst: dst.into(),
            size,
            value,
        });
    }

    fn copy_buffer_to_buffer(
        &mut self,
        src: crate::BufferPiece,
        dst: crate::BufferPiece,
        size: u64,
    ) {
        self.commands.push(Command::CopyBufferToBuffer {
            src: src.into(),
            dst: dst.into(),
            size,
        });
    }

    fn copy_texture_to_texture(
        &mut self,
        src: crate::TexturePiece,
        dst: crate::TexturePiece,
        size: crate::Extent,
    ) {
        self.commands.push(Command::CopyTextureToTexture {
            src: src.into(),
            dst: dst.into(),
            size,
        });
    }

    fn copy_buffer_to_texture(
        &mut self,
        src: crate::BufferPiece,
        bytes_per_row: u32,
        dst: crate::TexturePiece,
        size: crate::Extent,
    ) {
        self.commands.push(Command::CopyBufferToTexture {
            src: src.into(),
            bytes_per_row,
            dst: dst.into(),
            size,
        });
    }

    fn copy_texture_to_buffer(
        &mut self,
        src: crate::TexturePiece,
        dst: crate::BufferPiece,
        bytes_per_row: u32,
        size: crate::Extent,
    ) {
        self.commands.push(Command::CopyTextureToBuffer {
            src: src.into(),
            dst: dst.into(),
            bytes_per_row,
            size,
        });
    }
}

//=============================================================================
// RenderEncoder Implementation
//=============================================================================

#[hidden_trait::expose]
impl crate::traits::RenderEncoder for RenderCommandEncoder<'_> {
    fn set_scissor_rect(&mut self, rect: &crate::ScissorRect) {
        self.commands.push(Command::SetScissor { rect: rect.clone() });
    }

    fn set_viewport(&mut self, viewport: &crate::Viewport) {
        self.commands.push(Command::SetViewport { viewport: viewport.clone() });
    }

    fn set_stencil_reference(&mut self, reference: u32) {
        self.commands.push(Command::SetStencilReference { reference });
    }
}

//=============================================================================
// PipelineEncoder Implementation
//=============================================================================

impl ComputeCommandEncoder<'_> {
    /// Bind a compute pipeline and return a pipeline encoder
    pub fn with<'p>(&'p mut self, pipeline: &'p ComputePipeline) -> PipelineEncoder<'p> {
        self.commands.push(Command::SetComputePipeline { key: pipeline.raw });

        PipelineEncoder {
            commands: self.commands,
            plain_data: self.plain_data,
            group_mappings: &pipeline.group_mappings,
            limits: self.limits,
        }
    }
}

impl RenderCommandEncoder<'_> {
    /// Bind a render pipeline and return a pipeline encoder
    pub fn with<'p>(&'p mut self, pipeline: &'p RenderPipeline) -> PipelineEncoder<'p> {
        self.commands.push(Command::SetRenderPipeline { key: pipeline.raw });

        PipelineEncoder {
            commands: self.commands,
            plain_data: self.plain_data,
            group_mappings: &pipeline.group_mappings,
            limits: self.limits,
        }
    }
}

#[hidden_trait::expose]
impl crate::traits::PipelineEncoder for PipelineEncoder<'_> {
    fn bind<D: crate::ShaderData>(&mut self, group: u32, data: &D) {
        // Use pre-computed group mappings from pipeline
        let targets = &self.group_mappings[group as usize].targets;

        let ctx = PipelineContext {
            commands: self.commands,
            plain_data: self.plain_data,
            targets,
            limits: self.limits,
        };

        data.fill(ctx);
    }
}

// PipelineEncoder also needs RenderEncoder for RenderPipelineEncoder bound
#[hidden_trait::expose]
impl crate::traits::RenderEncoder for PipelineEncoder<'_> {
    fn set_scissor_rect(&mut self, rect: &crate::ScissorRect) {
        self.commands.push(Command::SetScissor { rect: rect.clone() });
    }

    fn set_viewport(&mut self, viewport: &crate::Viewport) {
        self.commands.push(Command::SetViewport { viewport: viewport.clone() });
    }

    fn set_stencil_reference(&mut self, reference: u32) {
        self.commands.push(Command::SetStencilReference { reference });
    }
}

#[hidden_trait::expose]
impl crate::traits::ComputePipelineEncoder for PipelineEncoder<'_> {
    type BufferPiece = crate::BufferPiece;

    fn dispatch(&mut self, groups: [u32; 3]) {
        self.commands.push(Command::Dispatch { groups });
    }

    fn dispatch_indirect(&mut self, indirect_buf: crate::BufferPiece) {
        self.commands.push(Command::DispatchIndirect {
            indirect_buffer: indirect_buf.into(),
        });
    }
}

#[hidden_trait::expose]
impl crate::traits::RenderPipelineEncoder for PipelineEncoder<'_> {
    type BufferPiece = crate::BufferPiece;

    fn bind_vertex(&mut self, index: u32, vertex_buf: crate::BufferPiece) {
        self.commands.push(Command::SetVertexBuffer {
            slot: index,
            buffer: vertex_buf.into(),
        });
    }

    fn draw(
        &mut self,
        first_vertex: u32,
        vertex_count: u32,
        first_instance: u32,
        instance_count: u32,
    ) {
        self.commands.push(Command::Draw {
            first_vertex,
            vertex_count,
            first_instance,
            instance_count,
        });
    }

    fn draw_indexed(
        &mut self,
        index_buf: crate::BufferPiece,
        index_type: crate::IndexType,
        index_count: u32,
        base_vertex: i32,
        start_instance: u32,
        instance_count: u32,
    ) {
        self.commands.push(Command::DrawIndexed {
            index_buffer: index_buf.into(),
            index_format: index_type,
            index_count,
            base_vertex,
            first_instance: start_instance,
            instance_count,
        });
    }

    fn draw_indirect(&mut self, indirect_buf: crate::BufferPiece) {
        self.commands.push(Command::DrawIndirect {
            indirect_buffer: indirect_buf.into(),
        });
    }

    fn draw_indexed_indirect(
        &mut self,
        index_buf: crate::BufferPiece,
        index_type: crate::IndexType,
        indirect_buf: crate::BufferPiece,
    ) {
        self.commands.push(Command::DrawIndexedIndirect {
            index_buffer: index_buf.into(),
            index_format: index_type,
            indirect_buffer: indirect_buf.into(),
        });
    }
}

//=============================================================================
// Pass Drop Handling
//=============================================================================

impl<P> Drop for PassEncoder<'_, P> {
    fn drop(&mut self) {
        match self.kind {
            PassKind::Transfer => {
                // Transfer passes don't need explicit end
            }
            PassKind::Compute => {
                self.commands.push(Command::EndComputePass);
            }
            PassKind::Render => {
                self.commands.push(Command::EndRenderPass);
            }
        }
    }
}

//=============================================================================
// CommandDevice Implementation
//=============================================================================

#[hidden_trait::expose]
impl crate::traits::CommandDevice for Context {
    type CommandEncoder = CommandEncoder;
    type SyncPoint = SyncPoint;

    fn create_command_encoder(&self, desc: crate::CommandEncoderDesc) -> CommandEncoder {
        CommandEncoder::new(desc.name.to_string(), self.limits.clone())
    }

    fn destroy_command_encoder(&self, _encoder: &mut CommandEncoder) {
        // No explicit cleanup needed - Rust drops automatically
    }

    fn submit(&self, encoder: &mut CommandEncoder) -> SyncPoint {
        // 1. Sync all dirty shadow buffers to GPU
        self.sync_dirty_buffers();

        // 2. Create wgpu command encoder
        let mut cmd_encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some(&encoder.name),
        });

        // 3. Execute recorded commands with bind group cache and reusable uniform buffer
        let hub = self.hub.read().unwrap();
        let mut cache = self.bind_group_cache.write().unwrap();
        let mut uniform_buffer = self.uniform_buffer.write().unwrap();
        self.execute_commands(&hub, &mut cache, &mut uniform_buffer, &mut cmd_encoder, &encoder.commands, &encoder.plain_data);
        drop(uniform_buffer);
        drop(cache);
        drop(hub);

        // 4. Submit to queue
        let submission_index = self.queue.submit(std::iter::once(cmd_encoder.finish()));

        // 5. Present frames and cleanup their views from hub
        {
            let mut hub = self.hub.write().unwrap();
            for frame in encoder.present_frames.drain(..) {
                // Remove frame view from hub (it was added in acquire_frame)
                if let Some(view_key) = frame.view_key {
                    hub.texture_views.remove(view_key);
                }
                frame.texture.present();
            }
        }

        SyncPoint { submission_index }
    }

    fn wait_for(&self, sp: &SyncPoint, timeout_ms: u32) -> bool {
        // wgpu v28: use PollType::Wait with submission_index
        let timeout = std::time::Duration::from_millis(timeout_ms as u64);
        self.device.poll(wgpu::PollType::Wait {
            submission_index: Some(sp.submission_index.clone()),
            timeout: Some(timeout),
        }).is_ok()
    }
}

/// Map InitOp to wgpu LoadOp
fn map_load_op(op: &crate::InitOp) -> wgpu::LoadOp<wgpu::Color> {
    match op {
        crate::InitOp::Load => wgpu::LoadOp::Load,
        crate::InitOp::Clear(color) => {
            let (r, g, b, a) = match color {
                crate::TextureColor::TransparentBlack => (0.0, 0.0, 0.0, 0.0),
                crate::TextureColor::OpaqueBlack => (0.0, 0.0, 0.0, 1.0),
                crate::TextureColor::White => (1.0, 1.0, 1.0, 1.0),
            };
            wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a })
        }
        crate::InitOp::DontCare => wgpu::LoadOp::Clear(wgpu::Color::BLACK),
    }
}

/// Map InitOp to depth LoadOp (f32 clear value)
fn map_depth_load_op(op: &crate::InitOp) -> wgpu::LoadOp<f32> {
    match op {
        crate::InitOp::Load => wgpu::LoadOp::Load,
        crate::InitOp::Clear(_) => wgpu::LoadOp::Clear(1.0), // Depth typically cleared to 1.0
        crate::InitOp::DontCare => wgpu::LoadOp::Clear(1.0),
    }
}

/// Map InitOp to stencil LoadOp (u32 clear value)
fn map_stencil_load_op(op: &crate::InitOp) -> wgpu::LoadOp<u32> {
    match op {
        crate::InitOp::Load => wgpu::LoadOp::Load,
        crate::InitOp::Clear(_) => wgpu::LoadOp::Clear(0),
        crate::InitOp::DontCare => wgpu::LoadOp::Clear(0),
    }
}

/// Map FinishOp to wgpu StoreOp
fn map_store_op(op: &crate::FinishOp) -> wgpu::StoreOp {
    match op {
        crate::FinishOp::Store => wgpu::StoreOp::Store,
        crate::FinishOp::Discard => wgpu::StoreOp::Discard,
        crate::FinishOp::ResolveTo(_) => wgpu::StoreOp::Store, // Resolve handled separately
        crate::FinishOp::Ignore => wgpu::StoreOp::Discard,
    }
}

/// Map IndexType to wgpu IndexFormat
fn map_index_format(index_type: crate::IndexType) -> wgpu::IndexFormat {
    match index_type {
        crate::IndexType::U16 => wgpu::IndexFormat::Uint16,
        crate::IndexType::U32 => wgpu::IndexFormat::Uint32,
    }
}

/// Execution state for command processing
struct ExecutionState<'a> {
    /// Current render pipeline key (for bind group creation)
    render_pipeline_key: Option<RenderPipelineKey>,
    /// Current compute pipeline key
    compute_pipeline_key: Option<ComputePipelineKey>,
    /// Pending bind group entries per group index
    pending_bind_groups: std::collections::HashMap<u32, Vec<BindGroupEntry>>,
    /// Hub reference
    hub: &'a Hub,
    /// Device reference for creating bind groups
    device: &'a wgpu::Device,
    /// Queue reference for buffer uploads
    #[allow(dead_code)]
    queue: &'a wgpu::Queue,
    /// Plain data buffer (for uniform data) - reference to reusable buffer
    plain_data_buffer: Option<&'a wgpu::Buffer>,
    /// Bind group cache reference
    cache: &'a mut BindGroupCache,
}

impl<'a> ExecutionState<'a> {
    fn new(hub: &'a Hub, device: &'a wgpu::Device, queue: &'a wgpu::Queue, cache: &'a mut BindGroupCache) -> Self {
        Self {
            render_pipeline_key: None,
            compute_pipeline_key: None,
            pending_bind_groups: std::collections::HashMap::new(),
            hub,
            device,
            queue,
            plain_data_buffer: None,
            cache,
        }
    }

    /// Convert BindGroupEntry to ResourceBinding for cache key
    fn entry_to_binding(entry: &BindGroupEntry) -> ResourceBinding {
        match entry {
            BindGroupEntry::Buffer { binding, buffer_key, offset, size } => {
                ResourceBinding::Buffer { binding: *binding, key: *buffer_key, offset: *offset, size: *size }
            }
            BindGroupEntry::Texture { binding, view_key } => {
                ResourceBinding::TextureView { binding: *binding, key: *view_key }
            }
            BindGroupEntry::Sampler { binding, sampler_key } => {
                ResourceBinding::Sampler { binding: *binding, key: *sampler_key }
            }
            BindGroupEntry::PlainData { binding, offset, size } => {
                ResourceBinding::PlainData { binding: *binding, offset: *offset, size: *size }
            }
        }
    }

    /// Flush all pending bind groups for render pass using cache
    fn flush_render_bind_groups(&mut self, render_pass: &mut wgpu::RenderPass) {
        let pipeline_key = match self.render_pipeline_key {
            Some(k) => k,
            None => {
                self.pending_bind_groups.clear();
                return;
            }
        };

        let pipeline_entry = match self.hub.render_pipelines.get(pipeline_key) {
            Some(e) => e,
            None => {
                self.pending_bind_groups.clear();
                return;
            }
        };

        let group_indices: Vec<u32> = self.pending_bind_groups.keys().copied().collect();

        for group_index in group_indices {
            let entries = match self.pending_bind_groups.get(&group_index) {
                Some(e) => e,
                None => continue,
            };

            let layout = match pipeline_entry.bind_group_layouts.get(group_index as usize) {
                Some(l) => l,
                None => continue,
            };

            // Deduplicate entries by binding index (keep last)
            let mut seen = std::collections::HashSet::new();
            let deduped: Vec<_> = entries.iter().rev()
                .filter(|e| seen.insert(e.binding_index()))
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();

            // Check if any entry is PlainData - those can't be cached
            // because the plain_data buffer is recreated each frame
            let has_plain_data = deduped.iter().any(|e| matches!(e, BindGroupEntry::PlainData { .. }));

            let hub = self.hub;
            let device = self.device;
            let plain_data_buffer = self.plain_data_buffer;

            if has_plain_data {
                // Don't cache - create bind group directly
                let wgpu_entries: Vec<wgpu::BindGroupEntry> = deduped.iter()
                    .filter_map(|entry| make_bind_group_entry(entry, hub, plain_data_buffer))
                    .collect();

                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Ephemeral Render BindGroup"),
                    layout,
                    entries: &wgpu_entries,
                });

                render_pass.set_bind_group(group_index, &bind_group, &[]);
            } else {
                // Build cache key from deduped entries
                let bindings: Vec<ResourceBinding> = deduped.iter()
                    .map(|e| Self::entry_to_binding(e))
                    .collect();

                let cache_key = BindGroupCacheKey {
                    pipeline_key: PipelineKey::Render(pipeline_key),
                    group_index,
                    bindings,
                };

                let bind_group = self.cache.get_or_create(cache_key, || {
                    let wgpu_entries: Vec<wgpu::BindGroupEntry> = deduped.iter()
                        .filter_map(|entry| make_bind_group_entry(entry, hub, plain_data_buffer))
                        .collect();

                    device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("Cached Render BindGroup"),
                        layout,
                        entries: &wgpu_entries,
                    })
                });

                render_pass.set_bind_group(group_index, bind_group, &[]);
            }
        }

        self.pending_bind_groups.clear();
    }

    /// Flush all pending bind groups for compute pass using cache
    fn flush_compute_bind_groups(&mut self, compute_pass: &mut wgpu::ComputePass) {
        let pipeline_key = match self.compute_pipeline_key {
            Some(k) => k,
            None => {
                self.pending_bind_groups.clear();
                return;
            }
        };

        let pipeline_entry = match self.hub.compute_pipelines.get(pipeline_key) {
            Some(e) => e,
            None => {
                self.pending_bind_groups.clear();
                return;
            }
        };

        let group_indices: Vec<u32> = self.pending_bind_groups.keys().copied().collect();

        for group_index in group_indices {
            let entries = match self.pending_bind_groups.get(&group_index) {
                Some(e) => e,
                None => continue,
            };

            let layout = match pipeline_entry.bind_group_layouts.get(group_index as usize) {
                Some(l) => l,
                None => continue,
            };

            // Deduplicate entries by binding index (keep last)
            let mut seen = std::collections::HashSet::new();
            let deduped: Vec<_> = entries.iter().rev()
                .filter(|e| seen.insert(e.binding_index()))
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();

            // Check if any entry is PlainData - those can't be cached
            // because the plain_data buffer is recreated each frame
            let has_plain_data = deduped.iter().any(|e| matches!(e, BindGroupEntry::PlainData { .. }));

            let hub = self.hub;
            let device = self.device;
            let plain_data_buffer = self.plain_data_buffer;

            if has_plain_data {
                // Don't cache - create bind group directly
                let wgpu_entries: Vec<wgpu::BindGroupEntry> = deduped.iter()
                    .filter_map(|entry| make_bind_group_entry(entry, hub, plain_data_buffer))
                    .collect();

                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Ephemeral Compute BindGroup"),
                    layout,
                    entries: &wgpu_entries,
                });

                compute_pass.set_bind_group(group_index, &bind_group, &[]);
            } else {
                // Build cache key from deduped entries
                let bindings: Vec<ResourceBinding> = deduped.iter()
                    .map(|e| Self::entry_to_binding(e))
                    .collect();

                let cache_key = BindGroupCacheKey {
                    pipeline_key: PipelineKey::Compute(pipeline_key),
                    group_index,
                    bindings,
                };

                let bind_group = self.cache.get_or_create(cache_key, || {
                    let wgpu_entries: Vec<wgpu::BindGroupEntry> = deduped.iter()
                        .filter_map(|entry| make_bind_group_entry(entry, hub, plain_data_buffer))
                        .collect();

                    device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("Cached Compute BindGroup"),
                        layout,
                        entries: &wgpu_entries,
                    })
                });

                compute_pass.set_bind_group(group_index, bind_group, &[]);
            }
        }

        self.pending_bind_groups.clear();
    }
}

/// Convert a BindGroupEntry to wgpu::BindGroupEntry (standalone function for use in closures)
fn make_bind_group_entry<'a>(
    entry: &BindGroupEntry,
    hub: &'a Hub,
    plain_data_buffer: Option<&'a wgpu::Buffer>,
) -> Option<wgpu::BindGroupEntry<'a>> {
    match entry {
        BindGroupEntry::Buffer { binding, buffer_key, offset, size } => {
            let buffer_entry = hub.buffers.get(*buffer_key)?;
            Some(wgpu::BindGroupEntry {
                binding: *binding,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer_entry.gpu,
                    offset: *offset,
                    size: std::num::NonZeroU64::new(*size),
                }),
            })
        }
        BindGroupEntry::Texture { binding, view_key } => {
            let view = hub.texture_views.get(*view_key)?;
            Some(wgpu::BindGroupEntry {
                binding: *binding,
                resource: wgpu::BindingResource::TextureView(view),
            })
        }
        BindGroupEntry::Sampler { binding, sampler_key } => {
            let sampler = hub.samplers.get(*sampler_key)?;
            Some(wgpu::BindGroupEntry {
                binding: *binding,
                resource: wgpu::BindingResource::Sampler(sampler),
            })
        }
        BindGroupEntry::PlainData { binding, offset, size } => {
            let buffer = plain_data_buffer?;
            Some(wgpu::BindGroupEntry {
                binding: *binding,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer,
                    offset: *offset as u64,
                    size: std::num::NonZeroU64::new(*size as u64),
                }),
            })
        }
    }
}

impl Context {
    /// Execute recorded commands on a wgpu command encoder
    fn execute_commands(
        &self,
        hub: &Hub,
        cache: &mut BindGroupCache,
        uniform_buffer: &mut super::UniformBuffer,
        encoder: &mut wgpu::CommandEncoder,
        commands: &[Command],
        plain_data: &[u8],
    ) {
        // Reuse uniform buffer for plain data (avoids expensive create_buffer_init every frame)
        let plain_data_buffer = if !plain_data.is_empty() {
            let buffer = uniform_buffer.ensure_capacity(&self.device, plain_data.len() as u64);
            self.queue.write_buffer(buffer, 0, plain_data);
            Some(buffer)
        } else {
            None
        };

        let mut state = ExecutionState::new(hub, &self.device, &self.queue, cache);
        state.plain_data_buffer = plain_data_buffer;

        let mut i = 0;
        while i < commands.len() {
            match &commands[i] {
                // Transfer commands (executed directly on encoder)
                Command::CopyBufferToBuffer { src, dst, size } => {
                    let src_buf = &hub.buffers.get(src.key).expect("Invalid src buffer").gpu;
                    let dst_buf = &hub.buffers.get(dst.key).expect("Invalid dst buffer").gpu;
                    encoder.copy_buffer_to_buffer(src_buf, src.offset, dst_buf, dst.offset, *size);
                }

                Command::CopyBufferToTexture { src, bytes_per_row, dst, size } => {
                    let src_buf = &hub.buffers.get(src.key).expect("Invalid src buffer").gpu;
                    let dst_tex = &hub.textures.get(dst.key).expect("Invalid dst texture").gpu;
                    // WebGPU requires bytes_per_row to be multiple of 256 for multi-row copies
                    // For single-row copies, we can omit it or use the actual value
                    let aligned_bpr = if size.height <= 1 && size.depth <= 1 {
                        // Single row - can use None
                        None
                    } else {
                        // Must be 256-aligned for multi-row copies
                        Some((*bytes_per_row + 255) & !255)
                    };
                    encoder.copy_buffer_to_texture(
                        wgpu::TexelCopyBufferInfo {
                            buffer: src_buf,
                            layout: wgpu::TexelCopyBufferLayout {
                                offset: src.offset,
                                bytes_per_row: aligned_bpr,
                                rows_per_image: None,
                            },
                        },
                        wgpu::TexelCopyTextureInfo {
                            texture: dst_tex,
                            mip_level: dst.mip_level,
                            origin: wgpu::Origin3d {
                                x: dst.origin[0],
                                y: dst.origin[1],
                                z: dst.origin[2],
                            },
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::Extent3d {
                            width: size.width,
                            height: size.height,
                            depth_or_array_layers: size.depth,
                        },
                    );
                }

                Command::CopyTextureToBuffer { src, dst, bytes_per_row, size } => {
                    let src_tex = &hub.textures.get(src.key).expect("Invalid src texture").gpu;
                    let dst_buf = &hub.buffers.get(dst.key).expect("Invalid dst buffer").gpu;
                    // WebGPU requires bytes_per_row to be multiple of 256 for multi-row copies
                    let aligned_bpr = if size.height <= 1 && size.depth <= 1 {
                        None
                    } else {
                        Some((*bytes_per_row + 255) & !255)
                    };
                    encoder.copy_texture_to_buffer(
                        wgpu::TexelCopyTextureInfo {
                            texture: src_tex,
                            mip_level: src.mip_level,
                            origin: wgpu::Origin3d {
                                x: src.origin[0],
                                y: src.origin[1],
                                z: src.origin[2],
                            },
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::TexelCopyBufferInfo {
                            buffer: dst_buf,
                            layout: wgpu::TexelCopyBufferLayout {
                                offset: dst.offset,
                                bytes_per_row: aligned_bpr,
                                rows_per_image: None,
                            },
                        },
                        wgpu::Extent3d {
                            width: size.width,
                            height: size.height,
                            depth_or_array_layers: size.depth,
                        },
                    );
                }

                Command::CopyTextureToTexture { src, dst, size } => {
                    let src_tex = &hub.textures.get(src.key).expect("Invalid src texture").gpu;
                    let dst_tex = &hub.textures.get(dst.key).expect("Invalid dst texture").gpu;
                    encoder.copy_texture_to_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: src_tex,
                            mip_level: src.mip_level,
                            origin: wgpu::Origin3d {
                                x: src.origin[0],
                                y: src.origin[1],
                                z: src.origin[2],
                            },
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::TexelCopyTextureInfo {
                            texture: dst_tex,
                            mip_level: dst.mip_level,
                            origin: wgpu::Origin3d {
                                x: dst.origin[0],
                                y: dst.origin[1],
                                z: dst.origin[2],
                            },
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::Extent3d {
                            width: size.width,
                            height: size.height,
                            depth_or_array_layers: size.depth,
                        },
                    );
                }

                Command::FillBuffer { dst, size, value } => {
                    let dst_buf = &hub.buffers.get(dst.key).expect("Invalid dst buffer").gpu;
                    if *value == 0 {
                        encoder.clear_buffer(dst_buf, dst.offset, Some(*size));
                    } else {
                        let fill_data = vec![*value; *size as usize];
                        self.queue.write_buffer(dst_buf, dst.offset, &fill_data);
                    }
                }

                Command::InitTexture { key: _ } => {
                    // WebGPU textures are zeroed on creation
                }

                // Render pass - find the matching EndRenderPass and execute as a block
                Command::BeginRenderPass { color_attachments, depth_attachment } => {
                    // Find EndRenderPass
                    let end_idx = commands[i..]
                        .iter()
                        .position(|c| matches!(c, Command::EndRenderPass))
                        .map(|p| i + p)
                        .unwrap_or(commands.len());

                    // Execute render pass
                    self.execute_render_pass(
                        hub,
                        encoder,
                        &commands[i + 1..end_idx],
                        color_attachments,
                        depth_attachment.as_ref(),
                        &mut state,
                    );

                    // Skip to after EndRenderPass
                    i = end_idx;
                }

                Command::EndRenderPass => {
                    // Handled by BeginRenderPass block
                }

                // Compute pass - find the matching EndComputePass and execute as a block
                Command::BeginComputePass => {
                    // Find EndComputePass
                    let end_idx = commands[i..]
                        .iter()
                        .position(|c| matches!(c, Command::EndComputePass))
                        .map(|p| i + p)
                        .unwrap_or(commands.len());

                    // Execute compute pass
                    self.execute_compute_pass(
                        hub,
                        encoder,
                        &commands[i + 1..end_idx],
                        &mut state,
                    );

                    // Skip to after EndComputePass
                    i = end_idx;
                }

                Command::EndComputePass => {
                    // Handled by BeginComputePass block
                }

                // These should be inside passes - skip if encountered at top level
                Command::SetRenderPipeline { .. } |
                Command::SetViewport { .. } |
                Command::SetScissor { .. } |
                Command::SetStencilReference { .. } |
                Command::SetVertexBuffer { .. } |
                Command::SetBindGroup { .. } |
                Command::Draw { .. } |
                Command::DrawIndexed { .. } |
                Command::DrawIndirect { .. } |
                Command::DrawIndexedIndirect { .. } |
                Command::SetComputePipeline { .. } |
                Command::Dispatch { .. } |
                Command::DispatchIndirect { .. } |
                Command::RecordBindGroup { .. } => {
                    // These are handled inside pass execution
                }
            }
            i += 1;
        }
    }

    /// Execute a render pass
    fn execute_render_pass(
        &self,
        hub: &Hub,
        encoder: &mut wgpu::CommandEncoder,
        commands: &[Command],
        color_attachments: &[RenderColorAttachment],
        depth_attachment: Option<&RenderDepthAttachment>,
        state: &mut ExecutionState,
    ) {
        // Build color attachments for wgpu
        // Note: We need to hold references to resolve target views, so collect them first
        let resolve_views: Vec<Option<&wgpu::TextureView>> = color_attachments
            .iter()
            .map(|ca| ca.resolve_target.and_then(|key| hub.texture_views.get(key)))
            .collect();

        let wgpu_color_attachments: Vec<Option<wgpu::RenderPassColorAttachment>> = color_attachments
            .iter()
            .zip(resolve_views.iter())
            .map(|(ca, resolve_view)| {
                // Try to get view from frame_view first, then from hub
                let view: &wgpu::TextureView = if let Some(ref frame_view) = ca.frame_view {
                    frame_view.as_ref()
                } else {
                    hub.texture_views.get(ca.view_key)?
                };

                Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: *resolve_view,
                    ops: wgpu::Operations {
                        load: map_load_op(&ca.load_op),
                        store: map_store_op(&ca.store_op),
                    },
                    depth_slice: None, // wgpu v28 requires this field
                })
            })
            .collect();

        // Build depth stencil attachment
        let wgpu_depth_attachment = depth_attachment.and_then(|da| {
            let view = hub.texture_views.get(da.view_key)?;
            Some(wgpu::RenderPassDepthStencilAttachment {
                view,
                depth_ops: Some(wgpu::Operations {
                    load: map_depth_load_op(&da.depth_load_op),
                    store: map_store_op(&da.depth_store_op),
                }),
                stencil_ops: Some(wgpu::Operations {
                    load: map_stencil_load_op(&da.stencil_load_op),
                    store: map_store_op(&da.stencil_store_op),
                }),
            })
        });

        // Create render pass
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &wgpu_color_attachments,
            depth_stencil_attachment: wgpu_depth_attachment,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None, // wgpu v28 requires this field
        });

        // Clear pending bind groups for this pass
        state.pending_bind_groups.clear();
        state.render_pipeline_key = None;

        // Execute commands
        for cmd in commands {
            match cmd {
                Command::SetRenderPipeline { key } => {
                    state.render_pipeline_key = Some(*key);
                    if let Some(pipeline) = hub.render_pipelines.get(*key) {
                        render_pass.set_pipeline(&pipeline.raw);
                    }
                }

                Command::SetViewport { viewport } => {
                    render_pass.set_viewport(
                        viewport.x,
                        viewport.y,
                        viewport.w,
                        viewport.h,
                        viewport.depth.start,
                        viewport.depth.end,
                    );
                }

                Command::SetScissor { rect } => {
                    // ScissorRect has i32 for x/y, wgpu wants u32
                    render_pass.set_scissor_rect(
                        rect.x.max(0) as u32,
                        rect.y.max(0) as u32,
                        rect.w,
                        rect.h,
                    );
                }

                Command::SetStencilReference { reference } => {
                    render_pass.set_stencil_reference(*reference);
                }

                Command::SetVertexBuffer { slot, buffer } => {
                    if let Some(buf_entry) = hub.buffers.get(buffer.key) {
                        render_pass.set_vertex_buffer(
                            *slot,
                            buf_entry.gpu.slice(buffer.offset..),
                        );
                    }
                }

                Command::RecordBindGroup { group_index, entries } => {
                    // Just accumulate entries - bind group created at draw time
                    let group_entries = state.pending_bind_groups
                        .entry(*group_index)
                        .or_insert_with(Vec::new);
                    group_entries.extend(entries.iter().cloned());
                }

                Command::Draw {
                    first_vertex,
                    vertex_count,
                    first_instance,
                    instance_count,
                } => {
                    // Flush pending bind groups before draw
                    state.flush_render_bind_groups(&mut render_pass);
                    render_pass.draw(
                        *first_vertex..*first_vertex + *vertex_count,
                        *first_instance..*first_instance + *instance_count,
                    );
                }

                Command::DrawIndexed {
                    index_buffer,
                    index_format,
                    index_count,
                    base_vertex,
                    first_instance,
                    instance_count,
                } => {
                    state.flush_render_bind_groups(&mut render_pass);
                    if let Some(buf_entry) = hub.buffers.get(index_buffer.key) {
                        render_pass.set_index_buffer(
                            buf_entry.gpu.slice(index_buffer.offset..),
                            map_index_format(*index_format),
                        );
                        render_pass.draw_indexed(
                            0..*index_count,
                            *base_vertex,
                            *first_instance..*first_instance + *instance_count,
                        );
                    }
                }

                Command::DrawIndirect { indirect_buffer } => {
                    state.flush_render_bind_groups(&mut render_pass);
                    if let Some(buf_entry) = hub.buffers.get(indirect_buffer.key) {
                        render_pass.draw_indirect(&buf_entry.gpu, indirect_buffer.offset);
                    }
                }

                Command::DrawIndexedIndirect {
                    index_buffer,
                    index_format,
                    indirect_buffer,
                } => {
                    state.flush_render_bind_groups(&mut render_pass);
                    if let Some(idx_entry) = hub.buffers.get(index_buffer.key) {
                        if let Some(ind_entry) = hub.buffers.get(indirect_buffer.key) {
                            render_pass.set_index_buffer(
                                idx_entry.gpu.slice(index_buffer.offset..),
                                map_index_format(*index_format),
                            );
                            render_pass.draw_indexed_indirect(&ind_entry.gpu, indirect_buffer.offset);
                        }
                    }
                }

                _ => {} // Ignore non-render commands
            }
        }
    }

    /// Execute a compute pass
    fn execute_compute_pass(
        &self,
        hub: &Hub,
        encoder: &mut wgpu::CommandEncoder,
        commands: &[Command],
        state: &mut ExecutionState,
    ) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });

        // Clear pending bind groups for this pass
        state.pending_bind_groups.clear();
        state.compute_pipeline_key = None;

        // Execute commands
        for cmd in commands {
            match cmd {
                Command::SetComputePipeline { key } => {
                    state.compute_pipeline_key = Some(*key);
                    if let Some(pipeline) = hub.compute_pipelines.get(*key) {
                        compute_pass.set_pipeline(&pipeline.raw);
                    }
                }

                Command::RecordBindGroup { group_index, entries } => {
                    // Just accumulate entries - bind group created at dispatch time
                    let group_entries = state.pending_bind_groups
                        .entry(*group_index)
                        .or_insert_with(Vec::new);
                    group_entries.extend(entries.iter().cloned());
                }

                Command::Dispatch { groups } => {
                    state.flush_compute_bind_groups(&mut compute_pass);
                    compute_pass.dispatch_workgroups(groups[0], groups[1], groups[2]);
                }

                Command::DispatchIndirect { indirect_buffer } => {
                    state.flush_compute_bind_groups(&mut compute_pass);
                    if let Some(buf_entry) = hub.buffers.get(indirect_buffer.key) {
                        compute_pass.dispatch_workgroups_indirect(
                            &buf_entry.gpu,
                            indirect_buffer.offset,
                        );
                    }
                }

                _ => {} // Ignore non-compute commands
            }
        }
    }
}
