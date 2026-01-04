#![allow(irrefutable_let_patterns)]

use blade_graphics as gpu;
use bytemuck::{Pod, Zeroable};
use std::{mem, ptr};

const BUNNY_SIZE: f32 = 0.15 * 256.0;
const GRAVITY: f32 = -9.8 * 100.0;
const MAX_VELOCITY: i32 = 750;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    mvp_transform: [[f32; 4]; 4],
    sprite_size: [f32; 2],
    pad: [f32; 2],
}

#[derive(blade_macros::ShaderData)]
struct Params {
    globals: Globals,
    sprite_texture: gpu::TextureView,
    sprite_sampler: gpu::Sampler,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Locals {
    position: [f32; 2],
    velocity: [f32; 2],
    color: u32,
    pad: u32,
}

#[derive(blade_macros::ShaderData)]
struct SpriteData {
    locals: Locals,
}
#[derive(blade_macros::Vertex)]
struct SpriteVertex {
    pos: [f32; 2],
}
struct Sprite {
    data: SpriteData,
    vertex_buf: gpu::BufferPiece,
}

struct Example {
    pipeline: gpu::RenderPipeline,
    command_encoder: gpu::CommandEncoder,
    prev_sync_point: Option<gpu::SyncPoint>,
    texture: gpu::Texture,
    view: gpu::TextureView,
    sampler: gpu::Sampler,
    vertex_buf: gpu::Buffer,
    window_size: winit::dpi::PhysicalSize<u32>,
    bunnies: Vec<Sprite>,
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
            timing: false,
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

        let global_layout = <Params as gpu::ShaderData>::layout();
        let local_layout = <SpriteData as gpu::ShaderData>::layout();
        #[cfg(target_arch = "wasm32")]
        let shader_source = include_str!("shader.wgsl");
        #[cfg(not(target_arch = "wasm32"))]
        let shader_source = std::fs::read_to_string("examples/bunnymark/shader.wgsl").unwrap();
        let shader = context.create_shader(gpu::ShaderDesc {
            source: &shader_source,
        });

        let pipeline = context.create_render_pipeline(gpu::RenderPipelineDesc {
            name: "main",
            data_layouts: &[&global_layout, &local_layout],
            vertex: shader.at("vs_main"),
            vertex_fetches: &[gpu::VertexFetchState {
                layout: &<SpriteVertex as gpu::Vertex>::layout(),
                instanced: false,
            }],
            primitive: gpu::PrimitiveState {
                topology: gpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            fragment: Some(shader.at("fs_main")),
            color_targets: &[gpu::ColorTargetState {
                format: surface.info().format,
                blend: Some(gpu::BlendState::ALPHA_BLENDING),
                write_mask: gpu::ColorWrites::default(),
            }],
            multisample_state: gpu::MultisampleState::default(),
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

        let mut bunnies = Vec::new();
        bunnies.push(Sprite {
            data: SpriteData {
                locals: Locals {
                    position: [-100.0, 100.0],
                    velocity: [10.0, 0.0],
                    color: 0xFFFFFFFF,
                    pad: 0,
                },
            },
            vertex_buf: vertex_buf.into(),
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
            pipeline,
            command_encoder,
            prev_sync_point: None,
            texture,
            view,
            sampler,
            vertex_buf,
            window_size,
            bunnies,
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
        let spawn_count = 64 + self.bunnies.len() / 2;
        for _ in 0..spawn_count {
            let speed = self.rng.generate_range(-MAX_VELOCITY..=MAX_VELOCITY) as f32;
            self.bunnies.push(Sprite {
                data: SpriteData {
                    locals: Locals {
                        position: [0.0, 0.5 * (self.window_size.height as f32)],
                        velocity: [speed, 0.0],
                        color: self.rng.generate::<u32>(),
                        pad: 0,
                    },
                },
                vertex_buf: self.vertex_buf.into(),
            });
        }
        println!("Population: {} bunnies", self.bunnies.len());
    }

    fn step(&mut self, delta: f32) {
        for bunny in self.bunnies.iter_mut() {
            let Sprite {
                data:
                    SpriteData {
                        locals:
                            Locals {
                                position: ref mut pos,
                                velocity: ref mut vel,
                                ..
                            },
                    },
                ..
            } = *bunny;

            pos[0] += vel[0] * delta;
            pos[1] += vel[1] * delta;
            vel[1] += GRAVITY * delta;
            if (vel[0] > 0.0 && pos[0] + 0.5 * BUNNY_SIZE > self.window_size.width as f32)
                || (vel[0] < 0.0 && pos[0] - 0.5 * BUNNY_SIZE < 0.0)
            {
                vel[0] *= -1.0;
            }
            if vel[1] < 0.0 && pos[1] < 0.5 * BUNNY_SIZE {
                vel[1] *= -1.0;
            }
        }
    }

    fn render(&mut self) {
        if self.window_size == Default::default() {
            return;
        }
        let frame = self.surface.acquire_frame();

        self.command_encoder.start();
        self.command_encoder.init_texture(frame.texture());

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
            let mut rc = pass.with(&self.pipeline);
            rc.bind(
                0,
                &Params {
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
                    sprite_texture: self.view,
                    sprite_sampler: self.sampler,
                },
            );

            for sprite in self.bunnies.iter() {
                //Note: technically, we could get away with either of those bindings
                // but not them together. However, the purpose of this test is to
                // mimic a real world draw call, not a super optimized ideal.
                rc.bind(1, &sprite.data);
                rc.bind_vertex(0, sprite.vertex_buf);
                rc.draw(0, 4, 0, 1);
            }
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
        self.context
            .destroy_command_encoder(&mut self.command_encoder);
        self.context.destroy_render_pipeline(&mut self.pipeline);
        self.context.destroy_surface(&mut self.surface);
    }
}

/// Main for native and GLES WASM (sync init)
#[cfg(not(all(target_arch = "wasm32", blade_wgpu)))]
fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

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
                                frame_count, avg_frame_ms, fps, example.bunnies.len()
                            );
                            last_time = now;
                        }
                        example.step(0.01);
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
            target.set_control_flow(winit::event_loop::ControlFlow::Poll);
            match event {
                winit::event::Event::AboutToWait => {
                    // Start async init on first frame
                    if !*init_started_clone.borrow() {
                        *init_started_clone.borrow_mut() = true;
                        let example_init = example_clone.clone();
                        let window_init = window_clone.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            let mut ex = Example::new_async(&window_init).await;
                            // Pre-populate with bunnies
                            ex.increase();
                            ex.increase();
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

                            ex.step(0.01);
                            ex.render();

                            // Log performance every 100 frames
                            if count % 100 == 0 {
                                let now = perf.now();
                                let elapsed = now - *last_time.borrow();
                                let avg_frame_ms = elapsed / 100.0;
                                let fps = 1000.0 / avg_frame_ms;
                                log::info!(
                                    "Frame {}: avg {:.2}ms ({:.1} FPS), {} bunnies",
                                    count, avg_frame_ms, fps, ex.bunnies.len()
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
