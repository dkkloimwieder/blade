//! GPUI Web Demo - Interactive Component
//!
//! Demonstrates a full GPUI component running in the browser:
//! - Click the box to increment counter
//! - Hover to see color change
//! - Press 'r' to reset
//!
//! Run with: cargo run-wasm --example gpui-web

/// Main for native (not supported)
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("This example is designed for WASM. Run with:");
    println!("  cargo run-wasm --example gpui-web");
}

// WASM imports
#[cfg(target_arch = "wasm32")]
use gpui::{
    div, prelude::*, px, rgb, size, App, Application, Bounds, Context, Hsla,
    IntoElement, KeyBinding, ParentElement, Render, Styled, Window, WindowBounds, WindowOptions,
};

// Reset counter action
#[cfg(target_arch = "wasm32")]
gpui::actions!(counter_demo, [ResetCounter]);

/// Main for WASM
#[cfg(target_arch = "wasm32")]
fn main() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).expect("Failed to init logger");

    // Direct console log to verify web_sys works
    web_sys::console::log_1(&"=== GPUI WEB DEMO STARTING (direct console) ===".into());

    log::info!("Starting GPUI Web Demo...");

    // Ensure canvas exists in DOM
    setup_canvas();

    // Initialize renderer FIRST (async), then open window
    // This ensures the GPU atlas is available when text is first rasterized
    init_renderer_then_run_app();
}

/// Initialize the WebRenderer first, then run the GPUI app
#[cfg(target_arch = "wasm32")]
fn init_renderer_then_run_app() {
    use blade_graphics as gpu;
    use gpui::{WebRenderer, WebSurfaceConfig};

    wasm_bindgen_futures::spawn_local(async {
        log::info!("Starting async renderer initialization...");

        let canvas = gpui::get_canvas_element("gpui-canvas").expect("no canvas");

        // Use the CSS display size (client dimensions), not the buffer size
        // The buffer size is set by canvas.set_width/height, but GPUI uses client dimensions for layout
        let dpr = web_sys::window().unwrap().device_pixel_ratio() as u32;
        let display_width = (canvas.client_width() as u32).max(800) * dpr;
        let display_height = (canvas.client_height() as u32).max(600) * dpr;

        // Update canvas buffer to match display size for crisp rendering
        canvas.set_width(display_width);
        canvas.set_height(display_height);

        let size = gpu::Extent {
            width: display_width,
            height: display_height,
            depth: 1,
        };

        log::info!("Initializing renderer with size {}x{} (dpr={})", display_width, display_height, dpr);

        let renderer = WebRenderer::new();
        let config = WebSurfaceConfig {
            size,
            transparent: false,
        };

        match renderer.initialize_async(canvas, config).await {
            Ok(()) => {
                log::info!("WebRenderer initialized successfully!");
                // Pass the renderer to the app - it will be set as pending INSIDE Application::run
                // where the platform is already initialized
                run_gpui_app_with_renderer(renderer);
            }
            Err(e) => {
                log::error!("Failed to initialize WebRenderer: {:?}", e);
                // Fall back to running without renderer (no rendering will work)
                run_gpui_app_without_renderer();
            }
        }
    });
}

/// Run the GPUI application with a pre-initialized renderer
#[cfg(target_arch = "wasm32")]
fn run_gpui_app_with_renderer(renderer: gpui::WebRenderer) {
    use gpui::set_pending_renderer;

    Application::new().run(move |cx: &mut App| {
        log::info!("GPUI Application started, setting pending renderer...");

        // Register keybinding for reset (press 'r')
        cx.bind_keys([KeyBinding::new("r", ResetCounter, None)]);

        // Now that the platform is initialized, set the pending renderer
        // This must happen BEFORE opening the window
        set_pending_renderer(renderer);

        // Open window with our demo component
        // The pending renderer will be automatically attached to the window
        let bounds = Bounds::centered(None, size(px(600.), px(400.)), cx);
        let result = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_window, cx| cx.new(|_cx| CounterDemo::new()),
        );

        match result {
            Ok(handle) => {
                web_sys::console::log_1(&format!("=== Window opened: {:?} ===", handle).into());
                log::info!("Window opened successfully with pre-attached renderer: {:?}", handle);
            }
            Err(e) => {
                web_sys::console::error_1(&format!("=== Window open FAILED: {:?} ===", e).into());
                log::error!("Failed to open window: {:?}", e);
            }
        }
    });

    web_sys::console::log_1(&"=== GPUI run() returned ===".into());
    log::info!("GPUI run() returned, browser event loop will drive execution");
}

/// Fallback: Run without renderer
#[cfg(target_arch = "wasm32")]
fn run_gpui_app_without_renderer() {
    Application::new().run(|cx: &mut App| {
        log::warn!("Running without renderer - no GPU rendering will work");

        let bounds = Bounds::centered(None, size(px(600.), px(400.)), cx);
        let _ = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_window, cx| cx.new(|_cx| CounterDemo::new()),
        );
    });
}

/// Set up the canvas element in the DOM
#[cfg(target_arch = "wasm32")]
fn setup_canvas() {
    use wasm_bindgen::JsCast;

    let window = web_sys::window().expect("no window");
    let document = window.document().expect("no document");

    // Check if canvas already exists
    if document.get_element_by_id("gpui-canvas").is_some() {
        log::info!("Canvas already exists");
        return;
    }

    // Create canvas
    let canvas = document
        .create_element("canvas")
        .expect("failed to create canvas")
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .expect("not a canvas");

    canvas.set_id("gpui-canvas");
    canvas.set_width(800);
    canvas.set_height(600);

    // Style canvas
    canvas.style().set_property("width", "100%").ok();
    canvas.style().set_property("height", "100%").ok();
    canvas.style().set_property("display", "block").ok();

    // Style body
    let body = document.body().expect("no body");
    body.style().set_property("margin", "0").ok();
    body.style().set_property("padding", "0").ok();
    body.style().set_property("overflow", "hidden").ok();
    body.style().set_property("background", "#1a1a2e").ok();

    // Add canvas to body
    body.append_child(&canvas).expect("failed to append canvas");

    log::info!("Canvas created and added to DOM");
}

/// Demo component - a clickable counter
#[cfg(target_arch = "wasm32")]
struct CounterDemo {
    count: u32,
}

#[cfg(target_arch = "wasm32")]
impl CounterDemo {
    fn new() -> Self {
        Self { count: 0 }
    }
}

#[cfg(target_arch = "wasm32")]
impl Render for CounterDemo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .size_full()
            .bg(rgb(0x1a1a2e)) // Dark navy background
            .on_action(cx.listener(|this, _: &ResetCounter, _window, cx| {
                this.count = 0;
                cx.notify();
            }))
            .child(
                // Title
                div()
                    .text_color(rgb(0xeaeaea))
                    .text_xl()
                    .mb_4()
                    .child("GPUI Web Demo"),
            )
            .child(
                // Clickable box
                div()
                    .w(px(200.))
                    .h(px(150.))
                    .bg(Hsla { h: 0.6, s: 0.6, l: 0.4, a: 1.0 })
                    .rounded_lg()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .shadow_lg()
                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _event, _window, cx| {
                        this.count += 1;
                        cx.notify();
                    }))
                    .hover(|style| style.bg(Hsla { h: 0.6, s: 0.7, l: 0.5, a: 1.0 }))
                    .child(
                        div()
                            .text_color(rgb(0xffffff))
                            .text_2xl()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child(format!("{}", self.count)),
                    ),
            )
            .child(
                // Instructions
                div()
                    .mt_4()
                    .text_color(rgb(0x888888))
                    .text_sm()
                    .child("Click to increment · Press R to reset"),
            )
    }
}
