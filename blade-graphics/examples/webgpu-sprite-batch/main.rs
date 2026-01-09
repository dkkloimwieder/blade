//! WebGPU Sprite Batch Example
//!
//! Demonstrates efficient 2D sprite rendering using instancing.
//!
//! Key patterns shown:
//! - Storage buffer for per-sprite data
//! - Vertex buffer for quad geometry
//! - Instanced draw calls
//! - Screen-space 2D coordinate system
//!
//! Run with: RUSTFLAGS="--cfg blade_wgpu" cargo run -p blade-graphics --example webgpu-sprite-batch
//! For WASM: RUSTFLAGS="--cfg blade_wgpu" cargo run-wasm -p blade-graphics --example webgpu-sprite-batch

#![allow(irrefutable_let_patterns)]

use blade_graphics as gpu;
use bytemuck::{Pod, Zeroable};
use std::{mem, ptr};

// -----------------------------------------------------------------------------
// Data Structures
// -----------------------------------------------------------------------------

/// Global uniforms
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    screen_size: [f32; 2],
    time: f32,
    sprite_count: u32,
}

/// Per-vertex data (quad corners)
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 2],
}

/// Per-sprite data (stored in storage buffer)
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SpriteData {
    position: [f32; 2],
    size: [f32; 2],
    rotation: f32,
    color: u32,
    _pad: [f32; 2],
}

impl SpriteData {
    fn new(x: f32, y: f32, w: f32, h: f32, rotation: f32, r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            position: [x, y],
            size: [w, h],
            rotation,
            color: (a as u32) << 24 | (b as u32) << 16 | (g as u32) << 8 | (r as u32),
            _pad: [0.0; 2],
        }
    }
}

// Implement Vertex trait manually
impl gpu::Vertex for Vertex {
    fn layout() -> gpu::VertexLayout {
        gpu::VertexLayout {
            attributes: vec![
                ("pos", gpu::VertexAttribute { offset: 0, format: gpu::VertexFormat::F32Vec2 }),
            ],
            stride: mem::size_of::<Self>() as u32,
        }
    }
}

// -----------------------------------------------------------------------------
// Shader Data
// -----------------------------------------------------------------------------

struct RenderParams {
    globals: Globals,
    sprites: gpu::BufferPiece,
}

impl gpu::ShaderData for RenderParams {
    fn layout() -> gpu::ShaderDataLayout {
        gpu::ShaderDataLayout {
            bindings: vec![
                ("globals", gpu::ShaderBinding::Plain { size: 16 }),
                ("sprites", gpu::ShaderBinding::Buffer),
            ],
        }
    }

    fn fill(&self, mut ctx: gpu::PipelineContext) {
        use gpu::ShaderBindable as _;
        self.globals.bind_to(&mut ctx, 0);
        self.sprites.bind_to(&mut ctx, 1);
    }
}

// -----------------------------------------------------------------------------
// Example
// -----------------------------------------------------------------------------

const NUM_SPRITES: usize = 100;

struct Example {
    context: gpu::Context,
    surface: gpu::Surface,
    pipeline: gpu::RenderPipeline,
    command_encoder: gpu::CommandEncoder,
    vertex_buffer: gpu::Buffer,
    sprite_buffer: gpu::Buffer,
    // Animation state
    #[cfg(not(target_arch = "wasm32"))]
    start_time: std::time::Instant,
    #[cfg(target_arch = "wasm32")]
    frame_count: u32,
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

        // Load shader
        #[cfg(target_arch = "wasm32")]
        let shader_source = include_str!("shader.wgsl");
        #[cfg(not(target_arch = "wasm32"))]
        let shader_source =
            std::fs::read_to_string("blade-graphics/examples/webgpu-sprite-batch/shader.wgsl")
                .unwrap();

        let shader = context.create_shader(gpu::ShaderDesc {
            source: &shader_source,
        });

        // Create vertex buffer (quad: 6 vertices for 2 triangles)
        let vertices = [
            Vertex { pos: [-0.5, -0.5] }, // bottom-left
            Vertex { pos: [ 0.5, -0.5] }, // bottom-right
            Vertex { pos: [-0.5,  0.5] }, // top-left
            Vertex { pos: [-0.5,  0.5] }, // top-left
            Vertex { pos: [ 0.5, -0.5] }, // bottom-right
            Vertex { pos: [ 0.5,  0.5] }, // top-right
        ];

        let vertex_buffer = context.create_buffer(gpu::BufferDesc {
            name: "vertices",
            size: mem::size_of_val(&vertices) as u64,
            memory: gpu::Memory::Shared,
        });
        unsafe {
            ptr::copy_nonoverlapping(
                vertices.as_ptr(),
                vertex_buffer.data() as *mut Vertex,
                vertices.len(),
            );
        }
        context.sync_buffer(vertex_buffer);

        // Create sprite buffer (storage buffer for per-sprite data)
        let sprite_buffer = context.create_buffer(gpu::BufferDesc {
            name: "sprites",
            size: (mem::size_of::<SpriteData>() * NUM_SPRITES) as u64,
            memory: gpu::Memory::Shared,
        });

        // Create pipeline
        let render_layout = <RenderParams as gpu::ShaderData>::layout();
        let pipeline = context.create_render_pipeline(gpu::RenderPipelineDesc {
            name: "sprite",
            data_layouts: &[&render_layout],
            vertex: shader.at("vs_main"),
            vertex_fetches: &[gpu::VertexFetchState {
                layout: &<Vertex as gpu::Vertex>::layout(),
                instanced: false,
            }],
            fragment: Some(shader.at("fs_main")),
            primitive: gpu::PrimitiveState {
                topology: gpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            color_targets: &[gpu::ColorTargetState {
                format: surface.info().format,
                blend: Some(gpu::BlendState::ALPHA_BLENDING),
                write_mask: gpu::ColorWrites::ALL,
            }],
            multisample_state: gpu::MultisampleState::default(),
        });

        let command_encoder = context.create_command_encoder(gpu::CommandEncoderDesc {
            name: "main",
            buffer_count: 2,
        });

        Self {
            context,
            surface,
            pipeline,
            command_encoder,
            vertex_buffer,
            sprite_buffer,
            #[cfg(not(target_arch = "wasm32"))]
            start_time: std::time::Instant::now(),
            #[cfg(target_arch = "wasm32")]
            frame_count: 0,
            prev_sync_point: None,
            window_size,
        }
    }

    fn create_sprites(size: winit::dpi::PhysicalSize<u32>, time: f32) -> Vec<SpriteData> {
        let mut sprites = Vec::with_capacity(NUM_SPRITES);
        let w = size.width as f32;
        let h = size.height as f32;

        for i in 0..NUM_SPRITES {
            let t = i as f32 / NUM_SPRITES as f32;
            let angle = t * std::f32::consts::TAU * 3.0 + time;

            // Spiral pattern
            let radius = 50.0 + t * (w.min(h) * 0.35);
            let x = w / 2.0 + angle.cos() * radius;
            let y = h / 2.0 + angle.sin() * radius;

            // Size varies
            let sprite_size = 20.0 + t * 30.0;

            // Rotation based on position in spiral
            let rotation = angle + time * 2.0;

            // Rainbow colors
            let hue = (t + time * 0.1) % 1.0;
            let (r, g, b) = hsv_to_rgb(hue, 0.8, 1.0);

            sprites.push(SpriteData::new(
                x, y,
                sprite_size, sprite_size,
                rotation,
                (r * 255.0) as u8,
                (g * 255.0) as u8,
                (b * 255.0) as u8,
                220,
            ));
        }

        sprites
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
        if let Some(ref sp) = self.prev_sync_point {
            self.context.wait_for(sp, !0);
        }

        // Calculate time
        #[cfg(not(target_arch = "wasm32"))]
        let time = self.start_time.elapsed().as_secs_f32();
        #[cfg(target_arch = "wasm32")]
        let time = {
            self.frame_count += 1;
            self.frame_count as f32 / 60.0
        };

        // Update sprites
        let sprites = Self::create_sprites(self.window_size, time);

        // Upload sprite data
        unsafe {
            ptr::copy_nonoverlapping(
                sprites.as_ptr(),
                self.sprite_buffer.data() as *mut SpriteData,
                sprites.len(),
            );
        }
        self.context.sync_buffer(self.sprite_buffer);

        let globals = Globals {
            screen_size: [self.window_size.width as f32, self.window_size.height as f32],
            time,
            sprite_count: NUM_SPRITES as u32,
        };

        let frame = self.surface.acquire_frame();
        self.command_encoder.start();

        if let mut pass = self.command_encoder.render(
            "sprites",
            gpu::RenderTargetSet {
                colors: &[gpu::RenderTarget {
                    view: frame.texture_view(),
                    init_op: gpu::InitOp::Clear(gpu::TextureColor::OpaqueBlack),
                    finish_op: gpu::FinishOp::Store,
                }],
                depth_stencil: None,
            },
        ) {
            if let mut rc = pass.with(&self.pipeline) {
                rc.bind(0, &RenderParams {
                    globals,
                    sprites: self.sprite_buffer.into(),
                });
                rc.bind_vertex(0, self.vertex_buffer.into());
                // Draw 6 vertices (quad), NUM_SPRITES instances
                rc.draw(0, 6, 0, NUM_SPRITES as u32);
            }
        }

        self.command_encoder.present(frame);
        self.prev_sync_point = Some(self.context.submit(&mut self.command_encoder));
    }

    #[allow(dead_code)]
    fn deinit(mut self) {
        if let Some(ref sp) = self.prev_sync_point {
            self.context.wait_for(sp, !0);
        }
        self.context.destroy_buffer(self.vertex_buffer);
        self.context.destroy_buffer(self.sprite_buffer);
        self.context.destroy_command_encoder(&mut self.command_encoder);
        self.context.destroy_surface(&mut self.surface);
    }
}

// HSV to RGB conversion
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let c = v * s;
    let hp = h * 6.0;
    let x = c * (1.0 - ((hp % 2.0) - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if hp < 1.0 {
        (c, x, 0.0)
    } else if hp < 2.0 {
        (x, c, 0.0)
    } else if hp < 3.0 {
        (0.0, c, x)
    } else if hp < 4.0 {
        (0.0, x, c)
    } else if hp < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (r + m, g + m, b + m)
}

// -----------------------------------------------------------------------------
// Native main()
// -----------------------------------------------------------------------------
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    env_logger::init();

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let window_attributes =
        winit::window::Window::default_attributes().with_title("blade-webgpu-sprite-batch");
    #[allow(deprecated)]
    let window = event_loop.create_window(window_attributes).unwrap();

    let mut example = Example::new(&window);

    #[allow(deprecated)]
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
        winit::window::Window::default_attributes().with_title("blade-webgpu-sprite-batch");
    #[allow(deprecated)]
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

    #[allow(deprecated)]
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
                            log::info!("Sprite batch initialized!");
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
