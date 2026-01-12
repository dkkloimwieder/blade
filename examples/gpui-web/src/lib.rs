//! GPUI Web Example
//!
//! Entry point for running GPUI in a web browser via WebAssembly.
//!
//! Build: wasm-pack build --target web examples/gpui-web
//! Or: cargo build --target wasm32-unknown-unknown -p gpui-web

use wasm_bindgen::prelude::*;

/// Canvas element ID used by GPUI
pub const CANVAS_ID: &str = "gpui-canvas";

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

    log::info!(
        "Found canvas '{}': {}x{} (device pixel ratio: {})",
        CANVAS_ID,
        width,
        height,
        window.device_pixel_ratio()
    );

    // Start async initialization
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(e) = init_async(canvas).await {
            log::error!("Initialization failed: {:?}", e);
        }
    });
}

/// Async initialization - GPU context requires async on WebGPU
async fn init_async(canvas: web_sys::HtmlCanvasElement) -> Result<(), JsValue> {
    log::info!("Starting async GPU initialization...");

    // For now, just log success - full initialization will be added in blade-1wbn
    log::info!("GPUI WASM initialized successfully!");
    log::info!("Canvas ready for rendering");

    // TODO (blade-1wbn): Initialize WebRenderer
    // TODO (blade-1wbn): Setup event listeners
    // TODO (blade-1wbn): Start animation loop

    Ok(())
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
