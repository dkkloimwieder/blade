//! GPUI Web Example - Quad Rendering Test
//!
//! Demonstrates WebRenderer's quad rendering in the browser.
//! - Renders a colored quad that follows the mouse
//! - Click to cycle through colors (red, green, blue)
//! - Shows rounded corners
//!
//! Run with: cargo run-wasm --example gpui-web

use std::cell::RefCell;

#[cfg(target_arch = "wasm32")]
use blade_graphics as gpu;

thread_local! {
    static CLICK_COUNT: RefCell<u32> = const { RefCell::new(0) };
    static MOUSE_POS: RefCell<(f32, f32)> = const { RefCell::new((400.0, 300.0)) };
}

/// Get quad color based on click count (HSL format)
fn get_quad_color(index: u32) -> [f32; 4] {
    match index % 6 {
        0 => [0.0, 1.0, 0.5, 1.0],    // Red
        1 => [0.33, 1.0, 0.5, 1.0],   // Green
        2 => [0.66, 1.0, 0.5, 1.0],   // Blue
        3 => [0.08, 1.0, 0.5, 1.0],   // Orange
        4 => [0.83, 1.0, 0.5, 1.0],   // Purple
        _ => [0.5, 1.0, 0.5, 1.0],    // Cyan
    }
}

/// Main for native (sync init)
#[cfg(not(all(target_arch = "wasm32", blade_wgpu)))]
fn main() {
    env_logger::init();
    println!("This example is designed for WASM. Run with:");
    println!("  RUSTFLAGS=\"--cfg blade_wgpu\" cargo run-wasm --example gpui-web");
}

/// Main for WebGPU WASM (async init)
#[cfg(all(target_arch = "wasm32", blade_wgpu))]
fn main() {
    use gpui::{DevicePixels, Size, WebRenderer, WebSurfaceConfig};
    use std::rc::Rc;
    use winit::platform::web::WindowExtWebSys as _;
    use wasm_bindgen::JsCast;

    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).expect("Failed to init logger");

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let window_attributes = winit::window::Window::default_attributes()
        .with_title("GPUI Quad Test");
    let window = Rc::new(event_loop.create_window(window_attributes).unwrap());

    // Set up canvas
    let canvas = window.canvas().unwrap();
    canvas.set_id(gpu::CANVAS_ID);

    // Style the canvas to fill the viewport
    canvas.style().set_property("width", "100%").ok();
    canvas.style().set_property("height", "100%").ok();
    canvas.style().set_property("display", "block").ok();

    let body = web_sys::window()
        .and_then(|win| win.document())
        .and_then(|doc| doc.body())
        .expect("couldn't get document body");

    // Style body for full viewport
    body.style().set_property("margin", "0").ok();
    body.style().set_property("padding", "0").ok();
    body.style().set_property("overflow", "hidden").ok();
    body.style().set_property("background", "#222").ok();

    body.append_child(&web_sys::Element::from(canvas.clone()))
        .expect("couldn't append canvas to document body");

    // Create WebRenderer
    let renderer = Rc::new(WebRenderer::new());
    let init_started = Rc::new(RefCell::new(false));

    let renderer_clone = renderer.clone();
    let init_started_clone = init_started.clone();
    let canvas_clone = canvas.clone();

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
                        let renderer_init = renderer_clone.clone();
                        let canvas_init = canvas_clone.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            let size = gpu::Extent {
                                width: canvas_init.width().max(800),
                                height: canvas_init.height().max(600),
                                depth: 1,
                            };
                            let config = WebSurfaceConfig {
                                size,
                                transparent: false,
                            };
                            if let Err(e) = renderer_init.initialize_async(canvas_init, config).await {
                                log::error!("Failed to initialize WebRenderer: {:?}", e);
                            } else {
                                log::info!("WebRenderer initialized! Click to change colors, move mouse to move quad.");
                            }
                        });
                    }
                    window.request_redraw();
                }
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        target.exit();
                    }
                    WindowEvent::Resized(size) => {
                        if renderer.is_initialized() {
                            renderer.update_drawable_size(Size {
                                width: DevicePixels(size.width as i32),
                                height: DevicePixels(size.height as i32),
                            });
                        }
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        MOUSE_POS.with(|pos| {
                            *pos.borrow_mut() = (position.x as f32, position.y as f32);
                        });
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        if state == winit::event::ElementState::Pressed
                            && button == winit::event::MouseButton::Left
                        {
                            CLICK_COUNT.with(|c| {
                                let mut count = c.borrow_mut();
                                *count = (*count + 1) % 6;
                                log::info!("Click! Color index: {}", *count);
                            });
                        }
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        log::info!("Key: {:?}", event.logical_key);
                    }
                    WindowEvent::RedrawRequested => {
                        if renderer.is_initialized() {
                            let (mx, my) = MOUSE_POS.with(|pos| *pos.borrow());
                            let color_index = CLICK_COUNT.with(|c| *c.borrow());
                            let color = get_quad_color(color_index);

                            // Draw a 200x200 quad centered on mouse position
                            let quad_size = 200.0;
                            let x = (mx - quad_size / 2.0).max(0.0);
                            let y = (my - quad_size / 2.0).max(0.0);

                            renderer.draw_test_quad(x, y, quad_size, quad_size, color);
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        })
        .unwrap();
}
