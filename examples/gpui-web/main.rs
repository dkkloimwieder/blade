//! GPUI Web Example
//!
//! Demonstrates GPUI's WebGPU rendering in the browser.
//!
//! Run with: cargo run-wasm --example gpui-web

use blade_graphics as gpu;
use std::cell::RefCell;

thread_local! {
    static CLICK_COUNT: RefCell<u32> = const { RefCell::new(0) };
}

struct Example {
    context: gpu::Context,
    surface: gpu::Surface,
    surface_config: gpu::SurfaceConfig,
    command_encoder: gpu::CommandEncoder,
    last_sync_point: Option<gpu::SyncPoint>,
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
            #[cfg(all(target_arch = "wasm32", blade_wgpu))]
            display_sync: gpu::DisplaySync::Block,
            #[cfg(not(all(target_arch = "wasm32", blade_wgpu)))]
            display_sync: gpu::DisplaySync::Recent,
            ..Default::default()
        }
    }

    #[cfg(not(all(target_arch = "wasm32", blade_wgpu)))]
    fn new(window: &winit::window::Window) -> Self {
        let context = unsafe {
            gpu::Context::init(gpu::ContextDesc {
                presentation: true,
                validation: cfg!(debug_assertions),
                ..Default::default()
            })
        }
        .expect("Failed to create GPU context");

        Self::init_with_context(context, window)
    }

    #[cfg(all(target_arch = "wasm32", blade_wgpu))]
    async fn new_async(window: &winit::window::Window) -> Self {
        let context = gpu::Context::init_async(gpu::ContextDesc {
            presentation: true,
            validation: cfg!(debug_assertions),
            ..Default::default()
        })
        .await
        .expect("Failed to create GPU context");

        Self::init_with_context(context, window)
    }

    fn init_with_context(context: gpu::Context, window: &winit::window::Window) -> Self {
        let size = window.inner_size();

        let surface = context
            .create_surface_configured(window, Self::make_surface_config(size))
            .expect("Failed to create surface");

        let surface_config = Self::make_surface_config(size);

        let command_encoder = context.create_command_encoder(gpu::CommandEncoderDesc {
            name: "gpui-web",
            buffer_count: 2,
        });

        Self {
            context,
            surface,
            surface_config,
            command_encoder,
            last_sync_point: None,
        }
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.surface_config = Self::make_surface_config(size);
        self.context
            .reconfigure_surface(&mut self.surface, self.surface_config.clone());
    }

    fn render(&mut self) {
        // Wait for previous frame
        if let Some(ref sp) = self.last_sync_point {
            let _ = self.context.wait_for(sp, 1000);
        }

        // Acquire frame
        let frame = self.surface.acquire_frame();

        // Get clear color based on click count
        let color_index = CLICK_COUNT.with(|c| *c.borrow());
        let clear_color = match color_index % 3 {
            0 => gpu::TextureColor::OpaqueBlack,
            1 => gpu::TextureColor::White,
            _ => gpu::TextureColor::TransparentBlack,
        };

        // Begin encoding
        self.command_encoder.start();
        self.command_encoder.init_texture(frame.texture());

        // Clear screen
        {
            let _pass = self.command_encoder.render(
                "clear",
                gpu::RenderTargetSet {
                    colors: &[gpu::RenderTarget {
                        view: frame.texture_view(),
                        init_op: gpu::InitOp::Clear(clear_color),
                        finish_op: gpu::FinishOp::Store,
                    }],
                    depth_stencil: None,
                },
            );
        }

        // Present
        self.command_encoder.present(frame);
        self.last_sync_point = Some(self.context.submit(&mut self.command_encoder));
    }
}

/// Main for native and GLES WASM (sync init)
#[cfg(not(all(target_arch = "wasm32", blade_wgpu)))]
fn main() {
    env_logger::init();

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let window_attributes = winit::window::Window::default_attributes()
        .with_title("GPUI Web Example")
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600));

    let window = event_loop.create_window(window_attributes).unwrap();
    let mut example = Example::new(&window);

    event_loop
        .run(move |event, target| {
            use winit::event::{Event, WindowEvent};
            use winit::event_loop::ControlFlow;

            target.set_control_flow(ControlFlow::Poll);

            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        target.exit();
                    }
                    WindowEvent::Resized(size) => {
                        example.resize(size);
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        if state == winit::event::ElementState::Pressed
                            && button == winit::event::MouseButton::Left
                        {
                            CLICK_COUNT.with(|c| {
                                let mut count = c.borrow_mut();
                                *count = (*count + 1) % 3;
                                log::info!("Click! Color index: {}", *count);
                            });
                        }
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        log::info!("Key: {:?}", event.logical_key);
                    }
                    WindowEvent::RedrawRequested => {
                        example.render();
                    }
                    _ => {}
                },
                Event::AboutToWait => {
                    window.request_redraw();
                }
                _ => {}
            }
        })
        .unwrap();
}

/// Main for WebGPU WASM (async init)
#[cfg(all(target_arch = "wasm32", blade_wgpu))]
fn main() {
    use std::rc::Rc;
    use winit::platform::web::WindowExtWebSys as _;

    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).expect("Failed to init logger");

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let window_attributes = winit::window::Window::default_attributes()
        .with_title("GPUI Web Example");
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

    let example_clone = example.clone();
    let init_started_clone = init_started.clone();
    let window_clone = window.clone();

    event_loop
        .run(move |event, target| {
            use winit::event::{Event, WindowEvent};
            use winit::event_loop::ControlFlow;

            target.set_control_flow(ControlFlow::Wait);

            match event {
                Event::AboutToWait => {
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
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        target.exit();
                    }
                    WindowEvent::Resized(size) => {
                        if let Some(ref mut ex) = *example.borrow_mut() {
                            ex.resize(size);
                        }
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        if state == winit::event::ElementState::Pressed
                            && button == winit::event::MouseButton::Left
                        {
                            CLICK_COUNT.with(|c| {
                                let mut count = c.borrow_mut();
                                *count = (*count + 1) % 3;
                                log::info!("Click! Color index: {}", *count);
                            });
                        }
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        log::info!("Key: {:?}", event.logical_key);
                    }
                    WindowEvent::RedrawRequested => {
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
