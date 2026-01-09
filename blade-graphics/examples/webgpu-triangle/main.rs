//! Minimal WebGPU triangle example for blade-graphics
//!
//! This example tests the WebGPU backend with a simple colored triangle.
//! Demonstrates interactive color selection via keyboard (1-6 keys).
//!
//! Run with: RUSTFLAGS="--cfg blade_wgpu" cargo run --example webgpu-triangle
//! For WASM: RUSTFLAGS="--cfg blade_wgpu" cargo run-wasm --example webgpu-triangle

#![allow(irrefutable_let_patterns)]

use blade_graphics as gpu;
use bytemuck::{Pod, Zeroable};
use gpu::ShaderData;

// -----------------------------------------------------------------------------
// Color Tint Uniform
// -----------------------------------------------------------------------------

/// Color tint uniform passed to the fragment shader.
/// The alpha component controls blend strength: 0 = vertex colors, 1 = full tint.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ColorTint {
    rgba: [f32; 4],
}

impl ColorTint {
    fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { rgba: [r, g, b, a] }
    }

    /// No tint - show original vertex colors
    fn none() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    /// Preset colors with full tint
    fn red() -> Self {
        Self::new(1.0, 0.2, 0.2, 1.0)
    }
    fn green() -> Self {
        Self::new(0.2, 1.0, 0.2, 1.0)
    }
    fn blue() -> Self {
        Self::new(0.2, 0.2, 1.0, 1.0)
    }
    fn yellow() -> Self {
        Self::new(1.0, 1.0, 0.2, 1.0)
    }
    fn purple() -> Self {
        Self::new(0.8, 0.2, 1.0, 1.0)
    }
}

// -----------------------------------------------------------------------------
// Shader Data Binding
// -----------------------------------------------------------------------------

struct TriangleParams {
    color_tint: ColorTint,
}

impl gpu::ShaderData for TriangleParams {
    fn layout() -> gpu::ShaderDataLayout {
        gpu::ShaderDataLayout {
            // Matches WGSL: var<uniform> uniforms: Uniforms (16 bytes for vec4<f32>)
            bindings: vec![("uniforms", gpu::ShaderBinding::Plain { size: 16 })],
        }
    }

    fn fill(&self, mut ctx: gpu::PipelineContext) {
        use gpu::ShaderBindable as _;
        self.color_tint.bind_to(&mut ctx, 0);
    }
}

// -----------------------------------------------------------------------------
// Example Application
// -----------------------------------------------------------------------------

struct Example {
    context: gpu::Context,
    surface: gpu::Surface,
    pipeline: gpu::RenderPipeline,
    command_encoder: gpu::CommandEncoder,
    prev_sync_point: Option<gpu::SyncPoint>,
    window_size: winit::dpi::PhysicalSize<u32>,
    // Current color tint
    current_color: ColorTint,
    // Frame counter for timing display
    frame_count: u32,
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
                timing: true, // Enable GPU timing
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
            timing: true, // Enable GPU timing
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
            std::fs::read_to_string("blade-graphics/examples/webgpu-triangle/shader.wgsl").unwrap();

        let shader = context.create_shader(gpu::ShaderDesc {
            source: &shader_source,
        });

        // Create render pipeline with color tint uniform binding
        let pipeline = context.create_render_pipeline(gpu::RenderPipelineDesc {
            name: "triangle",
            data_layouts: &[&TriangleParams::layout()],
            vertex: shader.at("vs_main"),
            vertex_fetches: &[],
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

        let command_encoder = context.create_command_encoder(gpu::CommandEncoderDesc {
            name: "main",
            buffer_count: 2,
        });

        println!("Controls: 1-5 = preset colors, 0 = original vertex colors, Esc = quit");

        Self {
            context,
            surface,
            pipeline,
            command_encoder,
            prev_sync_point: None,
            window_size,
            current_color: ColorTint::none(),
            frame_count: 0,
        }
    }

    fn set_color(&mut self, color: ColorTint) {
        self.current_color = color;
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

        // Acquire frame
        let frame = self.surface.acquire_frame();

        // Record commands
        self.command_encoder.start();

        // Prepare shader params with current color
        let params = TriangleParams {
            color_tint: self.current_color,
        };

        if let mut pass = self.command_encoder.render(
            "triangle",
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
                encoder.bind(0, &params);
                encoder.draw(0, 3, 0, 1);
            }
        }

        self.command_encoder.present(frame);
        self.prev_sync_point = Some(self.context.submit(&mut self.command_encoder));

        // Display GPU timing every 60 frames
        self.frame_count += 1;
        if self.frame_count % 60 == 0 {
            let timing = self.context.timing_results();
            if !timing.is_empty() {
                #[cfg(target_arch = "wasm32")]
                {
                    for (name, duration) in &timing {
                        log::info!("GPU timing - {}: {:?}", name, duration);
                    }
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    for (name, duration) in &timing {
                        println!("GPU timing - {}: {:?}", name, duration);
                    }
                }
            }
        }
    }

    fn deinit(mut self) {
        if let Some(ref sp) = self.prev_sync_point {
            self.context.wait_for(sp, !0);
        }
        self.context.destroy_command_encoder(&mut self.command_encoder);
        self.context.destroy_surface(&mut self.surface);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    env_logger::init();

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let window_attributes =
        winit::window::Window::default_attributes().with_title("blade-webgpu-triangle");
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
                        use winit::keyboard::KeyCode;
                        match key_code {
                            KeyCode::Escape => target.exit(),
                            KeyCode::Digit0 => example.set_color(ColorTint::none()),
                            KeyCode::Digit1 => example.set_color(ColorTint::red()),
                            KeyCode::Digit2 => example.set_color(ColorTint::green()),
                            KeyCode::Digit3 => example.set_color(ColorTint::blue()),
                            KeyCode::Digit4 => example.set_color(ColorTint::yellow()),
                            KeyCode::Digit5 => example.set_color(ColorTint::purple()),
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

#[cfg(target_arch = "wasm32")]
fn main() {
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::prelude::*;
    use winit::platform::web::WindowExtWebSys as _;

    console_error_panic_hook::set_once();
    console_log::init().expect("could not initialize logger");

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let window_attributes =
        winit::window::Window::default_attributes().with_title("blade-webgpu-triangle");
    let window = Rc::new(event_loop.create_window(window_attributes).unwrap());

    // Set up canvas
    let canvas = window.canvas().unwrap();
    canvas.set_id(gpu::CANVAS_ID);

    let document = web_sys::window()
        .and_then(|win| win.document())
        .expect("couldn't get document");
    let body = document.body().expect("couldn't get body");

    body.append_child(&web_sys::Element::from(canvas))
        .expect("couldn't append canvas to document body");

    // Create color dropdown
    let select: web_sys::HtmlSelectElement = document
        .create_element("select")
        .expect("couldn't create select")
        .dyn_into()
        .unwrap();
    select.set_id("color-picker");
    select
        .style()
        .set_property("position", "absolute")
        .unwrap();
    select.style().set_property("top", "10px").unwrap();
    select.style().set_property("left", "10px").unwrap();
    select.style().set_property("font-size", "16px").unwrap();
    select.style().set_property("padding", "5px").unwrap();

    let colors = [
        ("Original", "0"),
        ("Red", "1"),
        ("Green", "2"),
        ("Blue", "3"),
        ("Yellow", "4"),
        ("Purple", "5"),
    ];

    for (name, value) in colors {
        let option: web_sys::HtmlOptionElement = document
            .create_element("option")
            .expect("couldn't create option")
            .dyn_into()
            .unwrap();
        option.set_value(value);
        option.set_text_content(Some(name));
        select.append_child(&option).unwrap();
    }

    body.append_child(&select).expect("couldn't append select");

    // State machine: None = initializing, Some = ready
    let example: Rc<RefCell<Option<Example>>> = Rc::new(RefCell::new(None));
    let init_started: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));

    // Set up dropdown change handler
    let example_for_select = example.clone();
    let on_change = Closure::<dyn FnMut(_)>::new(move |event: web_sys::Event| {
        let target = event.target().unwrap();
        let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
        let value = select.value();
        if let Some(ref mut ex) = *example_for_select.borrow_mut() {
            let color = match value.as_str() {
                "0" => ColorTint::none(),
                "1" => ColorTint::red(),
                "2" => ColorTint::green(),
                "3" => ColorTint::blue(),
                "4" => ColorTint::yellow(),
                "5" => ColorTint::purple(),
                _ => ColorTint::none(),
            };
            ex.set_color(color);
        }
    });
    select
        .add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref())
        .unwrap();
    on_change.forget(); // Leak the closure to keep it alive

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
                            log::info!("WebGPU initialized!");
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
                                KeyCode::Digit0 => ex.set_color(ColorTint::none()),
                                KeyCode::Digit1 => ex.set_color(ColorTint::red()),
                                KeyCode::Digit2 => ex.set_color(ColorTint::green()),
                                KeyCode::Digit3 => ex.set_color(ColorTint::blue()),
                                KeyCode::Digit4 => ex.set_color(ColorTint::yellow()),
                                KeyCode::Digit5 => ex.set_color(ColorTint::purple()),
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
