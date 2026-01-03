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
    // TODO: Add command variants
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
// Timings
//=============================================================================

impl CommandEncoder {
    pub fn timings(&self) -> crate::Timings {
        // WebGPU doesn't have the same timing infrastructure
        // Return empty timings for now (Timings is just Vec<(String, Duration)>)
        Vec::new()
    }
}
