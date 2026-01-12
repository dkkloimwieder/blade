//! WebGPU Mandelbrot Fractal Example
//!
//! Demonstrates compute shaders for fractal visualization with interactive controls.
//!
//! Key patterns shown:
//! - Compute shader writing to storage texture
//! - Uniform buffer for fractal parameters
//! - Interactive zoom/pan with mouse
//! - Compute → Render pipeline
//!
//! Controls:
//! - Mouse scroll: Zoom in/out (centered on cursor)
//! - Click and drag: Pan the view
//! - I key: Double iteration count (more detail, slower)
//! - U key: Halve iteration count (less detail, faster)
//! - R key: Reset to default view
//! - Escape: Exit (native only)
//!
//! Run with: RUSTFLAGS="--cfg blade_wgpu" cargo run -p blade-graphics --example webgpu-mandelbrot
//! For WASM: RUSTFLAGS="--cfg blade_wgpu" cargo run-wasm -p blade-graphics --example webgpu-mandelbrot

#![allow(irrefutable_let_patterns)]

use blade_graphics as gpu;
use bytemuck::{Pod, Zeroable};

// -----------------------------------------------------------------------------
// Shader Data
// -----------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ComputeParams {
    center_x: f32,
    center_y: f32,
    zoom: f32,
    max_iterations: u32,
}

/// Compute shader bindings
struct ComputeData {
    params: ComputeParams,
    output_tex: gpu::TextureView,
}

impl gpu::ShaderData for ComputeData {
    fn layout() -> gpu::ShaderDataLayout {
        gpu::ShaderDataLayout {
            bindings: vec![
                ("params", gpu::ShaderBinding::Plain { size: 16 }),
                ("output_tex", gpu::ShaderBinding::Texture),
            ],
        }
    }

    fn fill(&self, mut ctx: gpu::PipelineContext) {
        use gpu::ShaderBindable as _;
        self.params.bind_to(&mut ctx, 0);
        self.output_tex.bind_to(&mut ctx, 1);
    }
}

/// Render shader bindings
struct RenderData {
    fractal_texture: gpu::TextureView,
    fractal_sampler: gpu::Sampler,
}

impl gpu::ShaderData for RenderData {
    fn layout() -> gpu::ShaderDataLayout {
        gpu::ShaderDataLayout {
            bindings: vec![
                ("fractal_texture", gpu::ShaderBinding::Texture),
                ("fractal_sampler", gpu::ShaderBinding::Sampler),
            ],
        }
    }

    fn fill(&self, mut ctx: gpu::PipelineContext) {
        use gpu::ShaderBindable as _;
        self.fractal_texture.bind_to(&mut ctx, 0);
        self.fractal_sampler.bind_to(&mut ctx, 1);
    }
}

// -----------------------------------------------------------------------------
// Example
// -----------------------------------------------------------------------------

const WORKGROUP_SIZE: u32 = 8;

// Default view parameters
const DEFAULT_CENTER_X: f64 = -0.5;
const DEFAULT_CENTER_Y: f64 = 0.0;
const DEFAULT_ZOOM: f64 = 1.0;

struct Example {
    context: gpu::Context,
    surface: gpu::Surface,
    compute_pipeline: gpu::ComputePipeline,
    render_pipeline: gpu::RenderPipeline,
    command_encoder: gpu::CommandEncoder,
    // Fractal texture
    fractal_texture: gpu::Texture,
    fractal_storage_view: gpu::TextureView,
    fractal_sample_view: gpu::TextureView,
    sampler: gpu::Sampler,
    // Interactive view state (f64 for precision at high zoom)
    center_x: f64,
    center_y: f64,
    zoom: f64,
    // Mouse interaction state
    is_dragging: bool,
    last_mouse_pos: Option<(f64, f64)>,
    // Rendering state
    prev_sync_point: Option<gpu::SyncPoint>,
    window_size: winit::dpi::PhysicalSize<u32>,
    needs_redraw: bool,
    // Manual iteration control
    iteration_multiplier: f64,
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
        let compute_source = include_str!("compute.wgsl");
        #[cfg(not(target_arch = "wasm32"))]
        let compute_source =
            std::fs::read_to_string("blade-graphics/examples/webgpu-mandelbrot/compute.wgsl").unwrap();

        #[cfg(target_arch = "wasm32")]
        let render_source = include_str!("render.wgsl");
        #[cfg(not(target_arch = "wasm32"))]
        let render_source =
            std::fs::read_to_string("blade-graphics/examples/webgpu-mandelbrot/render.wgsl").unwrap();

        let compute_shader = context.create_shader(gpu::ShaderDesc {
            source: &compute_source,
        });
        let render_shader = context.create_shader(gpu::ShaderDesc {
            source: &render_source,
        });

        // Create fractal texture
        let fractal_format = gpu::TextureFormat::Rgba8Unorm;
        let (fractal_texture, fractal_storage_view, fractal_sample_view) =
            Self::create_fractal_texture(&context, window_size, fractal_format);

        let sampler = context.create_sampler(gpu::SamplerDesc {
            name: "linear",
            mag_filter: gpu::FilterMode::Linear,
            min_filter: gpu::FilterMode::Linear,
            ..Default::default()
        });

        // Compute pipeline
        let compute_layout = <ComputeData as gpu::ShaderData>::layout();
        let compute_pipeline = context.create_compute_pipeline(gpu::ComputePipelineDesc {
            name: "mandelbrot",
            data_layouts: &[&compute_layout],
            compute: compute_shader.at("main"),
        });

        // Render pipeline
        let render_layout = <RenderData as gpu::ShaderData>::layout();
        let render_pipeline = context.create_render_pipeline(gpu::RenderPipelineDesc {
            name: "display",
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

        let command_encoder = context.create_command_encoder(gpu::CommandEncoderDesc {
            name: "main",
            buffer_count: 2,
        });

        Self {
            context,
            surface,
            compute_pipeline,
            render_pipeline,
            command_encoder,
            fractal_texture,
            fractal_storage_view,
            fractal_sample_view,
            sampler,
            center_x: DEFAULT_CENTER_X,
            center_y: DEFAULT_CENTER_Y,
            zoom: DEFAULT_ZOOM,
            is_dragging: false,
            last_mouse_pos: None,
            prev_sync_point: None,
            window_size,
            needs_redraw: true,
            iteration_multiplier: 1.0,
        }
    }

    /// Reset view to default position
    fn reset_view(&mut self) {
        self.center_x = DEFAULT_CENTER_X;
        self.center_y = DEFAULT_CENTER_Y;
        self.zoom = DEFAULT_ZOOM;
        self.iteration_multiplier = 1.0;
        self.needs_redraw = true;
    }

    /// Adjust iteration multiplier (I to double, U to halve)
    fn adjust_iterations(&mut self, increase: bool) {
        if increase {
            self.iteration_multiplier *= 2.0;
        } else {
            self.iteration_multiplier = (self.iteration_multiplier / 2.0).max(0.25);
        }
        self.needs_redraw = true;
    }

    /// Handle mouse scroll for zooming (centered on cursor position)
    fn handle_scroll(&mut self, delta: f64, cursor_x: f64, cursor_y: f64) {
        let zoom_factor = if delta > 0.0 { 1.1 } else { 1.0 / 1.1 };

        // Convert cursor position to match shader coordinates
        // Shader uses: px = (coord.x / dims.x - 0.5) * aspect, ranging [-0.5*aspect, 0.5*aspect]
        //              py = coord.y / dims.y - 0.5, ranging [-0.5, 0.5]
        //              c = px * (2.0/zoom) + center
        let w = self.window_size.width as f64;
        let h = self.window_size.height as f64;
        let aspect = w / h;

        // Match shader's px/py calculation
        let px = (cursor_x / w - 0.5) * aspect;
        let py = cursor_y / h - 0.5;

        // Fractal coordinates under cursor (before zoom) - must match shader formula
        // shader: c.x = px * (2.0/zoom) + center_x
        let scale = 2.0 / self.zoom;
        let fractal_x = px * scale + self.center_x;
        let fractal_y = py * scale + self.center_y;

        // Apply zoom
        self.zoom *= zoom_factor;
        self.zoom = self.zoom.clamp(0.1, 1e14); // Allow deep zoom with f64 precision

        // Adjust center so the point under cursor stays fixed
        // Solve: fractal_x = px * (2.0/new_zoom) + new_center_x
        // => new_center_x = fractal_x - px * (2.0/new_zoom)
        let new_scale = 2.0 / self.zoom;
        self.center_x = fractal_x - px * new_scale;
        self.center_y = fractal_y - py * new_scale;

        self.needs_redraw = true;
    }

    /// Handle mouse drag for panning
    fn handle_drag(&mut self, new_x: f64, new_y: f64) {
        if let Some((last_x, last_y)) = self.last_mouse_pos {
            let w = self.window_size.width as f64;
            let h = self.window_size.height as f64;
            let aspect = w / h;

            // Convert pixel delta to fractal coordinate delta
            // Shader: c.x = px * (2.0/zoom) + center_x
            // px delta for 1 pixel: aspect / w
            // fractal delta: (aspect / w) * (2.0/zoom) = 2.0 * aspect / (w * zoom)
            let scale = 2.0 / self.zoom;
            let dx = (new_x - last_x) / w * aspect * scale;
            let dy = (new_y - last_y) / h * scale;

            self.center_x -= dx;
            self.center_y -= dy; // No flip - shader y increases downward
            self.needs_redraw = true;
        }
        self.last_mouse_pos = Some((new_x, new_y));
    }

    fn create_fractal_texture(
        context: &gpu::Context,
        size: winit::dpi::PhysicalSize<u32>,
        format: gpu::TextureFormat,
    ) -> (gpu::Texture, gpu::TextureView, gpu::TextureView) {
        let extent = gpu::Extent {
            width: size.width.max(1),
            height: size.height.max(1),
            depth: 1,
        };

        let texture = context.create_texture(gpu::TextureDesc {
            name: "fractal",
            format,
            size: extent,
            dimension: gpu::TextureDimension::D2,
            array_layer_count: 1,
            mip_level_count: 1,
            usage: gpu::TextureUsage::STORAGE | gpu::TextureUsage::RESOURCE,
            sample_count: 1,
            external: None,
        });

        let storage_view = context.create_texture_view(
            texture,
            gpu::TextureViewDesc {
                name: "fractal_storage",
                format,
                dimension: gpu::ViewDimension::D2,
                subresources: &Default::default(),
            },
        );

        let sample_view = context.create_texture_view(
            texture,
            gpu::TextureViewDesc {
                name: "fractal_sample",
                format,
                dimension: gpu::ViewDimension::D2,
                subresources: &Default::default(),
            },
        );

        (texture, storage_view, sample_view)
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.window_size = size;
        self.context
            .reconfigure_surface(&mut self.surface, Self::make_surface_config(size));

        // Recreate fractal texture
        self.context.destroy_texture_view(self.fractal_storage_view);
        self.context.destroy_texture_view(self.fractal_sample_view);
        self.context.destroy_texture(self.fractal_texture);

        let format = gpu::TextureFormat::Rgba8Unorm;
        let (texture, storage_view, sample_view) =
            Self::create_fractal_texture(&self.context, size, format);
        self.fractal_texture = texture;
        self.fractal_storage_view = storage_view;
        self.fractal_sample_view = sample_view;
    }

    fn render(&mut self) {
        if let Some(ref sp) = self.prev_sync_point {
            self.context.wait_for(sp, !0);
        }

        // Dynamically adjust max iterations based on zoom level
        // Higher zoom = more detail needed = more iterations
        // Use quadratic scaling: base + zoom_factor^1.5 for faster iteration growth
        let base_iterations = 500.0 + self.zoom.log2().max(0.0).powf(1.5) * 200.0;
        let max_iterations = ((base_iterations * self.iteration_multiplier).min(50000.0)) as u32;

        // Log current state
        #[cfg(target_arch = "wasm32")]
        log::info!("Zoom: {:.2}x | Iterations: {} | Multiplier: {:.2}x", self.zoom, max_iterations, self.iteration_multiplier);
        #[cfg(not(target_arch = "wasm32"))]
        println!("Zoom: {:.2}x | Iterations: {} | Multiplier: {:.2}x", self.zoom, max_iterations, self.iteration_multiplier);

        // Pass current view state to shader
        // Note: f32 precision limits deep zoom - at zoom > ~1e6, detail is lost
        let params = ComputeParams {
            center_x: self.center_x as f32,
            center_y: self.center_y as f32,
            zoom: self.zoom as f32,
            max_iterations,
        };

        self.needs_redraw = false;

        let frame = self.surface.acquire_frame();
        self.command_encoder.start();

        // Compute pass: generate fractal
        if let mut pass = self.command_encoder.compute("mandelbrot") {
            if let mut pc = pass.with(&self.compute_pipeline) {
                pc.bind(
                    0,
                    &ComputeData {
                        params,
                        output_tex: self.fractal_storage_view,
                    },
                );

                let groups_x = (self.window_size.width + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
                let groups_y = (self.window_size.height + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
                pc.dispatch([groups_x, groups_y, 1]);
            }
        }

        // Render pass: display fractal
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
                    &RenderData {
                        fractal_texture: self.fractal_sample_view,
                        fractal_sampler: self.sampler,
                    },
                );
                rc.draw(0, 6, 0, 1);
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
        self.context.destroy_texture_view(self.fractal_storage_view);
        self.context.destroy_texture_view(self.fractal_sample_view);
        self.context.destroy_texture(self.fractal_texture);
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
        winit::window::Window::default_attributes().with_title("blade-webgpu-mandelbrot");
    #[allow(deprecated)]
    let window = event_loop.create_window(window_attributes).unwrap();

    let mut example = Example::new(&window);
    let mut cursor_pos = (0.0, 0.0);

    #[allow(deprecated)]
    event_loop
        .run(|event, target| {
            // Use Wait - only redraw when needed (on input or resize)
            target.set_control_flow(winit::event_loop::ControlFlow::Wait);
            match event {
                winit::event::Event::AboutToWait => {
                    if example.needs_redraw {
                        window.request_redraw();
                    }
                }
                winit::event::Event::WindowEvent { event, .. } => match event {
                    winit::event::WindowEvent::Resized(size) => {
                        example.resize(size);
                        example.needs_redraw = true;
                        window.request_redraw();
                    }
                    winit::event::WindowEvent::CursorMoved { position, .. } => {
                        cursor_pos = (position.x, position.y);
                        if example.is_dragging {
                            example.handle_drag(position.x, position.y);
                            window.request_redraw();
                        }
                    }
                    winit::event::WindowEvent::MouseWheel { delta, .. } => {
                        let scroll_delta = match delta {
                            winit::event::MouseScrollDelta::LineDelta(_, y) => y as f64,
                            winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y / 50.0,
                        };
                        example.handle_scroll(scroll_delta, cursor_pos.0, cursor_pos.1);
                        window.request_redraw();
                    }
                    winit::event::WindowEvent::MouseInput { state, button, .. } => {
                        if button == winit::event::MouseButton::Left {
                            match state {
                                winit::event::ElementState::Pressed => {
                                    example.is_dragging = true;
                                    example.last_mouse_pos = Some(cursor_pos);
                                }
                                winit::event::ElementState::Released => {
                                    example.is_dragging = false;
                                    example.last_mouse_pos = None;
                                }
                            }
                        }
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
                        use winit::keyboard::KeyCode;
                        match key_code {
                            KeyCode::Escape => target.exit(),
                            KeyCode::KeyR => {
                                example.reset_view();
                                window.request_redraw();
                            }
                            KeyCode::KeyI => {
                                example.adjust_iterations(true);
                                window.request_redraw();
                            }
                            KeyCode::KeyU => {
                                example.adjust_iterations(false);
                                window.request_redraw();
                            }
                            _ => {}
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
        winit::window::Window::default_attributes().with_title("blade-webgpu-mandelbrot");
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
    let cursor_pos: Rc<RefCell<(f64, f64)>> = Rc::new(RefCell::new((0.0, 0.0)));

    let example_clone = example.clone();
    let init_started_clone = init_started.clone();
    let window_clone = window.clone();

    #[allow(deprecated)]
    event_loop
        .run(move |event, target| {
            // Use Wait - browser handles timing via requestAnimationFrame
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
                            log::info!("Mandelbrot initialized! Use mouse to zoom/pan, R to reset.");
                            window_init.request_redraw();
                        });
                    }
                }
                winit::event::Event::WindowEvent { event, .. } => match event {
                    winit::event::WindowEvent::Resized(size) => {
                        if let Some(ref mut ex) = *example.borrow_mut() {
                            ex.resize(size);
                            ex.needs_redraw = true;
                            window.request_redraw();
                        }
                    }
                    winit::event::WindowEvent::CursorMoved { position, .. } => {
                        *cursor_pos.borrow_mut() = (position.x, position.y);
                        if let Some(ref mut ex) = *example.borrow_mut() {
                            if ex.is_dragging {
                                ex.handle_drag(position.x, position.y);
                                window.request_redraw();
                            }
                        }
                    }
                    winit::event::WindowEvent::MouseWheel { delta, .. } => {
                        let scroll_delta = match delta {
                            winit::event::MouseScrollDelta::LineDelta(_, y) => y as f64,
                            winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y / 50.0,
                        };
                        let pos = *cursor_pos.borrow();
                        if let Some(ref mut ex) = *example.borrow_mut() {
                            ex.handle_scroll(scroll_delta, pos.0, pos.1);
                            window.request_redraw();
                        }
                    }
                    winit::event::WindowEvent::MouseInput { state, button, .. } => {
                        if button == winit::event::MouseButton::Left {
                            if let Some(ref mut ex) = *example.borrow_mut() {
                                match state {
                                    winit::event::ElementState::Pressed => {
                                        ex.is_dragging = true;
                                        ex.last_mouse_pos = Some(*cursor_pos.borrow());
                                    }
                                    winit::event::ElementState::Released => {
                                        ex.is_dragging = false;
                                        ex.last_mouse_pos = None;
                                    }
                                }
                            }
                        }
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
                        use winit::keyboard::KeyCode;
                        if let Some(ref mut ex) = *example.borrow_mut() {
                            match key_code {
                                KeyCode::KeyR => {
                                    ex.reset_view();
                                    window.request_redraw();
                                }
                                KeyCode::KeyI => {
                                    ex.adjust_iterations(true);
                                    window.request_redraw();
                                }
                                KeyCode::KeyU => {
                                    ex.adjust_iterations(false);
                                    window.request_redraw();
                                }
                                _ => {}
                            }
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
