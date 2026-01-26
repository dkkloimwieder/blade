//! GPUI WASM Correctness Testing Demo
//!
//! A comprehensive visual testing suite for verifying GPUI WASM features.
//! Each test case renders a visual element with clear success criteria.
//!
//! Run with: cargo run-wasm --example gpui-web-test

mod tests;

/// Main for native (not supported)
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("This example is designed for WASM. Run with:");
    println!("  cargo run-wasm --example gpui-web-test");
}

#[cfg(target_arch = "wasm32")]
use gpui::{
    actions, div, prelude::*, px, rgb, size, AnyElement, App, Application, Bounds, Context,
    FocusHandle, IntoElement, KeyBinding, KeyDownEvent, KeyUpEvent, ModifiersChangedEvent,
    MouseButton, ParentElement, Render, Styled, Window, WindowBounds, WindowOptions,
};

#[cfg(target_arch = "wasm32")]
use tests::*;

// Actions
#[cfg(target_arch = "wasm32")]
actions!(
    gpui_test,
    [
        NextCategory,
        PrevCategory,
        ResetState,
        TestKeyAction,
        CtrlSAction
    ]
);

/// Main for WASM
#[cfg(target_arch = "wasm32")]
fn main() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).expect("Failed to init logger");

    web_sys::console::log_1(&"=== GPUI WASM Test Demo Starting ===".into());
    log::info!("Starting GPUI WASM Test Demo...");

    setup_canvas();
    init_renderer_then_run_app();
}

#[cfg(target_arch = "wasm32")]
fn init_renderer_then_run_app() {
    use blade_graphics as gpu;
    use gpui::{WebRenderer, WebSurfaceConfig};

    wasm_bindgen_futures::spawn_local(async {
        log::info!("Starting async renderer initialization...");

        let canvas = gpui::get_canvas_element("gpui-canvas").expect("no canvas");

        let dpr = web_sys::window().unwrap().device_pixel_ratio() as u32;
        let display_width = (canvas.client_width() as u32).max(1200) * dpr;
        let display_height = (canvas.client_height() as u32).max(800) * dpr;

        canvas.set_width(display_width);
        canvas.set_height(display_height);

        let size = gpu::Extent {
            width: display_width,
            height: display_height,
            depth: 1,
        };

        log::info!(
            "Initializing renderer with size {}x{} (dpr={})",
            display_width,
            display_height,
            dpr
        );

        let renderer = WebRenderer::new();
        let config = WebSurfaceConfig {
            size,
            transparent: false,
        };

        match renderer.initialize_async(canvas, config).await {
            Ok(()) => {
                log::info!("WebRenderer initialized successfully!");
                run_gpui_app_with_renderer(renderer);
            }
            Err(e) => {
                log::error!("Failed to initialize WebRenderer: {:?}", e);
            }
        }
    });
}

#[cfg(target_arch = "wasm32")]
fn run_gpui_app_with_renderer(renderer: gpui::WebRenderer) {
    use gpui::set_pending_renderer;

    Application::new().run(move |cx: &mut App| {
        log::info!("GPUI Application started");

        // Register keybindings
        cx.bind_keys([
            KeyBinding::new("tab", NextCategory, None),
            KeyBinding::new("shift-tab", PrevCategory, None),
            KeyBinding::new("r", ResetState, None),
            KeyBinding::new("t", TestKeyAction, None),
            KeyBinding::new("ctrl-s", CtrlSAction, None),
        ]);

        set_pending_renderer(renderer);

        let bounds = Bounds::centered(None, size(px(1200.), px(800.)), cx);
        let result = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_window, cx| cx.new(|cx| TestHarness::new(cx)),
        );

        match result {
            Ok(handle) => {
                log::info!("Test window opened successfully: {:?}", handle);
            }
            Err(e) => {
                log::error!("Failed to open window: {:?}", e);
            }
        }
    });
}

#[cfg(target_arch = "wasm32")]
fn setup_canvas() {
    use wasm_bindgen::JsCast;

    let window = web_sys::window().expect("no window");
    let document = window.document().expect("no document");

    if document.get_element_by_id("gpui-canvas").is_some() {
        log::info!("Canvas already exists");
        return;
    }

    let canvas = document
        .create_element("canvas")
        .expect("failed to create canvas")
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .expect("not a canvas");

    canvas.set_id("gpui-canvas");
    canvas.set_width(1200);
    canvas.set_height(800);

    canvas.style().set_property("width", "100%").ok();
    canvas.style().set_property("height", "100%").ok();
    canvas.style().set_property("display", "block").ok();

    let body = document.body().expect("no body");
    body.style().set_property("margin", "0").ok();
    body.style().set_property("padding", "0").ok();
    body.style().set_property("overflow", "hidden").ok();
    body.style().set_property("background", "#0f0f1a").ok();

    body.append_child(&canvas).expect("failed to append canvas");

    log::info!("Canvas created and added to DOM");
}

/// Test categories
#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestCategory {
    Quads,
    LayoutFlex,
    Sizing,
    Spacing,
    Text,
    MouseEvents,
    KeyboardEvents,
    ScrollWheel,
    DragDrop,
    Focus,
    Tooltips,
    Shadows,
    Paths,
    Underlines,
    Sprites,
    StressTests,
}

#[cfg(target_arch = "wasm32")]
impl TestCategory {
    fn all() -> &'static [TestCategory] {
        &[
            TestCategory::Quads,
            TestCategory::LayoutFlex,
            TestCategory::Sizing,
            TestCategory::Spacing,
            TestCategory::Text,
            TestCategory::MouseEvents,
            TestCategory::KeyboardEvents,
            TestCategory::ScrollWheel,
            TestCategory::DragDrop,
            TestCategory::Focus,
            TestCategory::Tooltips,
            TestCategory::Shadows,
            TestCategory::Paths,
            TestCategory::Underlines,
            TestCategory::Sprites,
            TestCategory::StressTests,
        ]
    }

    fn name(&self) -> &'static str {
        match self {
            TestCategory::Quads => "Quads",
            TestCategory::LayoutFlex => "Layout: Flex",
            TestCategory::Sizing => "Layout: Sizing",
            TestCategory::Spacing => "Layout: Spacing",
            TestCategory::Text => "Text",
            TestCategory::MouseEvents => "Mouse Events",
            TestCategory::KeyboardEvents => "Keyboard",
            TestCategory::ScrollWheel => "Scroll/Wheel",
            TestCategory::DragDrop => "Drag & Drop",
            TestCategory::Focus => "Focus/Tab",
            TestCategory::Tooltips => "Tooltips",
            TestCategory::Shadows => "Shadows",
            TestCategory::Paths => "Paths",
            TestCategory::Underlines => "Underlines",
            TestCategory::Sprites => "Sprites",
            TestCategory::StressTests => "Stress Tests",
        }
    }

    fn is_implemented(&self) -> bool {
        // All test categories are now implemented
        true
    }
}

/// Main test harness component
#[cfg(target_arch = "wasm32")]
pub struct TestHarness {
    selected_category: TestCategory,
    focus_handle: FocusHandle,
    // Event test state
    click_count: u32,
    double_click_count: u32,
    right_click_count: u32,
    mouse_position: (f32, f32),
    last_key: String,
    last_key_up: String,
    active_modifiers: String,
    key_action_fired: bool,
    ctrl_s_fired: bool,
    scroll_delta: (f32, f32),
    scroll_position: f32,
    mouse_up_count: u32,
    // Drag state
    drag_position: Option<(f32, f32)>,
    drop_received: Option<String>,
    // Focus state
    focused_element: Option<usize>,
    // Focus test handles (F01-F04)
    f01_focus: FocusHandle,
    f02_focus_1: FocusHandle,
    f02_focus_2: FocusHandle,
    f02_focus_3: FocusHandle,
    f02_focus_4: FocusHandle,
    f04_focus: FocusHandle,
    f04_focus_event: String,
    // Stress test state
    stress_counter: u32,
}

#[cfg(target_arch = "wasm32")]
impl TestHarness {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            selected_category: TestCategory::Quads,
            focus_handle: cx.focus_handle(),
            click_count: 0,
            double_click_count: 0,
            right_click_count: 0,
            mouse_position: (0.0, 0.0),
            last_key: String::new(),
            last_key_up: String::new(),
            active_modifiers: String::new(),
            key_action_fired: false,
            ctrl_s_fired: false,
            scroll_delta: (0.0, 0.0),
            scroll_position: 0.0,
            mouse_up_count: 0,
            drag_position: None,
            drop_received: None,
            focused_element: None,
            f01_focus: cx.focus_handle(),
            f02_focus_1: cx.focus_handle(),
            f02_focus_2: cx.focus_handle(),
            f02_focus_3: cx.focus_handle(),
            f02_focus_4: cx.focus_handle(),
            f04_focus: cx.focus_handle(),
            f04_focus_event: String::new(),
            stress_counter: 0,
        }
    }

    fn next_category(&mut self) {
        let categories = TestCategory::all();
        let current_idx = categories
            .iter()
            .position(|c| *c == self.selected_category)
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % categories.len();
        self.selected_category = categories[next_idx];
    }

    fn prev_category(&mut self) {
        let categories = TestCategory::all();
        let current_idx = categories
            .iter()
            .position(|c| *c == self.selected_category)
            .unwrap_or(0);
        let prev_idx = if current_idx == 0 {
            categories.len() - 1
        } else {
            current_idx - 1
        };
        self.selected_category = categories[prev_idx];
    }

    fn reset_state(&mut self) {
        self.click_count = 0;
        self.double_click_count = 0;
        self.right_click_count = 0;
        self.mouse_position = (0.0, 0.0);
        self.last_key.clear();
        self.last_key_up.clear();
        self.active_modifiers.clear();
        self.key_action_fired = false;
        self.ctrl_s_fired = false;
        self.scroll_delta = (0.0, 0.0);
        self.scroll_position = 0.0;
        self.drag_position = None;
        self.drop_received = None;
        self.focused_element = None;
        self.stress_counter = 0;
    }
}

#[cfg(target_arch = "wasm32")]
impl Render for TestHarness {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bg_dark = rgb(0x0f0f1a);
        let bg_sidebar = rgb(0x1a1a2e);
        let bg_surface = rgb(0x252540);
        let text_primary = rgb(0xeaeaea);
        let text_muted = rgb(0x888888);
        let accent = rgb(0x6366f1);

        div()
            .flex()
            .size_full()
            .bg(bg_dark)
            .text_color(text_primary)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|this, _: &NextCategory, _window, cx| {
                this.next_category();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &PrevCategory, _window, cx| {
                this.prev_category();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ResetState, _window, cx| {
                this.reset_state();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &TestKeyAction, _window, cx| {
                this.key_action_fired = true;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &CtrlSAction, _window, cx| {
                this.ctrl_s_fired = true;
                cx.notify();
            }))
            // Global key event handler for K01 test
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                this.last_key = event.keystroke.key.clone();
                cx.notify();
            }))
            // Key up handler for K02 test
            .on_key_up(cx.listener(|this, event: &KeyUpEvent, _window, cx| {
                this.last_key_up = event.keystroke.key.clone();
                cx.notify();
            }))
            // Modifier change handler for K03 test
            .on_modifiers_changed(cx.listener(|this, event: &ModifiersChangedEvent, _window, cx| {
                let mut mods = Vec::new();
                if event.modifiers.control {
                    mods.push("Ctrl");
                }
                if event.modifiers.alt {
                    mods.push("Alt");
                }
                if event.modifiers.shift {
                    mods.push("Shift");
                }
                if event.modifiers.platform {
                    mods.push("Meta");
                }
                this.active_modifiers = if mods.is_empty() {
                    String::new()
                } else {
                    mods.join("+")
                };
                cx.notify();
            }))
            // Sidebar
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w(px(200.))
                    .h_full()
                    .bg(bg_sidebar)
                    .border_r_1()
                    .border_color(bg_surface)
                    .child(
                        // Header
                        div()
                            .p_4()
                            .border_b_1()
                            .border_color(bg_surface)
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child("GPUI Test Suite"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_muted)
                                    .mt_1()
                                    .child("Tab/Shift-Tab: Navigate"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_muted)
                                    .child("R: Reset state"),
                            ),
                    )
                    .child(
                        // Category list
                        div().flex().flex_col().p_2().gap_1().children(
                            TestCategory::all().iter().map(|category| {
                                let is_selected = *category == self.selected_category;
                                let cat = *category;
                                div()
                                    .px_3()
                                    .py_2()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .text_sm()
                                    .when(is_selected, |el| el.bg(accent).text_color(rgb(0xffffff)))
                                    .when(!is_selected, |el| {
                                        el.hover(|style| style.bg(bg_surface))
                                    })
                                    .when(!category.is_implemented(), |el| {
                                        el.text_color(text_muted)
                                    })
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(move |this, _event, _window, cx| {
                                            this.selected_category = cat;
                                            cx.notify();
                                        }),
                                    )
                                    .child(category.name())
                            }),
                        ),
                    )
                    .child(
                        // Footer with legend
                        div()
                            .mt_auto()
                            .p_4()
                            .border_t_1()
                            .border_color(bg_surface)
                            .text_xs()
                            .text_color(text_muted)
                            .child("* Not yet implemented"),
                    ),
            )
            // Main content area
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(
                        // Category header
                        div()
                            .px_6()
                            .py_4()
                            .border_b_1()
                            .border_color(bg_surface)
                            .child(
                                div()
                                    .text_xl()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child(self.selected_category.name()),
                            )
                            .when(!self.selected_category.is_implemented(), |el| {
                                el.child(
                                    div()
                                        .mt_2()
                                        .px_3()
                                        .py_1()
                                        .bg(rgb(0xfbbf24))
                                        .text_color(rgb(0x000000))
                                        .text_sm()
                                        .rounded_md()
                                        .child("NOT IMPLEMENTED - Aspirational Feature"),
                                )
                            }),
                    )
                    // Test content
                    .child(
                        div()
                            .id("test-content")
                            .flex_1()
                            .overflow_scroll()
                            .p_6()
                            .child(self.render_category_tests(cx)),
                    ),
            )
    }
}

#[cfg(target_arch = "wasm32")]
impl TestHarness {
    fn render_category_tests(&mut self, cx: &mut Context<Self>) -> AnyElement {
        match self.selected_category {
            TestCategory::Quads => render_quad_tests().into_any_element(),
            TestCategory::LayoutFlex => render_layout_flex_tests().into_any_element(),
            TestCategory::Sizing => render_sizing_tests().into_any_element(),
            TestCategory::Spacing => render_spacing_tests().into_any_element(),
            TestCategory::Text => render_text_tests().into_any_element(),
            TestCategory::MouseEvents => self.render_mouse_event_tests(cx).into_any_element(),
            TestCategory::KeyboardEvents => self.render_keyboard_tests(cx).into_any_element(),
            TestCategory::ScrollWheel => self.render_scroll_tests(cx).into_any_element(),
            TestCategory::DragDrop => self.render_drag_drop_tests(cx).into_any_element(),
            TestCategory::Focus => self.render_focus_tests(cx).into_any_element(),
            TestCategory::Tooltips => self.render_tooltip_tests(cx).into_any_element(),
            TestCategory::Shadows => render_shadows().into_any_element(),
            TestCategory::Paths => render_path_tests().into_any_element(),
            TestCategory::Underlines => render_aspirational_underlines().into_any_element(),
            TestCategory::Sprites => render_sprite_tests().into_any_element(),
            TestCategory::StressTests => self.render_stress_tests(cx).into_any_element(),
        }
    }
}
