//! Frustum Cull Baseline Example
//!
//! Draws ALL cubes without GPU culling - baseline for comparison with frustum-cull.
//! Use this to verify rendering is correct before adding culling.

#![allow(irrefutable_let_patterns)]

use blade_graphics as gpu;
use gpu::ShaderData as _;
use bytemuck::{Pod, Zeroable};

const GRID_SIZE: u32 = 20; // 20x20x20 = 8000 cubes
const CUBE_SPACING: f32 = 3.0;

//=============================================================================
// Shader Data Structures
//=============================================================================

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
struct RenderData {
    globals: Globals,
    objects: gpu::BufferPiece,
}

//=============================================================================
// Application State
//=============================================================================

struct Example {
    context: gpu::Context,
    surface: gpu::Surface,
    render_pipeline: gpu::RenderPipeline,
    objects_buf: gpu::Buffer,
    command_encoder: gpu::CommandEncoder,
    prev_sync_point: Option<gpu::SyncPoint>,
    object_count: u32,
    camera_angle: f32,
    window_size: winit::dpi::PhysicalSize<u32>,
}

impl Example {
    fn make_surface_config(size: winit::dpi::PhysicalSize<u32>) -> gpu::SurfaceConfig {
        log::info!("Window size: {:?}", size);
        gpu::SurfaceConfig {
            size: gpu::Extent {
                width: size.width,
                height: size.height,
                depth: 1,
            },
            usage: gpu::TextureUsage::TARGET,
            #[cfg(all(target_arch = "wasm32", blade_wgpu))]
            display_sync: gpu::DisplaySync::Block,
            #[cfg(not(all(target_arch = "wasm32", blade_wgpu)))]
            display_sync: gpu::DisplaySync::Recent,
            ..Default::default()
        }
    }

    /// Sync initialization for native and GLES WASM
    #[cfg(not(all(target_arch = "wasm32", blade_wgpu)))]
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
        Self::init_with_context(context, window)
    }

    /// Async initialization for WebGPU WASM
    #[cfg(all(target_arch = "wasm32", blade_wgpu))]
    async fn new_async(window: &winit::window::Window) -> Self {
        let context = gpu::Context::init_async(gpu::ContextDesc {
            presentation: true,
            validation: cfg!(debug_assertions),
            timing: false,
            capture: false,
            overlay: false,
            device_id: 0,
        })
        .await
        .expect("Failed to create GPU context");
        Self::init_with_context(context, window)
    }

    fn init_with_context(context: gpu::Context, window: &winit::window::Window) -> Self {
        println!("{:?}", context.device_information());
        let window_size = window.inner_size();

        let surface = context
            .create_surface_configured(window, Self::make_surface_config(window_size))
            .expect("Failed to create surface");

        let surface_info = surface.info();
        let surface_format = surface_info.format;

        // Load shader
        #[cfg(target_arch = "wasm32")]
        let render_source = include_str!("render.wgsl");
        #[cfg(not(target_arch = "wasm32"))]
        let render_source = std::fs::read_to_string("examples/frustum-cull-baseline/render.wgsl")
            .expect("Failed to load render.wgsl");

        let render_shader = context.create_shader(gpu::ShaderDesc { source: &render_source });

        // Create pipeline
        let render_layout = RenderData::layout();

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

        // Create buffer
        let objects_buf = context.create_buffer(gpu::BufferDesc {
            name: "objects",
            size: (object_count as usize * std::mem::size_of::<ObjectData>()) as u64,
            memory: gpu::Memory::Shared,
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

        context.sync_buffer(objects_buf);

        let command_encoder = context.create_command_encoder(gpu::CommandEncoderDesc {
            name: "main",
            buffer_count: 2,
        });

        log::info!(
            "Frustum Cull BASELINE: {} cubes ({}x{}x{}) - NO CULLING",
            object_count, GRID_SIZE, GRID_SIZE, GRID_SIZE
        );

        Self {
            context,
            surface,
            render_pipeline,
            objects_buf,
            command_encoder,
            prev_sync_point: None,
            object_count,
            camera_angle: 0.0,
            window_size,
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
        self.context.reconfigure_surface(&mut self.surface, Self::make_surface_config(size));
    }

    fn render(&mut self) {
        if self.window_size.width == 0 || self.window_size.height == 0 {
            return;
        }

        let dt = 1.0 / 60.0;
        self.camera_angle += dt * 0.3;

        let globals = self.compute_camera();

        let frame = self.surface.acquire_frame();

        let render_data = RenderData {
            globals,
            objects: self.objects_buf.into(),
        };

        self.command_encoder.start();
        self.command_encoder.init_texture(frame.texture());

        // Render ALL objects - no culling
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
                // Draw ALL cubes - 36 vertices per cube, object_count instances
                pc.draw(0, 36, 0, self.object_count);
            }
        }

        self.command_encoder.present(frame);
        self.prev_sync_point = Some(self.context.submit(&mut self.command_encoder));
    }

    fn compute_camera(&self) -> Globals {
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

        Globals { view_proj }
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
        self.context.destroy_buffer(self.objects_buf);
        self.context.destroy_render_pipeline(&mut self.render_pipeline);
        self.context.destroy_command_encoder(&mut self.command_encoder);
        self.context.destroy_surface(&mut self.surface);
    }
}

//=============================================================================
// Main Entry Points
//=============================================================================

/// Main for native and GLES WASM (sync init)
#[cfg(not(all(target_arch = "wasm32", blade_wgpu)))]
fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let window_attributes = winit::window::Window::default_attributes()
        .with_title("Frustum Cull BASELINE (No Culling)");
    let window = event_loop.create_window(window_attributes).unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::WindowExtWebSys as _;
        console_error_panic_hook::set_once();
        console_log::init().expect("could not initialize logger");
        let canvas = window.canvas().unwrap();
        canvas.set_id(gpu::CANVAS_ID);
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| body.append_child(&web_sys::Element::from(canvas)).ok())
            .expect("couldn't append canvas to document body");
    }

    let mut example = Example::new(&window);

    #[cfg(not(target_arch = "wasm32"))]
    let mut last_snapshot = std::time::Instant::now();
    #[cfg(not(target_arch = "wasm32"))]
    let mut frame_count = 0u32;

    #[allow(deprecated)]
    event_loop
        .run(|event, target| {
            target.set_control_flow(winit::event_loop::ControlFlow::Poll);
            use winit::event::{Event, WindowEvent};
            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => target.exit(),
                    WindowEvent::Resized(size) => example.resize(size),
                    WindowEvent::RedrawRequested => {
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            frame_count += 1;
                            if frame_count == 100 {
                                let accum_time = last_snapshot.elapsed().as_secs_f32();
                                println!(
                                    "BASELINE Avg frame time {}ms",
                                    accum_time * 1000.0 / frame_count as f32
                                );
                                last_snapshot = std::time::Instant::now();
                                frame_count = 0;
                            }
                        }
                        example.render();
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    WindowEvent::KeyboardInput {
                        event: winit::event::KeyEvent {
                            physical_key: winit::keyboard::PhysicalKey::Code(
                                winit::keyboard::KeyCode::Escape,
                            ),
                            state: winit::event::ElementState::Pressed,
                            ..
                        },
                        ..
                    } => target.exit(),
                    _ => {}
                },
                Event::AboutToWait => window.request_redraw(),
                _ => {}
            }
        })
        .unwrap();

    example.deinit();
}

/// Main for WebGPU WASM (async init)
#[cfg(all(target_arch = "wasm32", blade_wgpu))]
fn main() {
    use std::cell::RefCell;
    use std::rc::Rc;
    use winit::platform::web::WindowExtWebSys as _;

    console_error_panic_hook::set_once();
    console_log::init().expect("could not initialize logger");

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let window_attributes = winit::window::Window::default_attributes()
        .with_title("Frustum Cull BASELINE (No Culling)");
    let window = Rc::new(event_loop.create_window(window_attributes).unwrap());

    // Set up canvas
    let canvas = window.canvas().unwrap();
    canvas.set_id(gpu::CANVAS_ID);
    web_sys::window()
        .and_then(|win| win.document())
        .and_then(|doc| doc.body())
        .and_then(|body| body.append_child(&web_sys::Element::from(canvas)).ok())
        .expect("couldn't append canvas to document body");

    // State machine: None = initializing, Some = ready
    let example: Rc<RefCell<Option<Example>>> = Rc::new(RefCell::new(None));
    let init_started: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
    let frame_count: Rc<RefCell<u32>> = Rc::new(RefCell::new(0));

    // Performance timing
    let perf = web_sys::window().unwrap().performance().unwrap();
    let last_time: Rc<RefCell<f64>> = Rc::new(RefCell::new(perf.now()));
    let perf = Rc::new(perf);

    let example_clone = example.clone();
    let init_started_clone = init_started.clone();
    let window_clone = window.clone();

    event_loop
        .run(move |event, target| {
            target.set_control_flow(winit::event_loop::ControlFlow::Wait);
            match event {
                winit::event::Event::AboutToWait => {
                    // Start async init on first frame
                    if !*init_started_clone.borrow() {
                        *init_started_clone.borrow_mut() = true;
                        let example_init = example_clone.clone();
                        let window_init = window_clone.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            let ex = Example::new_async(&window_init).await;
                            *example_init.borrow_mut() = Some(ex);
                            log::info!("WebGPU frustum-cull-baseline initialized");
                        });
                    }
                    window.request_redraw();
                }
                winit::event::Event::WindowEvent { event, .. } => match event {
                    winit::event::WindowEvent::Resized(size) => {
                        if let Some(ref mut ex) = *example.borrow_mut() {
                            ex.resize(size);
                        }
                    }
                    winit::event::WindowEvent::CloseRequested => {
                        target.exit();
                    }
                    winit::event::WindowEvent::RedrawRequested => {
                        if let Some(ref mut ex) = *example.borrow_mut() {
                            let count = {
                                let mut fc = frame_count.borrow_mut();
                                *fc += 1;
                                *fc
                            };

                            ex.render();

                            // Log performance every 100 frames
                            if count % 100 == 0 {
                                let now = perf.now();
                                let elapsed = now - *last_time.borrow();
                                let avg_frame_ms = elapsed / 100.0;
                                let fps = 1000.0 / avg_frame_ms;

                                log::info!(
                                    "BASELINE Frame {}: avg {:.2}ms ({:.1} FPS), {} cubes (NO CULLING)",
                                    count, avg_frame_ms, fps, ex.object_count
                                );
                                *last_time.borrow_mut() = now;
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        })
        .unwrap();
}
