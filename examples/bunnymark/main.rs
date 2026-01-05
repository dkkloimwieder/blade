#![allow(irrefutable_let_patterns)]

use blade_graphics as gpu;
use bytemuck::{Pod, Zeroable};
use std::{mem, ptr};

const BUNNY_SIZE: f32 = 0.15 * 256.0;
const GRAVITY: f32 = -9.8 * 100.0;
const MAX_VELOCITY: i32 = 750;
const WORKGROUP_SIZE: u32 = 256;

// Instance data: one per bunny, matches shader InstanceData struct
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct InstanceData {
    position: [f32; 2],
    velocity: [f32; 2],
    color: u32,
    pad: u32,
}

// Group 0: Static resources (cached - bind group is reused)
#[derive(blade_macros::ShaderData)]
struct StaticParams {
    sprite_texture: gpu::TextureView,
    sprite_sampler: gpu::Sampler,
    instances: gpu::BufferPiece,
}

// Group 1: Per-frame uniform (recreated each frame - cheap, only uniform data)
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    mvp_transform: [[f32; 4]; 4],
    sprite_size: [f32; 2],
    pad: [f32; 2],
}

#[derive(blade_macros::ShaderData)]
struct FrameParams {
    globals: Globals,
}

// Compute pipeline: Physics simulation params
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SimParams {
    delta_time: f32,
    gravity: f32,
    bounds_width: f32,
    bounds_height: f32,
    sprite_half_size: f32,
    bunny_count: u32,
    _pad: [f32; 2],
}

#[derive(blade_macros::ShaderData)]
struct ComputeParams {
    sim_params: SimParams,
    instances_rw: gpu::BufferPiece,
}

#[derive(blade_macros::Vertex)]
struct SpriteVertex {
    pos: [f32; 2],
}

const MAX_BUNNIES: usize = 100_000;

struct Example {
    render_pipeline: gpu::RenderPipeline,
    compute_pipeline: gpu::ComputePipeline,
    command_encoder: gpu::CommandEncoder,
    prev_sync_point: Option<gpu::SyncPoint>,
    texture: gpu::Texture,
    view: gpu::TextureView,
    sampler: gpu::Sampler,
    vertex_buf: gpu::Buffer,
    instance_buf: gpu::Buffer,
    staging_buf: gpu::Buffer,
    window_size: winit::dpi::PhysicalSize<u32>,
    bunny_count: usize,
    pending_bunnies: Vec<InstanceData>,
    rng: nanorand::WyRand,
    surface: gpu::Surface,
    context: gpu::Context,
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
            // WebGPU on web only supports FIFO/Auto present modes
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
                capture: false,
                overlay: true,
                device_id: 0,
            })
            .unwrap()
        };
        Self::init_with_context(context, window)
    }

    /// Async initialization for WebGPU WASM
    #[cfg(all(target_arch = "wasm32", blade_wgpu))]
    async fn new_async(window: &winit::window::Window) -> Self {
        let context = gpu::Context::init_async(gpu::ContextDesc {
            presentation: true,
            validation: cfg!(debug_assertions),
            timing: false, // Timing queries add overhead on WASM
            capture: false,
            overlay: false, // overlay not supported on WASM
            device_id: 0,
        })
        .await
        .unwrap();
        Self::init_with_context(context, window)
    }

    fn init_with_context(context: gpu::Context, window: &winit::window::Window) -> Self {
        println!("{:?}", context.device_information());
        let window_size = window.inner_size();

        let surface = context
            .create_surface_configured(window, Self::make_surface_config(window_size))
            .unwrap();

        let static_layout = <StaticParams as gpu::ShaderData>::layout();
        let frame_layout = <FrameParams as gpu::ShaderData>::layout();
        let compute_layout = <ComputeParams as gpu::ShaderData>::layout();

        // Load render shader
        #[cfg(target_arch = "wasm32")]
        let render_source = include_str!("shader.wgsl");
        #[cfg(not(target_arch = "wasm32"))]
        let render_source = std::fs::read_to_string("examples/bunnymark/shader.wgsl").unwrap();
        let render_shader = context.create_shader(gpu::ShaderDesc {
            source: &render_source,
        });

        // Load compute shader (separate file to avoid binding conflicts)
        #[cfg(target_arch = "wasm32")]
        let compute_source = include_str!("compute.wgsl");
        #[cfg(not(target_arch = "wasm32"))]
        let compute_source = std::fs::read_to_string("examples/bunnymark/compute.wgsl").unwrap();
        let compute_shader = context.create_shader(gpu::ShaderDesc {
            source: &compute_source,
        });

        let render_pipeline = context.create_render_pipeline(gpu::RenderPipelineDesc {
            name: "render",
            data_layouts: &[&static_layout, &frame_layout],
            vertex: render_shader.at("vs_main"),
            vertex_fetches: &[gpu::VertexFetchState {
                layout: &<SpriteVertex as gpu::Vertex>::layout(),
                instanced: false,
            }],
            primitive: gpu::PrimitiveState {
                topology: gpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            fragment: Some(render_shader.at("fs_main")),
            color_targets: &[gpu::ColorTargetState {
                format: surface.info().format,
                blend: Some(gpu::BlendState::ALPHA_BLENDING),
                write_mask: gpu::ColorWrites::default(),
            }],
            multisample_state: gpu::MultisampleState::default(),
        });

        let compute_pipeline = context.create_compute_pipeline(gpu::ComputePipelineDesc {
            name: "physics",
            data_layouts: &[&compute_layout],
            compute: compute_shader.at("cs_update"),
        });

        let extent = gpu::Extent {
            width: 1,
            height: 1,
            depth: 1,
        };
        let texture = context.create_texture(gpu::TextureDesc {
            name: "texutre",
            format: gpu::TextureFormat::Rgba8Unorm,
            size: extent,
            dimension: gpu::TextureDimension::D2,
            array_layer_count: 1,
            mip_level_count: 1,
            usage: gpu::TextureUsage::RESOURCE | gpu::TextureUsage::COPY,
            sample_count: 1,
            external: None,
        });
        let view = context.create_texture_view(
            texture,
            gpu::TextureViewDesc {
                name: "view",
                format: gpu::TextureFormat::Rgba8Unorm,
                dimension: gpu::ViewDimension::D2,
                subresources: &Default::default(),
            },
        );

        let upload_buffer = context.create_buffer(gpu::BufferDesc {
            name: "staging",
            size: (extent.width * extent.height) as u64 * 4,
            memory: gpu::Memory::Upload,
        });
        let texture_data = [0xFFu8; 4];
        unsafe {
            ptr::copy_nonoverlapping(
                texture_data.as_ptr(),
                upload_buffer.data(),
                texture_data.len(),
            );
        }
        context.sync_buffer(upload_buffer);

        let sampler = context.create_sampler(gpu::SamplerDesc {
            name: "main",
            ..Default::default()
        });

        let vertex_data = [
            SpriteVertex { pos: [0.0, 0.0] },
            SpriteVertex { pos: [1.0, 0.0] },
            SpriteVertex { pos: [0.0, 1.0] },
            SpriteVertex { pos: [1.0, 1.0] },
        ];
        let vertex_buf = context.create_buffer(gpu::BufferDesc {
            name: "vertex",
            size: (vertex_data.len() * mem::size_of::<SpriteVertex>()) as u64,
            memory: gpu::Memory::Shared,
        });
        unsafe {
            ptr::copy_nonoverlapping(
                vertex_data.as_ptr(),
                vertex_buf.data() as *mut SpriteVertex,
                vertex_data.len(),
            );
        }
        context.sync_buffer(vertex_buf);

        // Instance buffer for all bunnies - GPU-only, updated by compute shader
        // Using Device memory eliminates the shadow buffer and WASM→JS copy overhead
        let instance_buf = context.create_buffer(gpu::BufferDesc {
            name: "instances",
            size: (MAX_BUNNIES * mem::size_of::<InstanceData>()) as u64,
            memory: gpu::Memory::Device,
        });

        // Staging buffer for uploading new bunnies to GPU
        let staging_buf = context.create_buffer(gpu::BufferDesc {
            name: "staging",
            size: (MAX_BUNNIES * mem::size_of::<InstanceData>()) as u64,
            memory: gpu::Memory::Upload,
        });

        // Initial bunny
        let mut pending_bunnies = Vec::with_capacity(MAX_BUNNIES);
        pending_bunnies.push(InstanceData {
            position: [-100.0, 100.0],
            velocity: [10.0, 0.0],
            color: 0xFFFFFFFF,
            pad: 0,
        });

        let mut command_encoder = context.create_command_encoder(gpu::CommandEncoderDesc {
            name: "main",
            buffer_count: 2,
        });
        command_encoder.start();
        command_encoder.init_texture(texture);
        if let mut transfer = command_encoder.transfer("init texture") {
            transfer.copy_buffer_to_texture(upload_buffer.into(), 4, texture.into(), extent);
        }
        let sync_point = context.submit(&mut command_encoder);
        context.wait_for(&sync_point, !0);

        context.destroy_buffer(upload_buffer);

        Self {
            render_pipeline,
            compute_pipeline,
            command_encoder,
            prev_sync_point: None,
            texture,
            view,
            sampler,
            vertex_buf,
            instance_buf,
            staging_buf,
            window_size,
            bunny_count: 0, // Will be set when pending_bunnies are uploaded
            pending_bunnies,
            rng: nanorand::WyRand::new_seed(73),
            surface,
            context,
        }
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.window_size = size;
        let config = Self::make_surface_config(size);
        self.context.reconfigure_surface(&mut self.surface, config);
    }

    fn increase(&mut self) {
        use nanorand::Rng as _;
        let total_bunnies = self.bunny_count + self.pending_bunnies.len();
        let spawn_count = (64 + total_bunnies / 2).min(MAX_BUNNIES - total_bunnies);
        for _ in 0..spawn_count {
            let speed = self.rng.generate_range(-MAX_VELOCITY..=MAX_VELOCITY) as f32;
            self.pending_bunnies.push(InstanceData {
                position: [0.0, 0.5 * (self.window_size.height as f32)],
                velocity: [speed, 0.0],
                color: self.rng.generate::<u32>(),
                pad: 0,
            });
        }
        println!("Population: {} bunnies", total_bunnies + spawn_count);
    }

    fn render(&mut self, delta: f32) {
        if self.window_size == Default::default() {
            return;
        }
        let frame = self.surface.acquire_frame();

        self.command_encoder.start();
        self.command_encoder.init_texture(frame.texture());

        // Upload any pending bunnies to GPU via staging buffer
        if !self.pending_bunnies.is_empty() {
            let upload_count = self.pending_bunnies.len();
            let byte_offset = (self.bunny_count * mem::size_of::<InstanceData>()) as u64;
            let byte_size = (upload_count * mem::size_of::<InstanceData>()) as u64;

            // Copy to staging buffer
            unsafe {
                ptr::copy_nonoverlapping(
                    self.pending_bunnies.as_ptr(),
                    self.staging_buf.data() as *mut InstanceData,
                    upload_count,
                );
            }
            self.context.sync_buffer(self.staging_buf);

            // Copy from staging to instance buffer on GPU
            if let mut transfer = self.command_encoder.transfer("upload bunnies") {
                transfer.copy_buffer_to_buffer(
                    self.staging_buf.at(0),
                    self.instance_buf.at(byte_offset),
                    byte_size,
                );
            }

            self.bunny_count += upload_count;
            self.pending_bunnies.clear();
        }

        // Run physics simulation on GPU (compute shader)
        if self.bunny_count > 0 {
            if let mut compute = self.command_encoder.compute("physics") {
                if let mut pc = compute.with(&self.compute_pipeline) {
                    pc.bind(
                        0,
                        &ComputeParams {
                            sim_params: SimParams {
                                delta_time: delta,
                                gravity: GRAVITY,
                                bounds_width: self.window_size.width as f32,
                                bounds_height: self.window_size.height as f32,
                                sprite_half_size: 0.5 * BUNNY_SIZE,
                                bunny_count: self.bunny_count as u32,
                                _pad: [0.0; 2],
                            },
                            instances_rw: self.instance_buf.into(),
                        },
                    );
                    let workgroups = (self.bunny_count as u32 + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
                    pc.dispatch([workgroups, 1, 1]);
                }
            }
        }

        // Render pass
        if let mut pass = self.command_encoder.render(
            "main",
            gpu::RenderTargetSet {
                colors: &[gpu::RenderTarget {
                    view: frame.texture_view(),
                    init_op: gpu::InitOp::Clear(gpu::TextureColor::OpaqueBlack),
                    finish_op: gpu::FinishOp::Store,
                }],
                depth_stencil: None,
            },
        ) {
            let mut rc = pass.with(&self.render_pipeline);

            // Group 0: Static resources (cached - bind group reused across frames)
            rc.bind(
                0,
                &StaticParams {
                    sprite_texture: self.view,
                    sprite_sampler: self.sampler,
                    instances: self.instance_buf.into(),
                },
            );

            // Group 1: Per-frame uniform (recreated each frame - cheap, only uniform data)
            rc.bind(
                1,
                &FrameParams {
                    globals: Globals {
                        mvp_transform: [
                            [2.0 / self.window_size.width as f32, 0.0, 0.0, 0.0],
                            [0.0, 2.0 / self.window_size.height as f32, 0.0, 0.0],
                            [0.0, 0.0, 1.0, 0.0],
                            [-1.0, -1.0, 0.0, 1.0],
                        ],
                        sprite_size: [BUNNY_SIZE; 2],
                        pad: [0.0; 2],
                    },
                },
            );

            // Bind vertex buffer (quad corners)
            rc.bind_vertex(0, self.vertex_buf.into());
            // Single instanced draw: 4 vertices, bunny_count instances
            rc.draw(0, 4, 0, self.bunny_count as u32);
        }
        self.command_encoder.present(frame);
        let sync_point = self.context.submit(&mut self.command_encoder);
        if let Some(sp) = self.prev_sync_point.take() {
            self.context.wait_for(&sp, !0);
        }
        self.prev_sync_point = Some(sync_point);
    }

    fn deinit(&mut self) {
        if let Some(sp) = self.prev_sync_point.take() {
            self.context.wait_for(&sp, !0);
        }
        self.context.destroy_texture_view(self.view);
        self.context.destroy_texture(self.texture);
        self.context.destroy_sampler(self.sampler);
        self.context.destroy_buffer(self.vertex_buf);
        self.context.destroy_buffer(self.instance_buf);
        self.context.destroy_buffer(self.staging_buf);
        self.context
            .destroy_command_encoder(&mut self.command_encoder);
        self.context
            .destroy_render_pipeline(&mut self.render_pipeline);
        self.context
            .destroy_compute_pipeline(&mut self.compute_pipeline);
        self.context.destroy_surface(&mut self.surface);
    }
}

/// Parse initial bunny count from CLI args or environment
#[cfg(not(target_arch = "wasm32"))]
fn parse_bunny_count() -> usize {
    // Check --bunny-count=N or --bunny-count N
    let args: Vec<String> = std::env::args().collect();
    for i in 0..args.len() {
        if args[i].starts_with("--bunny-count=") {
            if let Ok(n) = args[i].trim_start_matches("--bunny-count=").parse() {
                return n;
            }
        } else if args[i] == "--bunny-count" && i + 1 < args.len() {
            if let Ok(n) = args[i + 1].parse() {
                return n;
            }
        }
    }
    // Check BUNNY_COUNT env var
    if let Ok(val) = std::env::var("BUNNY_COUNT") {
        if let Ok(n) = val.parse() {
            return n;
        }
    }
    // Default: 1 bunny (original behavior)
    1
}

/// Main for native and GLES WASM (sync init)
#[cfg(not(all(target_arch = "wasm32", blade_wgpu)))]
fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    #[cfg(not(target_arch = "wasm32"))]
    let initial_bunny_count = parse_bunny_count();

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let window_attributes =
        winit::window::Window::default_attributes().with_title("blade-bunnymark");

    let window = event_loop.create_window(window_attributes).unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::WindowExtWebSys as _;

        console_error_panic_hook::set_once();
        console_log::init().expect("could not initialize logger");
        // On wasm, append the canvas to the document body
        let canvas = window.canvas().unwrap();
        canvas.set_id(gpu::CANVAS_ID);
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| body.append_child(&web_sys::Element::from(canvas)).ok())
            .expect("couldn't append canvas to document body");
    }

    let mut example = Example::new(&window);

    // Spawn initial bunnies based on CLI arg or env var
    #[cfg(not(target_arch = "wasm32"))]
    {
        use nanorand::Rng as _;
        let spawn_count = initial_bunny_count.min(MAX_BUNNIES);
        for _ in 0..spawn_count {
            let speed = example.rng.generate_range(-MAX_VELOCITY..=MAX_VELOCITY) as f32;
            example.pending_bunnies.push(InstanceData {
                position: [0.0, 0.5 * (example.window_size.height as f32)],
                velocity: [speed, 0.0],
                color: example.rng.generate::<u32>(),
                pad: 0,
            });
        }
        if spawn_count > 1 {
            println!("Starting with {} bunnies", spawn_count);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    let mut last_snapshot = std::time::Instant::now();
    #[cfg(target_arch = "wasm32")]
    let perf = web_sys::window().unwrap().performance().unwrap();
    #[cfg(target_arch = "wasm32")]
    let mut last_time = perf.now();
    #[cfg(target_arch = "wasm32")]
    {
        example.increase();
        example.increase();
        log::info!("GLES bunnymark initialized (gles backend)");
    }
    let mut frame_count = 0u32;

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
                    #[cfg(not(target_arch = "wasm32"))]
                    winit::event::WindowEvent::KeyboardInput {
                        event:
                            winit::event::KeyEvent {
                                physical_key: winit::keyboard::PhysicalKey::Code(key_code),
                                state: winit::event::ElementState::Pressed,
                                ..
                            },
                        ..
                    } => match key_code {
                        winit::keyboard::KeyCode::Escape => {
                            target.exit();
                        }
                        winit::keyboard::KeyCode::Space => {
                            example.increase();
                        }
                        _ => {}
                    },
                    winit::event::WindowEvent::CloseRequested => {
                        target.exit();
                    }
                    winit::event::WindowEvent::RedrawRequested => {
                        frame_count += 1;
                        #[cfg(not(target_arch = "wasm32"))]
                        if frame_count == 100 {
                            let accum_time = last_snapshot.elapsed().as_secs_f32();
                            println!(
                                "Avg frame time {}ms",
                                accum_time * 1000.0 / frame_count as f32
                            );
                            last_snapshot = std::time::Instant::now();
                            frame_count = 0;
                        }
                        #[cfg(target_arch = "wasm32")]
                        if frame_count % 100 == 0 {
                            let now = perf.now();
                            let elapsed = now - last_time;
                            let avg_frame_ms = elapsed / 100.0;
                            let fps = 1000.0 / avg_frame_ms;
                            log::info!(
                                "Frame {}: avg {:.2}ms ({:.1} FPS), {} bunnies",
                                frame_count, avg_frame_ms, fps, example.bunny_count
                            );
                            last_time = now;
                        }
                        example.render(0.01);
                    }
                    _ => {}
                },
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
    let window_attributes =
        winit::window::Window::default_attributes().with_title("blade-bunnymark");
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
            // WASM: Use Wait to properly sync with requestAnimationFrame
            // Poll causes RAF spam (34 calls/frame instead of 1)
            target.set_control_flow(winit::event_loop::ControlFlow::Wait);
            match event {
                winit::event::Event::AboutToWait => {
                    // Start async init on first frame
                    if !*init_started_clone.borrow() {
                        *init_started_clone.borrow_mut() = true;
                        let example_init = example_clone.clone();
                        let window_init = window_clone.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            let mut ex = Example::new_async(&window_init).await;
                            // Pre-populate with bunnies (~1000+)
                            for _ in 0..6 {  // ~1331 bunnies
                                ex.increase();
                            }
                            *example_init.borrow_mut() = Some(ex);
                            log::info!("WebGPU bunnymark initialized (blade_wgpu backend)");
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

                            ex.render(0.01);

                            // Log performance every 100 frames
                            if count % 100 == 0 {
                                let now = perf.now();
                                let elapsed = now - *last_time.borrow();
                                let avg_frame_ms = elapsed / 100.0;
                                let fps = 1000.0 / avg_frame_ms;

                                // Get GPU timing results
                                let timing = ex.context.timing_results();
                                let gpu_time_str = if timing.is_empty() {
                                    "N/A".to_string()
                                } else {
                                    timing.iter()
                                        .map(|(name, dur)| format!("{}:{:.2}ms", name, dur.as_secs_f64() * 1000.0))
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                };

                                // Get cache stats
                                let (hits, misses, size) = ex.context.cache_stats();
                                let hit_rate = if hits + misses > 0 {
                                    (hits as f64 / (hits + misses) as f64) * 100.0
                                } else {
                                    0.0
                                };

                                log::info!(
                                    "Frame {}: avg {:.2}ms ({:.1} FPS), {} bunnies | GPU: [{}] | Cache: {:.1}% ({}/{}), size={}",
                                    count, avg_frame_ms, fps, ex.bunny_count,
                                    gpu_time_str, hit_rate, hits, misses, size
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
