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

    // Start the render loop
    start_render_loop();

    log::info!("Render loop started");
    update_status("Running");

    Ok(())
}

/// Start the requestAnimationFrame render loop
fn start_render_loop() {
    // Use Rc<RefCell<Option<Closure>>> pattern for recursive closure
    let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();

    *g.borrow_mut() = Some(Closure::new(move || {
        // Render a frame - clear to black
        RENDERER.with(|r| {
            if let Some(ref renderer) = *r.borrow() {
                renderer.clear();
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
