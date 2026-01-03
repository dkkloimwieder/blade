//! Command encoding for WebGPU backend
//!
//! Implements deferred command recording pattern following GLES backend.

use super::*;

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
#[derive(Debug, Clone)]
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
            .map(|ct| RenderColorAttachment {
                view_key: ct.view.raw,
                load_op: ct.init_op,
                store_op: ct.finish_op,
                frame_view: None,
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
