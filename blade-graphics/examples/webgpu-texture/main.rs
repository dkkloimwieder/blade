//! WebGPU Texture Example
//!
//! Demonstrates texture creation, upload, and sampling in blade-graphics.
//!
//! Key patterns shown:
//! - Texture creation with format and usage flags
//! - Texture view creation for shader access
//! - Sampler creation with filter modes
//! - Staging buffer for CPU→GPU upload
//! - ShaderData derive macro for bindings
//! - textureSample() in fragment shader
//!
//! Run with: RUSTFLAGS="--cfg blade_wgpu" cargo run -p blade-graphics --example webgpu-texture
//! For WASM: RUSTFLAGS="--cfg blade_wgpu" cargo run-wasm -p blade-graphics --example webgpu-texture

#![allow(irrefutable_let_patterns)]

use blade_graphics as gpu;
use std::ptr;

// -----------------------------------------------------------------------------
// Shader Data Bindings
// -----------------------------------------------------------------------------
// Manual implementation of ShaderData trait.
// This shows what the #[derive(blade_macros::ShaderData)] macro generates.
//
// Bindings are matched by NAME to WGSL variables:
//   sprite_texture -> var sprite_texture: texture_2d<f32>
//   sprite_sampler -> var sprite_sampler: sampler

struct TextureParams {
    /// 2D texture bound as texture_2d<f32> in WGSL
    sprite_texture: gpu::TextureView,
    /// Sampler for filtering and addressing
    sprite_sampler: gpu::Sampler,
}

impl gpu::ShaderData for TextureParams {
    /// Define the layout: binding names and types
    fn layout() -> gpu::ShaderDataLayout {
        gpu::ShaderDataLayout {
            bindings: vec![
                ("sprite_texture", gpu::ShaderBinding::Texture),
                ("sprite_sampler", gpu::ShaderBinding::Sampler),
            ],
        }
    }

    /// Fill bindings with actual resources
    fn fill(&self, mut ctx: gpu::PipelineContext) {
        use gpu::ShaderBindable as _;
        self.sprite_texture.bind_to(&mut ctx, 0);
        self.sprite_sampler.bind_to(&mut ctx, 1);
    }
}

// -----------------------------------------------------------------------------
// Texture Generation
// -----------------------------------------------------------------------------

/// Generate a checkerboard pattern for demonstration
/// Returns RGBA8 data (4 bytes per pixel)
fn generate_checkerboard(width: u32, height: u32, tile_size: u32) -> Vec<u8> {
    let mut data = Vec::with_capacity((width * height * 4) as usize);

    for y in 0..height {
        for x in 0..width {
            // Determine if this pixel is in a "light" or "dark" tile
            let tile_x = x / tile_size;
            let tile_y = y / tile_size;
            let is_light = (tile_x + tile_y) % 2 == 0;

            if is_light {
                // Light tile: warm orange
                data.extend_from_slice(&[255, 180, 100, 255]);
            } else {
                // Dark tile: deep blue
                data.extend_from_slice(&[40, 60, 120, 255]);
            }
        }
    }

    data
}

// -----------------------------------------------------------------------------
// Example Application
// -----------------------------------------------------------------------------

struct Example {
    context: gpu::Context,
    surface: gpu::Surface,
    pipeline: gpu::RenderPipeline,
    command_encoder: gpu::CommandEncoder,
    // Texture resources (must be kept alive while in use)
    texture: gpu::Texture,
    texture_view: gpu::TextureView,
    sampler: gpu::Sampler,
    // Synchronization
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

    // -------------------------------------------------------------------------
    // Native initialization (synchronous)
    // -------------------------------------------------------------------------
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

    // -------------------------------------------------------------------------
    // WASM initialization (asynchronous - required by browser WebGPU)
    // -------------------------------------------------------------------------
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
        // Load shader (embedded for WASM, filesystem for native)
        // ---------------------------------------------------------------------
        #[cfg(target_arch = "wasm32")]
        let shader_source = include_str!("shader.wgsl");
        #[cfg(not(target_arch = "wasm32"))]
        let shader_source =
            std::fs::read_to_string("blade-graphics/examples/webgpu-texture/shader.wgsl").unwrap();

        let shader = context.create_shader(gpu::ShaderDesc {
            source: &shader_source,
        });

        // ---------------------------------------------------------------------
        // Create texture (GPU resource)
        // ---------------------------------------------------------------------
        let tex_width = 256;
        let tex_height = 256;
        let extent = gpu::Extent {
            width: tex_width,
            height: tex_height,
            depth: 1,
        };

        let texture = context.create_texture(gpu::TextureDesc {
            name: "checkerboard",
            format: gpu::TextureFormat::Rgba8Unorm,
            size: extent,
            dimension: gpu::TextureDimension::D2,
            array_layer_count: 1,
            mip_level_count: 1,
            // RESOURCE: can be sampled in shaders
            // COPY: can be copy destination (for upload)
            usage: gpu::TextureUsage::RESOURCE | gpu::TextureUsage::COPY,
            sample_count: 1,
            external: None,
        });

        // ---------------------------------------------------------------------
        // Create texture view (shader-visible handle to texture)
        // ---------------------------------------------------------------------
        let texture_view = context.create_texture_view(
            texture,
            gpu::TextureViewDesc {
                name: "checkerboard_view",
                format: gpu::TextureFormat::Rgba8Unorm,
                dimension: gpu::ViewDimension::D2,
                subresources: &Default::default(),
            },
        );

        // ---------------------------------------------------------------------
        // Create sampler (controls filtering and addressing)
        // ---------------------------------------------------------------------
        let sampler = context.create_sampler(gpu::SamplerDesc {
            name: "linear_sampler",
            // Linear filtering for smooth scaling
            mag_filter: gpu::FilterMode::Linear,
            min_filter: gpu::FilterMode::Linear,
            // Repeat addressing for tiling (u, v, w)
            address_modes: [
                gpu::AddressMode::Repeat,
                gpu::AddressMode::Repeat,
                gpu::AddressMode::Repeat,
            ],
            ..Default::default()
        });

        // ---------------------------------------------------------------------
        // Upload texture data via staging buffer
        // ---------------------------------------------------------------------
        let texture_data = generate_checkerboard(tex_width, tex_height, 32);
        let bytes_per_row = tex_width * 4; // RGBA8 = 4 bytes per pixel

        // Create staging buffer (CPU-visible for upload)
        let upload_buffer = context.create_buffer(gpu::BufferDesc {
            name: "texture_staging",
            size: texture_data.len() as u64,
            memory: gpu::Memory::Upload,
        });

        // Copy data to staging buffer
        unsafe {
            ptr::copy_nonoverlapping(
                texture_data.as_ptr(),
                upload_buffer.data(),
                texture_data.len(),
            );
        }
        // Mark buffer as modified (triggers sync to GPU)
        context.sync_buffer(upload_buffer);

        // ---------------------------------------------------------------------
        // Create render pipeline with texture bindings
        // ---------------------------------------------------------------------
        let texture_layout = <TextureParams as gpu::ShaderData>::layout();

        let pipeline = context.create_render_pipeline(gpu::RenderPipelineDesc {
            name: "textured_quad",
            data_layouts: &[&texture_layout],
            vertex: shader.at("vs_main"),
            vertex_fetches: &[], // No vertex buffers (positions in shader)
            fragment: Some(shader.at("fs_main")),
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
        // Upload texture via transfer pass
        // ---------------------------------------------------------------------
        let mut command_encoder = context.create_command_encoder(gpu::CommandEncoderDesc {
            name: "main",
            buffer_count: 2,
        });

        command_encoder.start();
        // Initialize texture layout (required before first use)
        command_encoder.init_texture(texture);

        // Transfer pass: copy staging buffer to texture
        if let mut transfer = command_encoder.transfer("upload_texture") {
            transfer.copy_buffer_to_texture(upload_buffer.into(), bytes_per_row, texture.into(), extent);
        }

        // Submit and wait for upload to complete
        let sync_point = context.submit(&mut command_encoder);
        context.wait_for(&sync_point, !0);

        // Clean up staging buffer (no longer needed)
        context.destroy_buffer(upload_buffer);

        Self {
            context,
            surface,
            pipeline,
            command_encoder,
            texture,
            texture_view,
            sampler,
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
        // Wait for previous frame to complete
        if let Some(ref sp) = self.prev_sync_point {
            self.context.wait_for(sp, !0);
        }

        // Acquire swapchain frame
        let frame = self.surface.acquire_frame();

        // Start recording commands
        self.command_encoder.start();

        // Render pass: draw textured quad
        if let mut pass = self.command_encoder.render(
            "textured_quad",
            gpu::RenderTargetSet {
                colors: &[gpu::RenderTarget {
                    view: frame.texture_view(),
                    init_op: gpu::InitOp::Clear(gpu::TextureColor::OpaqueBlack),
                    finish_op: gpu::FinishOp::Store,
                }],
                depth_stencil: None,
            },
        ) {
            if let mut encoder = pass.with(&self.pipeline) {
                // Bind texture and sampler (group 0)
                encoder.bind(
                    0,
                    &TextureParams {
                        sprite_texture: self.texture_view,
                        sprite_sampler: self.sampler,
                    },
                );
                // Draw fullscreen quad (6 vertices for 2 triangles)
                encoder.draw(0, 6, 0, 1);
            }
        }

        // Present and submit
        self.command_encoder.present(frame);
        self.prev_sync_point = Some(self.context.submit(&mut self.command_encoder));
    }

    fn deinit(mut self) {
        // Wait for GPU to finish
        if let Some(ref sp) = self.prev_sync_point {
            self.context.wait_for(sp, !0);
        }
        // Destroy resources in reverse order
        self.context.destroy_texture_view(self.texture_view);
        self.context.destroy_texture(self.texture);
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
        winit::window::Window::default_attributes().with_title("blade-webgpu-texture");
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
        winit::window::Window::default_attributes().with_title("blade-webgpu-texture");
    let window = Rc::new(event_loop.create_window(window_attributes).unwrap());

    // Set up canvas
    let canvas = window.canvas().unwrap();
    canvas.set_id(gpu::CANVAS_ID);
    web_sys::window()
        .and_then(|win| win.document())
        .and_then(|doc| doc.body())
        .and_then(|body| body.append_child(&web_sys::Element::from(canvas)).ok())
        .expect("couldn't append canvas to document body");

    // State machine for async initialization
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
                    // Start async init on first frame
                    if !*init_started_clone.borrow() {
                        *init_started_clone.borrow_mut() = true;
                        let example_init = example_clone.clone();
                        let window_init = window_clone.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            let ex = Example::new_async(&window_init).await;
                            *example_init.borrow_mut() = Some(ex);
                            log::info!("WebGPU texture example initialized!");
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
