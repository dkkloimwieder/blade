//! Resource creation for WebGPU backend

use super::*;

//=============================================================================
// Texture Format Mapping
//=============================================================================

pub(super) fn map_texture_format(format: crate::TextureFormat) -> wgpu::TextureFormat {
    match format {
        crate::TextureFormat::R8Unorm => wgpu::TextureFormat::R8Unorm,
        crate::TextureFormat::Rg8Unorm => wgpu::TextureFormat::Rg8Unorm,
        crate::TextureFormat::Rg8Snorm => wgpu::TextureFormat::Rg8Snorm,
        crate::TextureFormat::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
        crate::TextureFormat::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
        crate::TextureFormat::Bgra8Unorm => wgpu::TextureFormat::Bgra8Unorm,
        crate::TextureFormat::Bgra8UnormSrgb => wgpu::TextureFormat::Bgra8UnormSrgb,
        crate::TextureFormat::Rgba8Snorm => wgpu::TextureFormat::Rgba8Snorm,
        crate::TextureFormat::R16Float => wgpu::TextureFormat::R16Float,
        crate::TextureFormat::Rg16Float => wgpu::TextureFormat::Rg16Float,
        crate::TextureFormat::Rgba16Float => wgpu::TextureFormat::Rgba16Float,
        crate::TextureFormat::R32Float => wgpu::TextureFormat::R32Float,
        crate::TextureFormat::Rg32Float => wgpu::TextureFormat::Rg32Float,
        crate::TextureFormat::Rgba32Float => wgpu::TextureFormat::Rgba32Float,
        crate::TextureFormat::R32Uint => wgpu::TextureFormat::R32Uint,
        crate::TextureFormat::Rg32Uint => wgpu::TextureFormat::Rg32Uint,
        crate::TextureFormat::Rgba32Uint => wgpu::TextureFormat::Rgba32Uint,
        crate::TextureFormat::Depth32Float => wgpu::TextureFormat::Depth32Float,
        crate::TextureFormat::Depth32FloatStencil8Uint => wgpu::TextureFormat::Depth32FloatStencil8,
        crate::TextureFormat::Stencil8Uint => wgpu::TextureFormat::Stencil8,
        crate::TextureFormat::Bc1Unorm => wgpu::TextureFormat::Bc1RgbaUnorm,
        crate::TextureFormat::Bc1UnormSrgb => wgpu::TextureFormat::Bc1RgbaUnormSrgb,
        crate::TextureFormat::Bc2Unorm => wgpu::TextureFormat::Bc2RgbaUnorm,
        crate::TextureFormat::Bc2UnormSrgb => wgpu::TextureFormat::Bc2RgbaUnormSrgb,
        crate::TextureFormat::Bc3Unorm => wgpu::TextureFormat::Bc3RgbaUnorm,
        crate::TextureFormat::Bc3UnormSrgb => wgpu::TextureFormat::Bc3RgbaUnormSrgb,
        crate::TextureFormat::Bc4Unorm => wgpu::TextureFormat::Bc4RUnorm,
        crate::TextureFormat::Bc4Snorm => wgpu::TextureFormat::Bc4RSnorm,
        crate::TextureFormat::Bc5Unorm => wgpu::TextureFormat::Bc5RgUnorm,
        crate::TextureFormat::Bc5Snorm => wgpu::TextureFormat::Bc5RgSnorm,
        crate::TextureFormat::Bc6hUfloat => wgpu::TextureFormat::Bc6hRgbUfloat,
        crate::TextureFormat::Bc6hFloat => wgpu::TextureFormat::Bc6hRgbFloat,
        crate::TextureFormat::Bc7Unorm => wgpu::TextureFormat::Bc7RgbaUnorm,
        crate::TextureFormat::Bc7UnormSrgb => wgpu::TextureFormat::Bc7RgbaUnormSrgb,
        crate::TextureFormat::Rgb10a2Unorm => wgpu::TextureFormat::Rgb10a2Unorm,
        crate::TextureFormat::Rg11b10Ufloat => wgpu::TextureFormat::Rg11b10Ufloat,
        crate::TextureFormat::Rgb9e5Ufloat => wgpu::TextureFormat::Rgb9e5Ufloat,
    }
}

fn map_texture_dimension(dim: crate::TextureDimension) -> wgpu::TextureDimension {
    match dim {
        crate::TextureDimension::D1 => wgpu::TextureDimension::D1,
        crate::TextureDimension::D2 => wgpu::TextureDimension::D2,
        crate::TextureDimension::D3 => wgpu::TextureDimension::D3,
    }
}

fn map_texture_view_dimension(dim: crate::ViewDimension) -> wgpu::TextureViewDimension {
    match dim {
        crate::ViewDimension::D1 => wgpu::TextureViewDimension::D1,
        crate::ViewDimension::D1Array => wgpu::TextureViewDimension::D1,  // wgpu doesn't have D1Array
        crate::ViewDimension::D2 => wgpu::TextureViewDimension::D2,
        crate::ViewDimension::D2Array => wgpu::TextureViewDimension::D2Array,
        crate::ViewDimension::Cube => wgpu::TextureViewDimension::Cube,
        crate::ViewDimension::CubeArray => wgpu::TextureViewDimension::CubeArray,
        crate::ViewDimension::D3 => wgpu::TextureViewDimension::D3,
    }
}

/// Determine texture aspect from format
fn get_aspect_from_format(format: crate::TextureFormat) -> wgpu::TextureAspect {
    match format {
        crate::TextureFormat::Depth32Float => wgpu::TextureAspect::DepthOnly,
        crate::TextureFormat::Stencil8Uint => wgpu::TextureAspect::StencilOnly,
        crate::TextureFormat::Depth32FloatStencil8Uint => wgpu::TextureAspect::All,
        _ => wgpu::TextureAspect::All,
    }
}

fn map_address_mode(mode: crate::AddressMode) -> wgpu::AddressMode {
    match mode {
        crate::AddressMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
        crate::AddressMode::Repeat => wgpu::AddressMode::Repeat,
        crate::AddressMode::MirrorRepeat => wgpu::AddressMode::MirrorRepeat,
        crate::AddressMode::ClampToBorder => wgpu::AddressMode::ClampToBorder,
    }
}

fn map_filter_mode(mode: crate::FilterMode) -> wgpu::FilterMode {
    match mode {
        crate::FilterMode::Nearest => wgpu::FilterMode::Nearest,
        crate::FilterMode::Linear => wgpu::FilterMode::Linear,
    }
}

fn map_mipmap_filter_mode(mode: crate::FilterMode) -> wgpu::MipmapFilterMode {
    match mode {
        crate::FilterMode::Nearest => wgpu::MipmapFilterMode::Nearest,
        crate::FilterMode::Linear => wgpu::MipmapFilterMode::Linear,
    }
}

pub(super) fn map_compare_function(func: crate::CompareFunction) -> wgpu::CompareFunction {
    match func {
        crate::CompareFunction::Never => wgpu::CompareFunction::Never,
        crate::CompareFunction::Less => wgpu::CompareFunction::Less,
        crate::CompareFunction::LessEqual => wgpu::CompareFunction::LessEqual,
        crate::CompareFunction::Equal => wgpu::CompareFunction::Equal,
        crate::CompareFunction::GreaterEqual => wgpu::CompareFunction::GreaterEqual,
        crate::CompareFunction::Greater => wgpu::CompareFunction::Greater,
        crate::CompareFunction::NotEqual => wgpu::CompareFunction::NotEqual,
        crate::CompareFunction::Always => wgpu::CompareFunction::Always,
    }
}

//=============================================================================
// Acceleration Structure Helpers (Unsupported in WebGPU)
//=============================================================================

impl Context {
    /// Returns an error because acceleration structures are not supported in WebGPU.
    ///
    /// Check `Context::capabilities().ray_query` before attempting to use ray tracing.
    pub fn get_bottom_level_acceleration_structure_sizes(
        &self,
        _meshes: &[crate::AccelerationStructureMesh],
    ) -> Result<crate::AccelerationStructureSizes, crate::UnsupportedFeatureError> {
        Err(crate::UnsupportedFeatureError {
            feature: "Acceleration structures",
            capability_hint: "ray_query",
        })
    }

    /// Returns an error because acceleration structures are not supported in WebGPU.
    ///
    /// Check `Context::capabilities().ray_query` before attempting to use ray tracing.
    pub fn get_top_level_acceleration_structure_sizes(
        &self,
        _instance_count: u32,
    ) -> Result<crate::AccelerationStructureSizes, crate::UnsupportedFeatureError> {
        Err(crate::UnsupportedFeatureError {
            feature: "Acceleration structures",
            capability_hint: "ray_query",
        })
    }

    /// Returns an error because acceleration structures are not supported in WebGPU.
    ///
    /// Check `Context::capabilities().ray_query` before attempting to use ray tracing.
    pub fn create_acceleration_structure_instance_buffer(
        &self,
        _instances: &[crate::AccelerationStructureInstance],
        _bottom_level: &[AccelerationStructure],
    ) -> Result<Buffer, crate::UnsupportedFeatureError> {
        Err(crate::UnsupportedFeatureError {
            feature: "Acceleration structures",
            capability_hint: "ray_query",
        })
    }
}

//=============================================================================
// ResourceDevice Implementation
//=============================================================================

#[hidden_trait::expose]
impl crate::traits::ResourceDevice for Context {
    type Buffer = Buffer;
    type Texture = Texture;
    type TextureView = TextureView;
    type Sampler = Sampler;
    type AccelerationStructure = AccelerationStructure;

    fn create_buffer(&self, desc: crate::BufferDesc) -> Buffer {
        // Determine buffer usage flags
        let mut usage = wgpu::BufferUsages::empty();

        // Common usages for most buffers
        usage |= wgpu::BufferUsages::COPY_SRC;
        usage |= wgpu::BufferUsages::COPY_DST;

        // Add usages based on memory type
        let (shadow, data_ptr) = match desc.memory {
            crate::Memory::Device => {
                // Device-local buffer: add storage and uniform usages
                usage |= wgpu::BufferUsages::STORAGE;
                usage |= wgpu::BufferUsages::UNIFORM;
                usage |= wgpu::BufferUsages::VERTEX;
                usage |= wgpu::BufferUsages::INDEX;
                usage |= wgpu::BufferUsages::INDIRECT;
                (None, std::ptr::null_mut())
            }
            crate::Memory::Upload | crate::Memory::Shared => {
                // Host-visible buffer: create shadow memory for CPU access
                usage |= wgpu::BufferUsages::STORAGE;
                usage |= wgpu::BufferUsages::UNIFORM;
                usage |= wgpu::BufferUsages::VERTEX;
                usage |= wgpu::BufferUsages::INDEX;

                let mut shadow_data = vec![0u8; desc.size as usize].into_boxed_slice();
                let ptr = shadow_data.as_mut_ptr();
                (Some(shadow_data), ptr)
            }
            crate::Memory::External(_) => {
                panic!(
                    "External memory is not supported in WebGPU. \
                     Use Memory::Device, Memory::Upload, or Memory::Shared instead."
                )
            }
        };

        // Create the GPU buffer
        let gpu = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: if desc.name.is_empty() {
                None
            } else {
                Some(desc.name)
            },
            size: desc.size,
            usage,
            mapped_at_creation: false,
        });

        // Store in hub
        let key = {
            let mut hub = self.hub.write().unwrap();
            hub.buffers.insert(BufferEntry {
                gpu,
                shadow,
                dirty_range: Mutex::new(None),
            })
        };

        Buffer {
            raw: key,
            size: desc.size,
            data: data_ptr,
        }
    }

    fn sync_buffer(&self, buffer: Buffer) {
        // Mark the entire buffer as dirty so it gets synced before next submit
        self.mark_buffer_dirty(buffer);
    }

    fn destroy_buffer(&self, buffer: Buffer) {
        // CRITICAL: Invalidate cached bind groups FIRST
        // This drops Arc references before we remove from Hub
        {
            let mut cache = self.bind_group_cache.write().unwrap();
            cache.invalidate_buffer(buffer.raw);
        }

        let mut hub = self.hub.write().unwrap();
        if let Some(entry) = hub.buffers.remove(buffer.raw) {
            // GPU buffer is dropped automatically
            // Shadow memory is dropped automatically
            drop(entry);
        }
    }

    fn create_texture(&self, desc: crate::TextureDesc) -> Texture {
        let format = map_texture_format(desc.format);
        let dimension = map_texture_dimension(desc.dimension);

        // Determine usage flags
        let mut usage = wgpu::TextureUsages::empty();
        usage |= wgpu::TextureUsages::COPY_SRC;
        usage |= wgpu::TextureUsages::COPY_DST;

        if desc.usage.contains(crate::TextureUsage::RESOURCE) {
            usage |= wgpu::TextureUsages::TEXTURE_BINDING;
        }
        if desc.usage.contains(crate::TextureUsage::STORAGE) {
            usage |= wgpu::TextureUsages::STORAGE_BINDING;
        }
        if desc.usage.contains(crate::TextureUsage::TARGET) {
            usage |= wgpu::TextureUsages::RENDER_ATTACHMENT;
        }

        let gpu = self.device.create_texture(&wgpu::TextureDescriptor {
            label: if desc.name.is_empty() {
                None
            } else {
                Some(desc.name)
            },
            size: wgpu::Extent3d {
                width: desc.size.width,
                height: desc.size.height,
                depth_or_array_layers: if desc.dimension == crate::TextureDimension::D3 {
                    desc.size.depth
                } else {
                    desc.array_layer_count
                },
            },
            mip_level_count: desc.mip_level_count,
            sample_count: desc.sample_count,
            dimension,
            format,
            usage,
            view_formats: &[],
        });

        let target_size = [
            desc.size.width.min(u16::MAX as u32) as u16,
            desc.size.height.min(u16::MAX as u32) as u16,
        ];

        let key = {
            let mut hub = self.hub.write().unwrap();
            hub.textures.insert(TextureEntry { gpu })
        };

        Texture {
            raw: key,
            format: desc.format,
            target_size,
        }
    }

    fn destroy_texture(&self, texture: Texture) {
        let mut hub = self.hub.write().unwrap();
        if let Some(entry) = hub.textures.remove(texture.raw) {
            drop(entry);
        }
    }

    fn create_texture_view(
        &self,
        texture: Texture,
        desc: crate::TextureViewDesc,
    ) -> TextureView {
        let hub = self.hub.read().unwrap();
        let texture_entry = hub.textures.get(texture.raw).expect("Invalid texture handle");

        let format = map_texture_format(desc.format);
        let dimension = map_texture_view_dimension(desc.dimension);

        // Determine texture aspect from the format
        let aspect = get_aspect_from_format(desc.format);
        let aspects = desc.format.aspects();

        let view = texture_entry.gpu.create_view(&wgpu::TextureViewDescriptor {
            label: if desc.name.is_empty() {
                None
            } else {
                Some(desc.name)
            },
            format: Some(format),
            dimension: Some(dimension),
            usage: None,  // wgpu v28 requires this field
            aspect,
            base_mip_level: desc.subresources.base_mip_level,
            mip_level_count: desc.subresources.mip_level_count.map(|n| n.get()),
            base_array_layer: desc.subresources.base_array_layer,
            array_layer_count: desc.subresources.array_layer_count.map(|n| n.get()),
        });

        drop(hub);

        let key = {
            let mut hub = self.hub.write().unwrap();
            hub.texture_views.insert(view)
        };

        TextureView {
            raw: key,
            target_size: texture.target_size,
            aspects,
        }
    }

    fn destroy_texture_view(&self, view: TextureView) {
        // CRITICAL: Invalidate cached bind groups FIRST
        {
            let mut cache = self.bind_group_cache.write().unwrap();
            cache.invalidate_texture_view(view.raw);
        }

        let mut hub = self.hub.write().unwrap();
        hub.texture_views.remove(view.raw);
    }

    fn create_sampler(&self, desc: crate::SamplerDesc) -> Sampler {
        let address_mode_u = map_address_mode(desc.address_modes[0]);
        let address_mode_v = map_address_mode(desc.address_modes[1]);
        let address_mode_w = map_address_mode(desc.address_modes[2]);
        let mag_filter = map_filter_mode(desc.mag_filter);
        let min_filter = map_filter_mode(desc.min_filter);
        let mipmap_filter = map_mipmap_filter_mode(desc.mipmap_filter);
        let compare = desc.compare.map(map_compare_function);

        let border_color = desc.border_color.map(|color| match color {
            crate::TextureColor::TransparentBlack => wgpu::SamplerBorderColor::TransparentBlack,
            crate::TextureColor::OpaqueBlack => wgpu::SamplerBorderColor::OpaqueBlack,
            crate::TextureColor::White => wgpu::SamplerBorderColor::OpaqueWhite,
        });

        let gpu = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: if desc.name.is_empty() {
                None
            } else {
                Some(desc.name)
            },
            address_mode_u,
            address_mode_v,
            address_mode_w,
            mag_filter,
            min_filter,
            mipmap_filter,
            lod_min_clamp: desc.lod_min_clamp,
            lod_max_clamp: desc.lod_max_clamp.unwrap_or(f32::MAX),
            compare,
            // WebGPU requires anisotropy >= 1; use max(1) to ensure valid value
            anisotropy_clamp: desc.anisotropy_clamp.max(1).min(u16::MAX as u32) as u16,
            border_color,
        });

        let key = {
            let mut hub = self.hub.write().unwrap();
            hub.samplers.insert(gpu)
        };

        Sampler { raw: key }
    }

    fn destroy_sampler(&self, sampler: Sampler) {
        // CRITICAL: Invalidate cached bind groups FIRST
        {
            let mut cache = self.bind_group_cache.write().unwrap();
            cache.invalidate_sampler(sampler.raw);
        }

        let mut hub = self.hub.write().unwrap();
        hub.samplers.remove(sampler.raw);
    }

    fn create_acceleration_structure(
        &self,
        _desc: crate::AccelerationStructureDesc,
    ) -> AccelerationStructure {
        panic!(
            "Acceleration structures are not supported in WebGPU. \
             Check `Context::capabilities().ray_query` before using ray tracing features."
        )
    }

    fn destroy_acceleration_structure(&self, _acceleration_structure: AccelerationStructure) {
        panic!(
            "Acceleration structures are not supported in WebGPU. \
             Check `Context::capabilities().ray_query` before using ray tracing features."
        )
    }
}
