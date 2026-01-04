//! Pipeline creation for WebGPU backend

use super::*;

/// Create a pipeline with error scope for validation.
/// On native, blocks to check errors synchronously.
/// On WASM, spawns a future to log errors asynchronously.
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

/// On WASM, skip error scopes entirely to avoid ordering issues.
/// wgpu requires error scopes to be popped in reverse order, but async futures
/// don't guarantee execution order. Use browser DevTools for error debugging.
#[cfg(target_arch = "wasm32")]
fn with_error_scope<T, F: FnOnce() -> T>(_device: &wgpu::Device, _name: &str, f: F) -> T {
    f()
}

/// Compiled shader ready for pipeline creation
struct CompiledShader {
    module: wgpu::ShaderModule,
    entry_point: String,
    attribute_mappings: Vec<crate::VertexAttributeMapping>,
    wg_size: [u32; 3],
}

/// Map Blade ShaderBinding to wgpu BindingType
fn map_binding_type(
    binding: crate::ShaderBinding,
    access: crate::StorageAccess,
) -> wgpu::BindingType {
    match binding {
        crate::ShaderBinding::Texture => {
            if access.is_empty() {
                wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                }
            } else {
                wgpu::BindingType::StorageTexture {
                    access: if access.contains(crate::StorageAccess::LOAD | crate::StorageAccess::STORE) {
                        wgpu::StorageTextureAccess::ReadWrite
                    } else if access.contains(crate::StorageAccess::STORE) {
                        wgpu::StorageTextureAccess::WriteOnly
                    } else {
                        wgpu::StorageTextureAccess::ReadOnly
                    },
                    format: wgpu::TextureFormat::Rgba8Unorm, // will be overridden at bind time
                    view_dimension: wgpu::TextureViewDimension::D2,
                }
            }
        }
        crate::ShaderBinding::TextureArray { count: _ } => {
            // WebGPU has limited support for texture arrays in base spec
            wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            }
        }
        crate::ShaderBinding::Sampler => wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        crate::ShaderBinding::Buffer => {
            if access.is_empty() {
                wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                }
            } else {
                wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage {
                        read_only: !access.contains(crate::StorageAccess::STORE),
                    },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                }
            }
        }
        crate::ShaderBinding::BufferArray { count: _ } => {
            // WebGPU has limited support for buffer arrays in base spec
            wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            }
        }
        crate::ShaderBinding::AccelerationStructure => {
            panic!("AccelerationStructure not supported in WebGPU backend")
        }
        crate::ShaderBinding::Plain { size: _ } => wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
    }
}

/// Map Blade ShaderVisibility to wgpu ShaderStages
fn map_shader_visibility(visibility: crate::ShaderVisibility) -> wgpu::ShaderStages {
    let mut stages = wgpu::ShaderStages::empty();
    if visibility.contains(crate::ShaderVisibility::VERTEX) {
        stages |= wgpu::ShaderStages::VERTEX;
    }
    if visibility.contains(crate::ShaderVisibility::FRAGMENT) {
        stages |= wgpu::ShaderStages::FRAGMENT;
    }
    if visibility.contains(crate::ShaderVisibility::COMPUTE) {
        stages |= wgpu::ShaderStages::COMPUTE;
    }
    stages
}

/// Map Blade PrimitiveTopology to wgpu
fn map_primitive_topology(topology: crate::PrimitiveTopology) -> wgpu::PrimitiveTopology {
    match topology {
        crate::PrimitiveTopology::PointList => wgpu::PrimitiveTopology::PointList,
        crate::PrimitiveTopology::LineList => wgpu::PrimitiveTopology::LineList,
        crate::PrimitiveTopology::LineStrip => wgpu::PrimitiveTopology::LineStrip,
        crate::PrimitiveTopology::TriangleList => wgpu::PrimitiveTopology::TriangleList,
        crate::PrimitiveTopology::TriangleStrip => wgpu::PrimitiveTopology::TriangleStrip,
    }
}

/// Map Blade FrontFace to wgpu
fn map_front_face(front_face: crate::FrontFace) -> wgpu::FrontFace {
    match front_face {
        crate::FrontFace::Ccw => wgpu::FrontFace::Ccw,
        crate::FrontFace::Cw => wgpu::FrontFace::Cw,
    }
}

/// Map Blade Face to wgpu
fn map_face(face: crate::Face) -> wgpu::Face {
    match face {
        crate::Face::Front => wgpu::Face::Front,
        crate::Face::Back => wgpu::Face::Back,
    }
}

/// Map Blade CompareFunction to wgpu
fn map_compare_function(compare: crate::CompareFunction) -> wgpu::CompareFunction {
    match compare {
        crate::CompareFunction::Never => wgpu::CompareFunction::Never,
        crate::CompareFunction::Less => wgpu::CompareFunction::Less,
        crate::CompareFunction::Equal => wgpu::CompareFunction::Equal,
        crate::CompareFunction::LessEqual => wgpu::CompareFunction::LessEqual,
        crate::CompareFunction::Greater => wgpu::CompareFunction::Greater,
        crate::CompareFunction::NotEqual => wgpu::CompareFunction::NotEqual,
        crate::CompareFunction::GreaterEqual => wgpu::CompareFunction::GreaterEqual,
        crate::CompareFunction::Always => wgpu::CompareFunction::Always,
    }
}

/// Map Blade StencilOperation to wgpu
fn map_stencil_operation(op: crate::StencilOperation) -> wgpu::StencilOperation {
    match op {
        crate::StencilOperation::Keep => wgpu::StencilOperation::Keep,
        crate::StencilOperation::Zero => wgpu::StencilOperation::Zero,
        crate::StencilOperation::Replace => wgpu::StencilOperation::Replace,
        crate::StencilOperation::Invert => wgpu::StencilOperation::Invert,
        crate::StencilOperation::IncrementClamp => wgpu::StencilOperation::IncrementClamp,
        crate::StencilOperation::DecrementClamp => wgpu::StencilOperation::DecrementClamp,
        crate::StencilOperation::IncrementWrap => wgpu::StencilOperation::IncrementWrap,
        crate::StencilOperation::DecrementWrap => wgpu::StencilOperation::DecrementWrap,
    }
}

/// Map Blade StencilFaceState to wgpu
fn map_stencil_face_state(state: &crate::StencilFaceState) -> wgpu::StencilFaceState {
    wgpu::StencilFaceState {
        compare: map_compare_function(state.compare),
        fail_op: map_stencil_operation(state.fail_op),
        depth_fail_op: map_stencil_operation(state.depth_fail_op),
        pass_op: map_stencil_operation(state.pass_op),
    }
}

/// Map Blade BlendFactor to wgpu
fn map_blend_factor(factor: crate::BlendFactor) -> wgpu::BlendFactor {
    match factor {
        crate::BlendFactor::Zero => wgpu::BlendFactor::Zero,
        crate::BlendFactor::One => wgpu::BlendFactor::One,
        crate::BlendFactor::Src => wgpu::BlendFactor::Src,
        crate::BlendFactor::OneMinusSrc => wgpu::BlendFactor::OneMinusSrc,
        crate::BlendFactor::SrcAlpha => wgpu::BlendFactor::SrcAlpha,
        crate::BlendFactor::OneMinusSrcAlpha => wgpu::BlendFactor::OneMinusSrcAlpha,
        crate::BlendFactor::Dst => wgpu::BlendFactor::Dst,
        crate::BlendFactor::OneMinusDst => wgpu::BlendFactor::OneMinusDst,
        crate::BlendFactor::DstAlpha => wgpu::BlendFactor::DstAlpha,
        crate::BlendFactor::OneMinusDstAlpha => wgpu::BlendFactor::OneMinusDstAlpha,
        crate::BlendFactor::SrcAlphaSaturated => wgpu::BlendFactor::SrcAlphaSaturated,
        crate::BlendFactor::Constant => wgpu::BlendFactor::Constant,
        crate::BlendFactor::OneMinusConstant => wgpu::BlendFactor::OneMinusConstant,
        // Dual-source blending is not supported in WebGPU base spec
        crate::BlendFactor::Src1
        | crate::BlendFactor::OneMinusSrc1
        | crate::BlendFactor::Src1Alpha
        | crate::BlendFactor::OneMinusSrc1Alpha => {
            panic!("Dual-source blending (Src1 variants) not supported in WebGPU")
        }
    }
}

/// Map Blade BlendOperation to wgpu
fn map_blend_operation(op: crate::BlendOperation) -> wgpu::BlendOperation {
    match op {
        crate::BlendOperation::Add => wgpu::BlendOperation::Add,
        crate::BlendOperation::Subtract => wgpu::BlendOperation::Subtract,
        crate::BlendOperation::ReverseSubtract => wgpu::BlendOperation::ReverseSubtract,
        crate::BlendOperation::Min => wgpu::BlendOperation::Min,
        crate::BlendOperation::Max => wgpu::BlendOperation::Max,
    }
}

/// Map Blade BlendComponent to wgpu
fn map_blend_component(component: &crate::BlendComponent) -> wgpu::BlendComponent {
    wgpu::BlendComponent {
        src_factor: map_blend_factor(component.src_factor),
        dst_factor: map_blend_factor(component.dst_factor),
        operation: map_blend_operation(component.operation),
    }
}

/// Map Blade VertexFormat to wgpu
fn map_vertex_format(format: crate::VertexFormat) -> wgpu::VertexFormat {
    match format {
        crate::VertexFormat::F32 => wgpu::VertexFormat::Float32,
        crate::VertexFormat::F32Vec2 => wgpu::VertexFormat::Float32x2,
        crate::VertexFormat::F32Vec3 => wgpu::VertexFormat::Float32x3,
        crate::VertexFormat::F32Vec4 => wgpu::VertexFormat::Float32x4,
        crate::VertexFormat::U32 => wgpu::VertexFormat::Uint32,
        crate::VertexFormat::U32Vec2 => wgpu::VertexFormat::Uint32x2,
        crate::VertexFormat::U32Vec3 => wgpu::VertexFormat::Uint32x3,
        crate::VertexFormat::U32Vec4 => wgpu::VertexFormat::Uint32x4,
        crate::VertexFormat::I32 => wgpu::VertexFormat::Sint32,
        crate::VertexFormat::I32Vec2 => wgpu::VertexFormat::Sint32x2,
        crate::VertexFormat::I32Vec3 => wgpu::VertexFormat::Sint32x3,
        crate::VertexFormat::I32Vec4 => wgpu::VertexFormat::Sint32x4,
    }
}

impl Context {
    /// Create a bind group layout from a ShaderDataLayout
    fn create_bind_group_layout(
        &self,
        layout: &crate::ShaderDataLayout,
        info: &crate::ShaderDataInfo,
    ) -> wgpu::BindGroupLayout {
        if info.visibility.is_empty() {
            // Create empty bind group layout
            return self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[],
            });
        }

        let visibility = map_shader_visibility(info.visibility);
        let entries: Vec<wgpu::BindGroupLayoutEntry> = layout
            .bindings
            .iter()
            .zip(info.binding_access.iter())
            .enumerate()
            .map(|(binding_index, (&(_, binding), &access))| {
                wgpu::BindGroupLayoutEntry {
                    binding: binding_index as u32,
                    visibility,
                    ty: map_binding_type(binding, access),
                    count: match binding {
                        crate::ShaderBinding::TextureArray { count } |
                        crate::ShaderBinding::BufferArray { count } => {
                            std::num::NonZeroU32::new(count)
                        }
                        _ => None,
                    },
                }
            })
            .collect();

        self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &entries,
        })
    }

    /// Load a shader and create wgpu module
    fn load_shader(
        &self,
        sf: crate::ShaderFunction,
        group_layouts: &[&crate::ShaderDataLayout],
        group_infos: &mut [crate::ShaderDataInfo],
        vertex_fetch_states: &[crate::VertexFetchState],
    ) -> CompiledShader {
        let ep_index = sf.entry_point_index();

        let (mut module, module_info) = sf.shader.resolve_constants(&sf.constants);
        let wg_size = module.entry_points[ep_index].workgroup_size;

        // Collect entry point stages before mutable borrow
        let ep_stages: Vec<naga::ShaderStage> = module
            .entry_points
            .iter()
            .map(|ep| ep.stage)
            .collect();

        // Process ALL entry points to ensure all resource variables have bindings
        // (wgpu compiles the entire module, not just one entry point)
        for (ep_idx, &stage) in ep_stages.iter().enumerate() {
            let ep_info = module_info.get_entry_point(ep_idx);
            crate::Shader::fill_resource_bindings(
                &mut module,
                group_infos,
                stage,
                ep_info,
                group_layouts,
            );
        }

        let attribute_mappings =
            crate::Shader::fill_vertex_locations(&mut module, ep_index, vertex_fetch_states);

        // Emit the modified module back to WGSL with @group/@binding annotations
        let wgsl_source = naga::back::wgsl::write_string(
            &module,
            &module_info,
            naga::back::wgsl::WriterFlags::empty(),
        )
        .expect("Failed to emit WGSL from modified naga module");

        let wgpu_module = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(sf.entry_point),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Owned(wgsl_source)),
        });

        CompiledShader {
            module: wgpu_module,
            entry_point: sf.entry_point.to_string(),
            attribute_mappings,
            wg_size,
        }
    }
}

#[hidden_trait::expose]
impl crate::traits::ShaderDevice for Context {
    type ComputePipeline = ComputePipeline;
    type RenderPipeline = RenderPipeline;

    fn create_compute_pipeline(&self, desc: crate::ComputePipelineDesc) -> ComputePipeline {
        let mut group_infos: Vec<crate::ShaderDataInfo> = desc
            .data_layouts
            .iter()
            .map(|layout| layout.to_info())
            .collect();

        // Create group mappings
        let group_mappings: Box<[ShaderDataMapping]> = desc
            .data_layouts
            .iter()
            .enumerate()
            .map(|(group_index, layout)| ShaderDataMapping {
                targets: layout
                    .bindings
                    .iter()
                    .enumerate()
                    .map(|(binding_index, _)| {
                        vec![BindingSlot {
                            group: group_index as u32,
                            binding: binding_index as u32,
                        }]
                    })
                    .collect(),
            })
            .collect();

        // Load compute shader
        let shader = self.load_shader(desc.compute, desc.data_layouts, &mut group_infos, &[]);

        // Create bind group layouts
        let bind_group_layouts: Vec<wgpu::BindGroupLayout> = desc
            .data_layouts
            .iter()
            .zip(group_infos.iter())
            .map(|(layout, info)| self.create_bind_group_layout(layout, info))
            .collect();

        let bind_group_layout_refs: Vec<&wgpu::BindGroupLayout> =
            bind_group_layouts.iter().collect();

        // Create pipeline layout
        let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(desc.name),
            bind_group_layouts: &bind_group_layout_refs,
            immediate_size: 0, // wgpu v28: no push constants for WebGPU
        });

        // Create compute pipeline with error scope for validation
        let raw = with_error_scope(&self.device, desc.name, || {
            self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(desc.name),
                layout: Some(&pipeline_layout),
                module: &shader.module,
                entry_point: Some(&shader.entry_point),
                compilation_options: Default::default(),
                cache: None,
            })
        });

        // Store in hub
        let mut hub = self.hub.write().unwrap();
        let key = hub.compute_pipelines.insert(ComputePipelineEntry {
            raw,
            group_mappings: group_mappings.clone(),
            bind_group_layouts,
            wg_size: shader.wg_size,
        });

        ComputePipeline {
            raw: key,
            wg_size: shader.wg_size,
            group_mappings,
        }
    }

    fn destroy_compute_pipeline(&self, pipeline: &mut ComputePipeline) {
        let mut hub = self.hub.write().unwrap();
        hub.compute_pipelines.remove(pipeline.raw);
    }

    fn create_render_pipeline(&self, desc: crate::RenderPipelineDesc) -> RenderPipeline {
        use super::resource::map_texture_format;

        let mut group_infos: Vec<crate::ShaderDataInfo> = desc
            .data_layouts
            .iter()
            .map(|layout| layout.to_info())
            .collect();

        // Create group mappings
        let group_mappings: Box<[ShaderDataMapping]> = desc
            .data_layouts
            .iter()
            .enumerate()
            .map(|(group_index, layout)| ShaderDataMapping {
                targets: layout
                    .bindings
                    .iter()
                    .enumerate()
                    .map(|(binding_index, _)| {
                        vec![BindingSlot {
                            group: group_index as u32,
                            binding: binding_index as u32,
                        }]
                    })
                    .collect(),
            })
            .collect();

        // Load vertex shader
        let vertex_shader = self.load_shader(
            desc.vertex,
            desc.data_layouts,
            &mut group_infos,
            desc.vertex_fetches,
        );

        // Load fragment shader if present
        let fragment_shader = desc.fragment.map(|sf| {
            self.load_shader(sf, desc.data_layouts, &mut group_infos, &[])
        });

        // Create bind group layouts
        let bind_group_layouts: Vec<wgpu::BindGroupLayout> = desc
            .data_layouts
            .iter()
            .zip(group_infos.iter())
            .map(|(layout, info)| self.create_bind_group_layout(layout, info))
            .collect();

        let bind_group_layout_refs: Vec<&wgpu::BindGroupLayout> =
            bind_group_layouts.iter().collect();

        // Create pipeline layout
        let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(desc.name),
            bind_group_layouts: &bind_group_layout_refs,
            immediate_size: 0, // wgpu v28: no push constants for WebGPU
        });

        // Build vertex buffer layouts
        let mut vertex_buffer_layouts: Vec<wgpu::VertexBufferLayout> = Vec::new();
        let mut vertex_attributes: Vec<Vec<wgpu::VertexAttribute>> = Vec::new();

        for (buffer_index, fetch) in desc.vertex_fetches.iter().enumerate() {
            let mut attributes = Vec::new();
            for mapping in &vertex_shader.attribute_mappings {
                if mapping.buffer_index == buffer_index {
                    let (_, attrib) = fetch.layout.attributes[mapping.attribute_index];
                    attributes.push(wgpu::VertexAttribute {
                        format: map_vertex_format(attrib.format),
                        offset: attrib.offset as u64,
                        shader_location: attributes.len() as u32,
                    });
                }
            }
            vertex_attributes.push(attributes);
        }

        for (buffer_index, fetch) in desc.vertex_fetches.iter().enumerate() {
            vertex_buffer_layouts.push(wgpu::VertexBufferLayout {
                array_stride: fetch.layout.stride as u64,
                step_mode: if fetch.instanced {
                    wgpu::VertexStepMode::Instance
                } else {
                    wgpu::VertexStepMode::Vertex
                },
                attributes: &vertex_attributes[buffer_index],
            });
        }

        // Build color targets
        let color_targets: Vec<Option<wgpu::ColorTargetState>> = desc
            .color_targets
            .iter()
            .map(|target| {
                Some(wgpu::ColorTargetState {
                    format: map_texture_format(target.format),
                    blend: target.blend.as_ref().map(|b| wgpu::BlendState {
                        color: map_blend_component(&b.color),
                        alpha: map_blend_component(&b.alpha),
                    }),
                    write_mask: wgpu::ColorWrites::from_bits_truncate(target.write_mask.bits()),
                })
            })
            .collect();

        // Build depth stencil state
        let depth_stencil = desc.depth_stencil.as_ref().map(|ds| wgpu::DepthStencilState {
            format: map_texture_format(ds.format),
            depth_write_enabled: ds.depth_write_enabled,
            depth_compare: map_compare_function(ds.depth_compare),
            stencil: wgpu::StencilState {
                front: map_stencil_face_state(&ds.stencil.front),
                back: map_stencil_face_state(&ds.stencil.back),
                read_mask: ds.stencil.read_mask,
                write_mask: ds.stencil.write_mask,
            },
            bias: wgpu::DepthBiasState {
                constant: ds.bias.constant,
                slope_scale: ds.bias.slope_scale,
                clamp: ds.bias.clamp,
            },
        });

        // Create render pipeline with error scope for validation
        let raw = with_error_scope(&self.device, desc.name, || {
            self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(desc.name),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &vertex_shader.module,
                    entry_point: Some(&vertex_shader.entry_point),
                    compilation_options: Default::default(),
                    buffers: &vertex_buffer_layouts,
                },
                primitive: wgpu::PrimitiveState {
                    topology: map_primitive_topology(desc.primitive.topology),
                    strip_index_format: None,
                    front_face: map_front_face(desc.primitive.front_face),
                    cull_mode: desc.primitive.cull_mode.map(map_face),
                    unclipped_depth: desc.primitive.unclipped_depth,
                    polygon_mode: if desc.primitive.wireframe {
                        wgpu::PolygonMode::Line
                    } else {
                        wgpu::PolygonMode::Fill
                    },
                    conservative: false,
                },
                depth_stencil,
                multisample: wgpu::MultisampleState {
                    count: desc.multisample_state.sample_count,
                    mask: desc.multisample_state.sample_mask,
                    alpha_to_coverage_enabled: desc.multisample_state.alpha_to_coverage,
                },
                fragment: fragment_shader.as_ref().map(|fs| wgpu::FragmentState {
                    module: &fs.module,
                    entry_point: Some(&fs.entry_point),
                    compilation_options: Default::default(),
                    targets: &color_targets,
                }),
                multiview_mask: None, // wgpu v28: multiview renamed
                cache: None,
            })
        });

        // Store in hub
        let mut hub = self.hub.write().unwrap();
        let key = hub.render_pipelines.insert(RenderPipelineEntry {
            raw,
            group_mappings: group_mappings.clone(),
            bind_group_layouts,
            topology: desc.primitive.topology,
        });

        RenderPipeline {
            raw: key,
            topology: desc.primitive.topology,
            group_mappings,
        }
    }

    fn destroy_render_pipeline(&self, pipeline: &mut RenderPipeline) {
        let mut hub = self.hub.write().unwrap();
        hub.render_pipelines.remove(pipeline.raw);
    }
}
