//! GPUI Web Example
//!
//! Entry point for running GPUI in a web browser via WebAssembly.
//!
//! Build: wasm-pack build --target web examples/gpui-web
//! Or: cargo build --target wasm32-unknown-unknown -p gpui-web

use gpui::{DEFAULT_CANVAS_ID, WebRenderer, WebSurfaceConfig};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Canvas element ID used by GPUI
pub const CANVAS_ID: &str = DEFAULT_CANVAS_ID;

/// Global renderer state (single-threaded WASM)
thread_local! {
    static RENDERER: RefCell<Option<WebRenderer>> = const { RefCell::new(None) };
    static CLICK_COUNT: RefCell<u32> = const { RefCell::new(0) };
}

/// Initialize the GPUI web application
///
/// This is called from JavaScript when the WASM module loads.
#[wasm_bindgen(start)]
pub fn main() {
    // Set up panic hook for better error messages
    console_error_panic_hook::set_once();

    // Initialize logging to browser console
    console_log::init_with_level(log::Level::Debug).expect("Failed to initialize logger");

    log::info!("GPUI WASM initializing...");

    // Get the canvas element
    let window = web_sys::window().expect("No window");
    let document = window.document().expect("No document");

    let canvas = document
        .get_element_by_id(CANVAS_ID)
        .expect("Canvas element not found")
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .expect("Element is not a canvas");

    let width = canvas.client_width();
    let height = canvas.client_height();
    let dpr = window.device_pixel_ratio();

    log::info!(
        "Found canvas '{}': {}x{} (device pixel ratio: {})",
        CANVAS_ID,
        width,
        height,
        dpr
    );

    // Start async initialization
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(e) = init_async(canvas, dpr).await {
            log::error!("Initialization failed: {:?}", e);
            update_status(&format!("Error: {:?}", e));
        }
    });
}

/// Async initialization - GPU context requires async on WebGPU
async fn init_async(canvas: web_sys::HtmlCanvasElement, dpr: f64) -> Result<(), JsValue> {
    log::info!("Starting async GPU initialization...");
    update_status("Initializing WebGPU...");

    // Calculate device pixel dimensions
    let width = canvas.client_width() as u32;
    let height = canvas.client_height() as u32;
    let device_width = (width as f64 * dpr) as u32;
    let device_height = (height as f64 * dpr) as u32;

    // Set canvas buffer size for HiDPI
    canvas.set_width(device_width);
    canvas.set_height(device_height);

    // Create renderer
    let renderer = WebRenderer::new();

    // Initialize with canvas
    let config = WebSurfaceConfig {
        size: blade_graphics::Extent {
            width: device_width,
            height: device_height,
            depth: 1,
        },
        transparent: false,
    };

    renderer
        .initialize_async(canvas.clone(), config)
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to initialize renderer: {:?}", e)))?;

    log::info!("WebRenderer initialized successfully!");
    update_status("WebGPU initialized!");

    // Store renderer globally
    RENDERER.with(|r| {
        *r.borrow_mut() = Some(renderer);
    });

    // Set up event listeners for testing
    setup_test_events(&canvas)?;
    log::info!("Event listeners attached");

    // Start the render loop
    start_render_loop();

    log::info!("Render loop started");
    update_status("Running - click canvas to cycle colors, check console for events");

    Ok(())
}

/// Set up simple event listeners for testing
fn setup_test_events(canvas: &web_sys::HtmlCanvasElement) -> Result<(), JsValue> {
    // Make canvas focusable for keyboard events
    canvas.set_tab_index(0);

    // Mouse click - cycle colors and log
    let canvas_click = canvas.clone();
    let onclick = Closure::<dyn FnMut(_)>::new(move |event: web_sys::MouseEvent| {
        let x = event.offset_x();
        let y = event.offset_y();
        let click_count = CLICK_COUNT.with(|c| {
            let mut count = c.borrow_mut();
            *count = (*count + 1) % 4;
            *count
        });
        log::info!("Click at ({}, {}), color index: {}", x, y, click_count);

        // Focus canvas for keyboard events
        if let Ok(element) = canvas_click.clone().dyn_into::<web_sys::HtmlElement>() {
            let _ = element.focus();
        }
    });
    canvas.add_event_listener_with_callback("click", onclick.as_ref().unchecked_ref())?;
    onclick.forget();

    // Mouse move - log position
    let onmousemove = Closure::<dyn FnMut(_)>::new(move |event: web_sys::MouseEvent| {
        let x = event.offset_x();
        let y = event.offset_y();
        // Only log occasionally to avoid console spam
        if (x + y) % 50 == 0 {
            log::debug!("Mouse at ({}, {})", x, y);
        }
    });
    canvas.add_event_listener_with_callback("mousemove", onmousemove.as_ref().unchecked_ref())?;
    onmousemove.forget();

    // Key down - log key presses
    let onkeydown = Closure::<dyn FnMut(_)>::new(move |event: web_sys::KeyboardEvent| {
        let key = event.key();
        let code = event.code();
        log::info!("Key down: '{}' (code: {})", key, code);
    });
    canvas.add_event_listener_with_callback("keydown", onkeydown.as_ref().unchecked_ref())?;
    onkeydown.forget();

    Ok(())
}

/// Start the requestAnimationFrame render loop
fn start_render_loop() {
    // Use Rc<RefCell<Option<Closure>>> pattern for recursive closure
    let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();

    *g.borrow_mut() = Some(Closure::new(move || {
        // Get current color index from click count
        let color_index = CLICK_COUNT.with(|c| *c.borrow());

        // Render a frame - clear to color based on click count
        RENDERER.with(|r| {
            if let Some(ref renderer) = *r.borrow() {
                renderer.clear_with_index(color_index);
            }
        });

        // Schedule next frame
        if let Some(window) = web_sys::window() {
            if let Some(ref closure) = *f.borrow() {
                let _ = window.request_animation_frame(closure.as_ref().unchecked_ref());
            }
        }
    }));

    // Start the loop and leak the closure to keep it alive
    // (In a real app, you'd want to store this and cancel it on cleanup)
    {
        let closure = g.borrow_mut().take();
        if let Some(closure) = closure {
            if let Some(window) = web_sys::window() {
                let _ = window.request_animation_frame(closure.as_ref().unchecked_ref());
            }
            closure.forget();
        }
    }
}

/// Exported function to check if WASM is loaded (for debugging)
#[wasm_bindgen]
pub fn is_loaded() -> bool {
    true
}

/// Get the expected canvas ID (for JavaScript reference)
#[wasm_bindgen]
pub fn get_canvas_id() -> String {
    CANVAS_ID.to_string()
}

/// Update the status element in the HTML page
fn update_status(message: &str) {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(status) = document.get_element_by_id("status") {
                status.set_text_content(Some(message));
            }
        }
    }
}
