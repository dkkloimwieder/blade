//! WebGPU Post-Processing Effects Example
//!
//! Demonstrates render-to-texture and post-processing pipeline.
//!
//! Key patterns shown:
//! - Offscreen render target (render to texture)
//! - Multi-pass rendering (scene pass → post-fx pass)
//! - Uniform buffer for time-based animation
//! - Post-processing effects (vignette, chromatic aberration)
//!
//! Run with: RUSTFLAGS="--cfg blade_wgpu" cargo run -p blade-graphics --example webgpu-post-fx
//! For WASM: RUSTFLAGS="--cfg blade_wgpu" cargo run-wasm -p blade-graphics --example webgpu-post-fx

#![allow(irrefutable_let_patterns)]

use blade_graphics as gpu;
use bytemuck::{Pod, Zeroable};
use std::time::Instant;

// -----------------------------------------------------------------------------
// Uniform Data
// -----------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    time: f32,
    _pad: [f32; 3],
}

// -----------------------------------------------------------------------------
// Shader Data Bindings
// -----------------------------------------------------------------------------

/// Scene shader bindings (just uniforms)
struct SceneParams {
    uniforms: Uniforms,
}

impl gpu::ShaderData for SceneParams {
    fn layout() -> gpu::ShaderDataLayout {
        gpu::ShaderDataLayout {
            bindings: vec![("uniforms", gpu::ShaderBinding::Plain { size: 16 })],
        }
    }

    fn fill(&self, mut ctx: gpu::PipelineContext) {
        use gpu::ShaderBindable as _;
        self.uniforms.bind_to(&mut ctx, 0);
    }
}

/// Post-fx shader bindings (uniforms + scene texture)
struct PostFxParams {
    uniforms: Uniforms,
    scene_texture: gpu::TextureView,
    scene_sampler: gpu::Sampler,
}

impl gpu::ShaderData for PostFxParams {
    fn layout() -> gpu::ShaderDataLayout {
        gpu::ShaderDataLayout {
            bindings: vec![
                ("uniforms", gpu::ShaderBinding::Plain { size: 16 }),
                ("scene_texture", gpu::ShaderBinding::Texture),
                ("scene_sampler", gpu::ShaderBinding::Sampler),
            ],
        }
    }

    fn fill(&self, mut ctx: gpu::PipelineContext) {
        use gpu::ShaderBindable as _;
        self.uniforms.bind_to(&mut ctx, 0);
        self.scene_texture.bind_to(&mut ctx, 1);
        self.scene_sampler.bind_to(&mut ctx, 2);
    }
}

// -----------------------------------------------------------------------------
// Example Application
// -----------------------------------------------------------------------------

struct Example {
    context: gpu::Context,
    surface: gpu::Surface,
    scene_pipeline: gpu::RenderPipeline,
    postfx_pipeline: gpu::RenderPipeline,
    command_encoder: gpu::CommandEncoder,
    // Offscreen render target
    offscreen_texture: gpu::Texture,
    offscreen_view: gpu::TextureView,
    sampler: gpu::Sampler,
    // Timing
    #[cfg(not(target_arch = "wasm32"))]
    start_time: Instant,
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

        // Load shaders
        #[cfg(target_arch = "wasm32")]
        let scene_source = include_str!("scene.wgsl");
        #[cfg(not(target_arch = "wasm32"))]
        let scene_source =
            std::fs::read_to_string("blade-graphics/examples/webgpu-post-fx/scene.wgsl").unwrap();

        #[cfg(target_arch = "wasm32")]
        let postfx_source = include_str!("postfx.wgsl");
        #[cfg(not(target_arch = "wasm32"))]
        let postfx_source =
            std::fs::read_to_string("blade-graphics/examples/webgpu-post-fx/postfx.wgsl").unwrap();

        let scene_shader = context.create_shader(gpu::ShaderDesc {
            source: &scene_source,
        });
        let postfx_shader = context.create_shader(gpu::ShaderDesc {
            source: &postfx_source,
        });

        // Create offscreen render target
        let offscreen_format = gpu::TextureFormat::Rgba8Unorm;
        let offscreen_extent = gpu::Extent {
            width: window_size.width.max(1),
            height: window_size.height.max(1),
            depth: 1,
        };

        let offscreen_texture = context.create_texture(gpu::TextureDesc {
            name: "offscreen",
            format: offscreen_format,
            size: offscreen_extent,
            dimension: gpu::TextureDimension::D2,
            array_layer_count: 1,
            mip_level_count: 1,
            // TARGET: can be rendered to
            // RESOURCE: can be sampled
            usage: gpu::TextureUsage::TARGET | gpu::TextureUsage::RESOURCE,
            sample_count: 1,
            external: None,
        });

        let offscreen_view = context.create_texture_view(
            offscreen_texture,
            gpu::TextureViewDesc {
                name: "offscreen_view",
                format: offscreen_format,
                dimension: gpu::ViewDimension::D2,
                subresources: &Default::default(),
            },
        );

        let sampler = context.create_sampler(gpu::SamplerDesc {
            name: "linear",
            mag_filter: gpu::FilterMode::Linear,
            min_filter: gpu::FilterMode::Linear,
            ..Default::default()
        });

        // Scene pipeline (renders to offscreen texture)
        let scene_layout = <SceneParams as gpu::ShaderData>::layout();
        let scene_pipeline = context.create_render_pipeline(gpu::RenderPipelineDesc {
            name: "scene",
            data_layouts: &[&scene_layout],
            vertex: scene_shader.at("vs_main"),
            vertex_fetches: &[],
            fragment: Some(scene_shader.at("fs_main")),
            primitive: gpu::PrimitiveState {
                topology: gpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            color_targets: &[gpu::ColorTargetState {
                format: offscreen_format,
                blend: None,
                write_mask: gpu::ColorWrites::ALL,
            }],
            multisample_state: gpu::MultisampleState::default(),
        });

        // Post-fx pipeline (renders to screen)
        let postfx_layout = <PostFxParams as gpu::ShaderData>::layout();
        let postfx_pipeline = context.create_render_pipeline(gpu::RenderPipelineDesc {
            name: "postfx",
            data_layouts: &[&postfx_layout],
            vertex: postfx_shader.at("vs_main"),
            vertex_fetches: &[],
            fragment: Some(postfx_shader.at("fs_main")),
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

        let command_encoder = context.create_command_encoder(gpu::CommandEncoderDesc {
            name: "main",
            buffer_count: 2,
        });

        Self {
            context,
            surface,
            scene_pipeline,
            postfx_pipeline,
            command_encoder,
            offscreen_texture,
            offscreen_view,
            sampler,
            #[cfg(not(target_arch = "wasm32"))]
            start_time: Instant::now(),
            #[cfg(target_arch = "wasm32")]
            frame_count: 0,
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

        // Recreate offscreen texture at new size
        self.context.destroy_texture_view(self.offscreen_view);
        self.context.destroy_texture(self.offscreen_texture);

        let offscreen_format = gpu::TextureFormat::Rgba8Unorm;
        let offscreen_extent = gpu::Extent {
            width: size.width,
            height: size.height,
            depth: 1,
        };

        self.offscreen_texture = self.context.create_texture(gpu::TextureDesc {
            name: "offscreen",
            format: offscreen_format,
            size: offscreen_extent,
            dimension: gpu::TextureDimension::D2,
            array_layer_count: 1,
            mip_level_count: 1,
            usage: gpu::TextureUsage::TARGET | gpu::TextureUsage::RESOURCE,
            sample_count: 1,
            external: None,
        });

        self.offscreen_view = self.context.create_texture_view(
            self.offscreen_texture,
            gpu::TextureViewDesc {
                name: "offscreen_view",
                format: offscreen_format,
                dimension: gpu::ViewDimension::D2,
                subresources: &Default::default(),
            },
        );
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

        let uniforms = Uniforms {
            time,
            _pad: [0.0; 3],
        };

        let frame = self.surface.acquire_frame();
        self.command_encoder.start();

        // Pass 1: Render scene to offscreen texture
        if let mut pass = self.command_encoder.render(
            "scene",
            gpu::RenderTargetSet {
                colors: &[gpu::RenderTarget {
                    view: self.offscreen_view,
                    init_op: gpu::InitOp::Clear(gpu::TextureColor::TransparentBlack),
                    finish_op: gpu::FinishOp::Store,
                }],
                depth_stencil: None,
            },
        ) {
            if let mut rc = pass.with(&self.scene_pipeline) {
                rc.bind(0, &SceneParams { uniforms });
                rc.draw(0, 3, 0, 1);
            }
        }

        // Pass 2: Apply post-fx and render to screen
        if let mut pass = self.command_encoder.render(
            "postfx",
            gpu::RenderTargetSet {
                colors: &[gpu::RenderTarget {
                    view: frame.texture_view(),
                    init_op: gpu::InitOp::Clear(gpu::TextureColor::OpaqueBlack),
                    finish_op: gpu::FinishOp::Store,
                }],
                depth_stencil: None,
            },
        ) {
            if let mut rc = pass.with(&self.postfx_pipeline) {
                rc.bind(
                    0,
                    &PostFxParams {
                        uniforms,
                        scene_texture: self.offscreen_view,
                        scene_sampler: self.sampler,
                    },
                );
                rc.draw(0, 6, 0, 1);
            }
        }

        self.command_encoder.present(frame);
        self.prev_sync_point = Some(self.context.submit(&mut self.command_encoder));
    }

    fn deinit(mut self) {
        if let Some(ref sp) = self.prev_sync_point {
            self.context.wait_for(sp, !0);
        }
        self.context.destroy_texture_view(self.offscreen_view);
        self.context.destroy_texture(self.offscreen_texture);
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
        winit::window::Window::default_attributes().with_title("blade-webgpu-post-fx");
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
        winit::window::Window::default_attributes().with_title("blade-webgpu-post-fx");
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
            target.set_control_flow(winit::event_loop::ControlFlow::Poll);
            match event {
                winit::event::Event::AboutToWait => {
                    if !*init_started_clone.borrow() {
                        *init_started_clone.borrow_mut() = true;
                        let example_init = example_clone.clone();
                        let window_init = window_clone.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            let ex = Example::new_async(&window_init).await;
                            *example_init.borrow_mut() = Some(ex);
                            log::info!("Post-FX initialized!");
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
