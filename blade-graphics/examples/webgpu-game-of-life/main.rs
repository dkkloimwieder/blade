//! WebGPU Game of Life Example
//!
//! Demonstrates compute shader ping-pong simulation pattern.
//!
//! Key patterns shown:
//! - Storage textures for compute read/write
//! - Ping-pong double buffering (swap textures each frame)
//! - Compute dispatch based on texture dimensions
//! - Separate compute and render pipelines
//! - Cellular automata implementation
//!
//! Run with: RUSTFLAGS="--cfg blade_wgpu" cargo run -p blade-graphics --example webgpu-game-of-life
//! For WASM: RUSTFLAGS="--cfg blade_wgpu" cargo run-wasm -p blade-graphics --example webgpu-game-of-life

#![allow(irrefutable_let_patterns)]

use blade_graphics as gpu;
use std::ptr;

// -----------------------------------------------------------------------------
// Simulation Parameters
// -----------------------------------------------------------------------------

const GRID_WIDTH: u32 = 128;
const GRID_HEIGHT: u32 = 128;
const WORKGROUP_SIZE: u32 = 8;

// -----------------------------------------------------------------------------
// Shader Data Bindings
// -----------------------------------------------------------------------------

/// Compute shader bindings: read from one texture, write to another
struct ComputeParams {
    input_tex: gpu::TextureView,
    output_tex: gpu::TextureView,
}

impl gpu::ShaderData for ComputeParams {
    fn layout() -> gpu::ShaderDataLayout {
        gpu::ShaderDataLayout {
            bindings: vec![
                ("input_tex", gpu::ShaderBinding::Texture),
                ("output_tex", gpu::ShaderBinding::Texture),
            ],
        }
    }

    fn fill(&self, mut ctx: gpu::PipelineContext) {
        use gpu::ShaderBindable as _;
        self.input_tex.bind_to(&mut ctx, 0);
        self.output_tex.bind_to(&mut ctx, 1);
    }
}

/// Render shader bindings: display the current state
struct RenderParams {
    state_texture: gpu::TextureView,
    state_sampler: gpu::Sampler,
}

impl gpu::ShaderData for RenderParams {
    fn layout() -> gpu::ShaderDataLayout {
        gpu::ShaderDataLayout {
            bindings: vec![
                ("state_texture", gpu::ShaderBinding::Texture),
                ("state_sampler", gpu::ShaderBinding::Sampler),
            ],
        }
    }

    fn fill(&self, mut ctx: gpu::PipelineContext) {
        use gpu::ShaderBindable as _;
        self.state_texture.bind_to(&mut ctx, 0);
        self.state_sampler.bind_to(&mut ctx, 1);
    }
}

// -----------------------------------------------------------------------------
// Initial Pattern Generation
// -----------------------------------------------------------------------------

/// Generate random initial state with ~30% live cells
/// Returns RGBA8 data (4 bytes per pixel)
fn generate_random_state(width: u32, height: u32, seed: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    let mut rng = seed;

    for _ in 0..(width * height) {
        // Simple LCG random number generator
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        // ~20% chance of being alive (lower density = more dynamic patterns)
        let alive = (rng >> 33) % 100 < 20;
        let value = if alive { 255u8 } else { 0u8 };
        // RGBA: store state in all channels for visibility
        data.extend_from_slice(&[value, value, value, 255]);
    }

    data
}

// -----------------------------------------------------------------------------
// Example Application
// -----------------------------------------------------------------------------

struct Example {
    context: gpu::Context,
    surface: gpu::Surface,
    compute_pipeline: gpu::ComputePipeline,
    render_pipeline: gpu::RenderPipeline,
    command_encoder: gpu::CommandEncoder,
    // Double-buffered textures for ping-pong
    textures: [gpu::Texture; 2],
    texture_views: [gpu::TextureView; 2],
    // Render-only views (for sampling, not storage)
    render_views: [gpu::TextureView; 2],
    sampler: gpu::Sampler,
    // Current read/write indices (swapped each frame)
    current_read: usize,
    prev_sync_point: Option<gpu::SyncPoint>,
    window_size: winit::dpi::PhysicalSize<u32>,
}

impl Example {
    fn make_surface_config(size: winit::dpi::PhysicalSize<u32>) -> gpu::SurfaceConfig {
        gpu::SurfaceConfig {
            size: gpu::Extent {
                width: size.width,
                height: size.height,
                depth: 1,
            },
            usage: gpu::TextureUsage::TARGET,
            display_sync: gpu::DisplaySync::Block,
            ..Default::default()
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn new(window: &winit::window::Window) -> Self {
        let context = unsafe {
            gpu::Context::init(gpu::ContextDesc {
                presentation: true,
                validation: cfg!(debug_assertions),
                timing: false,
                capture: false,
                overlay: false,
                device_id: 0,
            })
            .unwrap()
        };
        Self::init_with_context(context, window)
    }

    #[cfg(target_arch = "wasm32")]
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
        .unwrap();
        Self::init_with_context(context, window)
    }

    fn init_with_context(context: gpu::Context, window: &winit::window::Window) -> Self {
        println!("Device: {:?}", context.device_information());

        let window_size = window.inner_size();
        let surface = context
            .create_surface_configured(window, Self::make_surface_config(window_size))
            .unwrap();

        // ---------------------------------------------------------------------
        // Load shaders
        // ---------------------------------------------------------------------
        #[cfg(target_arch = "wasm32")]
        let compute_source = include_str!("compute.wgsl");
        #[cfg(not(target_arch = "wasm32"))]
        let compute_source =
            std::fs::read_to_string("blade-graphics/examples/webgpu-game-of-life/compute.wgsl")
                .unwrap();

        #[cfg(target_arch = "wasm32")]
        let render_source = include_str!("render.wgsl");
        #[cfg(not(target_arch = "wasm32"))]
        let render_source =
            std::fs::read_to_string("blade-graphics/examples/webgpu-game-of-life/render.wgsl")
                .unwrap();

        let compute_shader = context.create_shader(gpu::ShaderDesc {
            source: &compute_source,
        });
        let render_shader = context.create_shader(gpu::ShaderDesc {
            source: &render_source,
        });

        // ---------------------------------------------------------------------
        // Create compute pipeline
        // ---------------------------------------------------------------------
        let compute_layout = <ComputeParams as gpu::ShaderData>::layout();
        let compute_pipeline = context.create_compute_pipeline(gpu::ComputePipelineDesc {
            name: "game_of_life_compute",
            data_layouts: &[&compute_layout],
            compute: compute_shader.at("cs_main"),
        });

        // ---------------------------------------------------------------------
        // Create render pipeline
        // ---------------------------------------------------------------------
        let render_layout = <RenderParams as gpu::ShaderData>::layout();
        let render_pipeline = context.create_render_pipeline(gpu::RenderPipelineDesc {
            name: "game_of_life_render",
            data_layouts: &[&render_layout],
            vertex: render_shader.at("vs_main"),
            vertex_fetches: &[],
            fragment: Some(render_shader.at("fs_main")),
            primitive: gpu::PrimitiveState {
                topology: gpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            color_targets: &[gpu::ColorTargetState {
                format: surface.info().format,
                blend: None,
                write_mask: gpu::ColorWrites::ALL,
            }],
            multisample_state: gpu::MultisampleState::default(),
        });

        // ---------------------------------------------------------------------
        // Create double-buffered textures (ping-pong)
        // ---------------------------------------------------------------------
        let extent = gpu::Extent {
            width: GRID_WIDTH,
            height: GRID_HEIGHT,
            depth: 1,
        };

        // Create texture A
        let texture_a = context.create_texture(gpu::TextureDesc {
            name: "state_a",
            format: gpu::TextureFormat::Rgba8Unorm,
            size: extent,
            dimension: gpu::TextureDimension::D2,
            array_layer_count: 1,
            mip_level_count: 1,
            // RESOURCE: can be sampled
            // STORAGE: can be read/written in compute
            // COPY: can be copy destination (for initial upload)
            usage: gpu::TextureUsage::RESOURCE | gpu::TextureUsage::STORAGE | gpu::TextureUsage::COPY,
            sample_count: 1,
            external: None,
        });

        // Create texture B
        let texture_b = context.create_texture(gpu::TextureDesc {
            name: "state_b",
            format: gpu::TextureFormat::Rgba8Unorm,
            size: extent,
            dimension: gpu::TextureDimension::D2,
            array_layer_count: 1,
            mip_level_count: 1,
            usage: gpu::TextureUsage::RESOURCE | gpu::TextureUsage::STORAGE | gpu::TextureUsage::COPY,
            sample_count: 1,
            external: None,
        });

        let textures = [texture_a, texture_b];

        // Storage views for compute shader
        let texture_views = [
            context.create_texture_view(
                textures[0],
                gpu::TextureViewDesc {
                    name: "state_a_storage",
                    format: gpu::TextureFormat::Rgba8Unorm,
                    dimension: gpu::ViewDimension::D2,
                    subresources: &Default::default(),
                },
            ),
            context.create_texture_view(
                textures[1],
                gpu::TextureViewDesc {
                    name: "state_b_storage",
                    format: gpu::TextureFormat::Rgba8Unorm,
                    dimension: gpu::ViewDimension::D2,
                    subresources: &Default::default(),
                },
            ),
        ];

        // Resource views for render shader
        let render_views = [
            context.create_texture_view(
                textures[0],
                gpu::TextureViewDesc {
                    name: "state_a_render",
                    format: gpu::TextureFormat::Rgba8Unorm,
                    dimension: gpu::ViewDimension::D2,
                    subresources: &Default::default(),
                },
            ),
            context.create_texture_view(
                textures[1],
                gpu::TextureViewDesc {
                    name: "state_b_render",
                    format: gpu::TextureFormat::Rgba8Unorm,
                    dimension: gpu::ViewDimension::D2,
                    subresources: &Default::default(),
                },
            ),
        ];

        // Sampler for render (nearest for crisp pixels)
        let sampler = context.create_sampler(gpu::SamplerDesc {
            name: "state_sampler",
            mag_filter: gpu::FilterMode::Nearest,
            min_filter: gpu::FilterMode::Nearest,
            ..Default::default()
        });

        // ---------------------------------------------------------------------
        // Upload initial state
        // ---------------------------------------------------------------------
        let initial_data = generate_random_state(GRID_WIDTH, GRID_HEIGHT, 42);
        let bytes_per_row = GRID_WIDTH * 4; // RGBA8 = 4 bytes per pixel

        let upload_buffer = context.create_buffer(gpu::BufferDesc {
            name: "initial_state_staging",
            size: initial_data.len() as u64,
            memory: gpu::Memory::Upload,
        });

        unsafe {
            ptr::copy_nonoverlapping(
                initial_data.as_ptr(),
                upload_buffer.data(),
                initial_data.len(),
            );
        }
        context.sync_buffer(upload_buffer);

        // Upload to first texture
        let mut command_encoder = context.create_command_encoder(gpu::CommandEncoderDesc {
            name: "main",
            buffer_count: 2,
        });

        command_encoder.start();
        command_encoder.init_texture(textures[0]);
        command_encoder.init_texture(textures[1]);

        if let mut transfer = command_encoder.transfer("upload_initial_state") {
            transfer.copy_buffer_to_texture(upload_buffer.into(), bytes_per_row, textures[0].into(), extent);
        }

        let sync_point = context.submit(&mut command_encoder);
        context.wait_for(&sync_point, !0);

        context.destroy_buffer(upload_buffer);

        Self {
            context,
            surface,
            compute_pipeline,
            render_pipeline,
            command_encoder,
            textures,
            texture_views,
            render_views,
            sampler,
            current_read: 0,
            prev_sync_point: None,
            window_size,
        }
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.window_size = size;
        self.context
            .reconfigure_surface(&mut self.surface, Self::make_surface_config(size));
    }

    fn render(&mut self) {
        // Wait for previous frame
        if let Some(ref sp) = self.prev_sync_point {
            self.context.wait_for(sp, !0);
        }

        let frame = self.surface.acquire_frame();
        self.command_encoder.start();

        // Compute pass: simulate one generation
        // Read from current_read, write to 1-current_read
        let read_idx = self.current_read;
        let write_idx = 1 - self.current_read;

        if let mut compute = self.command_encoder.compute("game_of_life_step") {
            if let mut pc = compute.with(&self.compute_pipeline) {
                pc.bind(
                    0,
                    &ComputeParams {
                        input_tex: self.texture_views[read_idx],
                        output_tex: self.texture_views[write_idx],
                    },
                );
                // Dispatch workgroups to cover entire grid
                let groups_x = (GRID_WIDTH + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
                let groups_y = (GRID_HEIGHT + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
                pc.dispatch([groups_x, groups_y, 1]);
            }
        }

        // Render pass: display the newly computed state
        if let mut pass = self.command_encoder.render(
            "display",
            gpu::RenderTargetSet {
                colors: &[gpu::RenderTarget {
                    view: frame.texture_view(),
                    init_op: gpu::InitOp::Clear(gpu::TextureColor::OpaqueBlack),
                    finish_op: gpu::FinishOp::Store,
                }],
                depth_stencil: None,
            },
        ) {
            if let mut rc = pass.with(&self.render_pipeline) {
                rc.bind(
                    0,
                    &RenderParams {
                        // Display the texture we just wrote to
                        state_texture: self.render_views[write_idx],
                        state_sampler: self.sampler,
                    },
                );
                rc.draw(0, 6, 0, 1);
            }
        }

        self.command_encoder.present(frame);
        self.prev_sync_point = Some(self.context.submit(&mut self.command_encoder));

        // Swap buffers for next frame (ping-pong)
        self.current_read = write_idx;
    }

    fn deinit(mut self) {
        if let Some(ref sp) = self.prev_sync_point {
            self.context.wait_for(sp, !0);
        }
        for i in 0..2 {
            self.context.destroy_texture_view(self.texture_views[i]);
            self.context.destroy_texture_view(self.render_views[i]);
            self.context.destroy_texture(self.textures[i]);
        }
        self.context.destroy_sampler(self.sampler);
        self.context.destroy_command_encoder(&mut self.command_encoder);
        self.context.destroy_surface(&mut self.surface);
    }
}

// -----------------------------------------------------------------------------
// Native main()
// -----------------------------------------------------------------------------
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    env_logger::init();

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let window_attributes =
        winit::window::Window::default_attributes().with_title("blade-webgpu-game-of-life");
    let window = event_loop.create_window(window_attributes).unwrap();

    let mut example = Example::new(&window);

    event_loop
        .run(|event, target| {
            target.set_control_flow(winit::event_loop::ControlFlow::Poll);
            match event {
                winit::event::Event::AboutToWait => {
                    window.request_redraw();
                }
                winit::event::Event::WindowEvent { event, .. } => match event {
                    winit::event::WindowEvent::Resized(size) => {
                        example.resize(size);
                    }
                    winit::event::WindowEvent::KeyboardInput {
                        event:
                            winit::event::KeyEvent {
                                physical_key: winit::keyboard::PhysicalKey::Code(key_code),
                                state: winit::event::ElementState::Pressed,
                                ..
                            },
                        ..
                    } => {
                        if key_code == winit::keyboard::KeyCode::Escape {
                            target.exit();
                        }
                    }
                    winit::event::WindowEvent::CloseRequested => {
                        target.exit();
                    }
                    winit::event::WindowEvent::RedrawRequested => {
                        example.render();
                    }
                    _ => {}
                },
                _ => {}
            }
        })
        .unwrap();

    example.deinit();
}

// -----------------------------------------------------------------------------
// WASM main()
// -----------------------------------------------------------------------------
#[cfg(target_arch = "wasm32")]
fn main() {
    use std::cell::RefCell;
    use std::rc::Rc;
    use winit::platform::web::WindowExtWebSys as _;

    console_error_panic_hook::set_once();
    console_log::init().expect("could not initialize logger");

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let window_attributes =
        winit::window::Window::default_attributes().with_title("blade-webgpu-game-of-life");
    let window = Rc::new(event_loop.create_window(window_attributes).unwrap());

    let canvas = window.canvas().unwrap();
    canvas.set_id(gpu::CANVAS_ID);
    web_sys::window()
        .and_then(|win| win.document())
        .and_then(|doc| doc.body())
        .and_then(|body| body.append_child(&web_sys::Element::from(canvas)).ok())
        .expect("couldn't append canvas to document body");

    let example: Rc<RefCell<Option<Example>>> = Rc::new(RefCell::new(None));
    let init_started: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));

    let example_clone = example.clone();
    let init_started_clone = init_started.clone();
    let window_clone = window.clone();

    event_loop
        .run(move |event, target| {
            target.set_control_flow(winit::event_loop::ControlFlow::Wait);
            match event {
                winit::event::Event::AboutToWait => {
                    if !*init_started_clone.borrow() {
                        *init_started_clone.borrow_mut() = true;
                        let example_init = example_clone.clone();
                        let window_init = window_clone.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            let ex = Example::new_async(&window_init).await;
                            *example_init.borrow_mut() = Some(ex);
                            log::info!("Game of Life initialized!");
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
                            ex.render();
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        })
        .unwrap();
}
