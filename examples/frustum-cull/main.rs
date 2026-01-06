//! GPU Frustum Culling Example
//!
//! Demonstrates GPU-driven visibility culling using compute shaders.
//! Objects outside the view frustum are culled on the GPU, and only
//! visible objects are drawn using indirect rendering.

#![allow(irrefutable_let_patterns)]

use blade_graphics as gpu;
use gpu::ShaderData as _;
use bytemuck::{Pod, Zeroable};

const GRID_SIZE: u32 = 20; // 20x20x20 = 8000 cubes
const CUBE_SPACING: f32 = 3.0;
const WORKGROUP_SIZE: u32 = 256;

//=============================================================================
// Shader Data Structures
//=============================================================================

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Frustum {
    planes: [[f32; 4]; 6],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BoundingSphere {
    center: [f32; 3],
    radius: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CullParams {
    object_count: u32,
    vertices_per_object: u32,
    _pad: [u32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct DrawIndirect {
    vertex_count: u32,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ObjectData {
    model: [[f32; 4]; 4],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    view_proj: [[f32; 4]; 4],
}

//=============================================================================
// Shader Data Bindings
//=============================================================================

#[derive(blade_macros::ShaderData)]
struct CullData {
    frustum: Frustum,
    params: CullParams,
    bounds: gpu::BufferPiece,
    indirect: gpu::BufferPiece,
    visible_indices: gpu::BufferPiece,
}

#[derive(blade_macros::ShaderData)]
struct RenderData {
    globals: Globals,
    objects: gpu::BufferPiece,
    visible_indices: gpu::BufferPiece,
}

//=============================================================================
// Application State
//=============================================================================

struct Example {
    context: gpu::Context,
    surface: gpu::Surface,
    reset_pipeline: gpu::ComputePipeline,
    cull_pipeline: gpu::ComputePipeline,
    render_pipeline: gpu::RenderPipeline,
    bounds_buf: gpu::Buffer,
    objects_buf: gpu::Buffer,
    indirect_buf: gpu::Buffer,
    visible_indices_buf: gpu::Buffer,
    command_encoder: gpu::CommandEncoder,
    prev_sync_point: Option<gpu::SyncPoint>,
    object_count: u32,
    camera_angle: f32,
    window_size: winit::dpi::PhysicalSize<u32>,
}

impl Example {
    fn new(window: &winit::window::Window) -> Self {
        let context = unsafe {
            gpu::Context::init(gpu::ContextDesc {
                presentation: true,
                validation: cfg!(debug_assertions),
                timing: false,
                ..Default::default()
            })
            .expect("Failed to create GPU context")
        };

        let mut surface = context
            .create_surface(window)
            .expect("Failed to create surface");

        let size = window.inner_size();
        context.reconfigure_surface(
            &mut surface,
            gpu::SurfaceConfig {
                size: gpu::Extent {
                    width: size.width,
                    height: size.height,
                    depth: 1,
                },
                display_sync: gpu::DisplaySync::Block,
                ..Default::default()
            },
        );

        let surface_info = surface.info();
        let surface_format = surface_info.format;

        // Load shaders
        let cull_source = std::fs::read_to_string("examples/frustum-cull/cull.wgsl")
            .expect("Failed to load cull.wgsl");
        let render_source = std::fs::read_to_string("examples/frustum-cull/render.wgsl")
            .expect("Failed to load render.wgsl");

        let cull_shader = context.create_shader(gpu::ShaderDesc { source: &cull_source });
        let render_shader = context.create_shader(gpu::ShaderDesc { source: &render_source });

        // Create pipelines
        let cull_layout = CullData::layout();
        let render_layout = RenderData::layout();

        let reset_pipeline = context.create_compute_pipeline(gpu::ComputePipelineDesc {
            name: "reset",
            data_layouts: &[&cull_layout],
            compute: cull_shader.at("cs_reset"),
        });

        let cull_pipeline = context.create_compute_pipeline(gpu::ComputePipelineDesc {
            name: "cull",
            data_layouts: &[&cull_layout],
            compute: cull_shader.at("cs_cull"),
        });

        let render_pipeline = context.create_render_pipeline(gpu::RenderPipelineDesc {
            name: "render",
            data_layouts: &[&render_layout],
            vertex: render_shader.at("vs_main"),
            vertex_fetches: &[],
            primitive: gpu::PrimitiveState {
                topology: gpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            fragment: Some(render_shader.at("fs_main")),
            color_targets: &[gpu::ColorTargetState {
                format: surface_format,
                blend: Some(gpu::BlendState::ALPHA_BLENDING),
                write_mask: gpu::ColorWrites::all(),
            }],
            multisample_state: gpu::MultisampleState::default(),
        });

        // Calculate object count
        let object_count = GRID_SIZE * GRID_SIZE * GRID_SIZE;

        // Create buffers
        let bounds_buf = context.create_buffer(gpu::BufferDesc {
            name: "bounds",
            size: (object_count as usize * std::mem::size_of::<BoundingSphere>()) as u64,
            memory: gpu::Memory::Shared,
        });

        let objects_buf = context.create_buffer(gpu::BufferDesc {
            name: "objects",
            size: (object_count as usize * std::mem::size_of::<ObjectData>()) as u64,
            memory: gpu::Memory::Shared,
        });

        let indirect_buf = context.create_buffer(gpu::BufferDesc {
            name: "indirect",
            size: std::mem::size_of::<DrawIndirect>() as u64,
            memory: gpu::Memory::Shared,
        });

        let visible_indices_buf = context.create_buffer(gpu::BufferDesc {
            name: "visible_indices",
            size: (object_count as usize * std::mem::size_of::<u32>()) as u64,
            memory: gpu::Memory::Device,
        });

        // Initialize object data
        let grid_offset = (GRID_SIZE as f32 * CUBE_SPACING) / 2.0;

        for z in 0..GRID_SIZE {
            for y in 0..GRID_SIZE {
                for x in 0..GRID_SIZE {
                    let idx = (z * GRID_SIZE * GRID_SIZE + y * GRID_SIZE + x) as usize;
                    let pos = [
                        x as f32 * CUBE_SPACING - grid_offset,
                        y as f32 * CUBE_SPACING - grid_offset,
                        z as f32 * CUBE_SPACING - grid_offset,
                    ];

                    unsafe {
                        let bounds_ptr = bounds_buf.data() as *mut BoundingSphere;
                        *bounds_ptr.add(idx) = BoundingSphere {
                            center: pos,
                            radius: 0.866,
                        };

                        let objects_ptr = objects_buf.data() as *mut ObjectData;
                        *objects_ptr.add(idx) = ObjectData {
                            model: Self::translation_matrix(pos),
                            color: [
                                x as f32 / GRID_SIZE as f32,
                                y as f32 / GRID_SIZE as f32,
                                z as f32 / GRID_SIZE as f32,
                                1.0,
                            ],
                        };
                    }
                }
            }
        }

        // Initialize indirect buffer
        unsafe {
            let indirect_ptr = indirect_buf.data() as *mut DrawIndirect;
            *indirect_ptr = DrawIndirect {
                vertex_count: 36,
                instance_count: 0,
                first_vertex: 0,
                first_instance: 0,
            };
        }

        context.sync_buffer(bounds_buf);
        context.sync_buffer(objects_buf);
        context.sync_buffer(indirect_buf);

        let command_encoder = context.create_command_encoder(gpu::CommandEncoderDesc {
            name: "main",
            buffer_count: 2,
        });

        println!(
            "GPU Frustum Culling: {} cubes ({}x{}x{})",
            object_count, GRID_SIZE, GRID_SIZE, GRID_SIZE
        );

        Self {
            context,
            surface,
            reset_pipeline,
            cull_pipeline,
            render_pipeline,
            bounds_buf,
            objects_buf,
            indirect_buf,
            visible_indices_buf,
            command_encoder,
            prev_sync_point: None,
            object_count,
            camera_angle: 0.0,
            window_size: size,
        }
    }

    fn translation_matrix(pos: [f32; 3]) -> [[f32; 4]; 4] {
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [pos[0], pos[1], pos[2], 1.0],
        ]
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.window_size = size;
        self.context.reconfigure_surface(
            &mut self.surface,
            gpu::SurfaceConfig {
                size: gpu::Extent {
                    width: size.width,
                    height: size.height,
                    depth: 1,
                },
                display_sync: gpu::DisplaySync::Block,
                ..Default::default()
            },
        );
    }

    fn render(&mut self) {
        if self.window_size.width == 0 || self.window_size.height == 0 {
            return;
        }

        let dt = 1.0 / 60.0;
        self.camera_angle += dt * 0.3;

        let (globals, frustum) = self.compute_camera();

        let frame = self.surface.acquire_frame();

        let cull_data = CullData {
            frustum,
            params: CullParams {
                object_count: self.object_count,
                vertices_per_object: 36,
                _pad: [0; 2],
            },
            bounds: self.bounds_buf.into(),
            indirect: self.indirect_buf.into(),
            visible_indices: self.visible_indices_buf.into(),
        };

        let render_data = RenderData {
            globals,
            objects: self.objects_buf.into(),
            visible_indices: self.visible_indices_buf.into(),
        };

        self.command_encoder.start();
        self.command_encoder.init_texture(frame.texture());

        // Reset indirect buffer
        if let mut pass = self.command_encoder.compute("reset") {
            if let mut pc = pass.with(&self.reset_pipeline) {
                pc.bind(0, &cull_data);
                pc.dispatch([1, 1, 1]);
            }
        }

        // Run frustum culling
        if let mut pass = self.command_encoder.compute("cull") {
            if let mut pc = pass.with(&self.cull_pipeline) {
                pc.bind(0, &cull_data);
                let workgroups = (self.object_count + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
                pc.dispatch([workgroups, 1, 1]);
            }
        }

        // Render visible objects
        if let mut pass = self.command_encoder.render(
            "draw",
            gpu::RenderTargetSet {
                colors: &[gpu::RenderTarget {
                    view: frame.texture_view(),
                    init_op: gpu::InitOp::Clear(gpu::TextureColor::OpaqueBlack),
                    finish_op: gpu::FinishOp::Store,
                }],
                depth_stencil: None,
            },
        ) {
            if let mut pc = pass.with(&self.render_pipeline) {
                pc.bind(0, &render_data);
                pc.draw_indirect(self.indirect_buf.into());
            }
        }

        self.command_encoder.present(frame);
        self.prev_sync_point = Some(self.context.submit(&mut self.command_encoder));
    }

    fn compute_camera(&self) -> (Globals, Frustum) {
        let aspect = self.window_size.width as f32 / self.window_size.height as f32;
        let fov = 60.0_f32.to_radians();
        let near = 0.1;
        let far = 200.0;

        let radius = 80.0;
        let eye = [
            radius * self.camera_angle.cos(),
            30.0,
            radius * self.camera_angle.sin(),
        ];
        let target = [0.0, 0.0, 0.0];
        let up = [0.0, 1.0, 0.0];

        let view = Self::look_at(eye, target, up);
        let proj = Self::perspective(fov, aspect, near, far);
        let view_proj = Self::mat4_mul(proj, view);

        let frustum = Self::extract_frustum_planes(view_proj);

        (Globals { view_proj }, frustum)
    }

    fn look_at(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> [[f32; 4]; 4] {
        let f = Self::normalize([target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]]);
        let s = Self::normalize(Self::cross(f, up));
        let u = Self::cross(s, f);
        [
            [s[0], u[0], -f[0], 0.0],
            [s[1], u[1], -f[1], 0.0],
            [s[2], u[2], -f[2], 0.0],
            [-Self::dot(s, eye), -Self::dot(u, eye), Self::dot(f, eye), 1.0],
        ]
    }

    fn perspective(fov: f32, aspect: f32, near: f32, far: f32) -> [[f32; 4]; 4] {
        let f = 1.0 / (fov / 2.0).tan();
        [
            [f / aspect, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, (far + near) / (near - far), -1.0],
            [0.0, 0.0, (2.0 * far * near) / (near - far), 0.0],
        ]
    }

    fn mat4_mul(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
        let mut r = [[0.0; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    r[i][j] += a[k][j] * b[i][k];
                }
            }
        }
        r
    }

    fn extract_frustum_planes(vp: [[f32; 4]; 4]) -> Frustum {
        let mut planes = [[0.0_f32; 4]; 6];
        // Left, Right, Bottom, Top, Near, Far
        planes[0] = [vp[0][3] + vp[0][0], vp[1][3] + vp[1][0], vp[2][3] + vp[2][0], vp[3][3] + vp[3][0]];
        planes[1] = [vp[0][3] - vp[0][0], vp[1][3] - vp[1][0], vp[2][3] - vp[2][0], vp[3][3] - vp[3][0]];
        planes[2] = [vp[0][3] + vp[0][1], vp[1][3] + vp[1][1], vp[2][3] + vp[2][1], vp[3][3] + vp[3][1]];
        planes[3] = [vp[0][3] - vp[0][1], vp[1][3] - vp[1][1], vp[2][3] - vp[2][1], vp[3][3] - vp[3][1]];
        planes[4] = [vp[0][3] + vp[0][2], vp[1][3] + vp[1][2], vp[2][3] + vp[2][2], vp[3][3] + vp[3][2]];
        planes[5] = [vp[0][3] - vp[0][2], vp[1][3] - vp[1][2], vp[2][3] - vp[2][2], vp[3][3] - vp[3][2]];
        for p in &mut planes {
            let len = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
            if len > 0.0 { p[0] /= len; p[1] /= len; p[2] /= len; p[3] /= len; }
        }
        Frustum { planes }
    }

    fn normalize(v: [f32; 3]) -> [f32; 3] {
        let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if len > 0.0 { [v[0] / len, v[1] / len, v[2] / len] } else { v }
    }

    fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
    }

    fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }

    fn deinit(&mut self) {
        if let Some(ref sp) = self.prev_sync_point {
            self.context.wait_for(sp, !0);
        }
        self.context.destroy_buffer(self.bounds_buf);
        self.context.destroy_buffer(self.objects_buf);
        self.context.destroy_buffer(self.indirect_buf);
        self.context.destroy_buffer(self.visible_indices_buf);
        self.context.destroy_compute_pipeline(&mut self.reset_pipeline);
        self.context.destroy_compute_pipeline(&mut self.cull_pipeline);
        self.context.destroy_render_pipeline(&mut self.render_pipeline);
        self.context.destroy_command_encoder(&mut self.command_encoder);
        self.context.destroy_surface(&mut self.surface);
    }
}

//=============================================================================
// Main Entry Point
//=============================================================================

fn main() {
    env_logger::init();

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let window_attributes = winit::window::Window::default_attributes()
        .with_title("GPU Frustum Culling");
    let window = event_loop.create_window(window_attributes).unwrap();

    let mut example = Example::new(&window);

    #[allow(deprecated)]
    event_loop
        .run(|event, target| {
            use winit::event::{Event, WindowEvent};
            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => target.exit(),
                    WindowEvent::Resized(size) => example.resize(size),
                    WindowEvent::RedrawRequested => example.render(),
                    _ => {}
                },
                Event::AboutToWait => window.request_redraw(),
                _ => {}
            }
        })
        .unwrap();

    example.deinit();
}
