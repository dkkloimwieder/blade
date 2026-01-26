//! Test modules for GPUI WASM correctness testing

mod visual;
mod layout;
mod text;
mod events;
mod advanced;
mod shadows;
mod paths;
mod stress;

pub use visual::*;
pub use layout::*;
pub use text::*;
// events and stress only define impl blocks on TestHarness
pub use advanced::*;
pub use shadows::*;
pub use paths::*;

use gpui::{div, px, rgb, IntoElement, ParentElement, Styled};

/// Standard test card wrapper - builds a test card with header, content, and criteria
pub fn test_card(id: &str, name: &str, criteria: &str, content: impl IntoElement) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .mb_6()
        .p_4()
        .bg(rgb(0x252540))
        .rounded_lg()
        .child(
            // Header
            div()
                .flex()
                .items_center()
                .gap_3()
                .mb_3()
                .child(
                    div()
                        .px_2()
                        .py_1()
                        .bg(rgb(0x6366f1))
                        .rounded_md()
                        .text_xs()
                        .font_weight(gpui::FontWeight::BOLD)
                        .child(id.to_string()),
                )
                .child(div().text_sm().font_weight(gpui::FontWeight::SEMIBOLD).child(name.to_string())),
        )
        .child(
            // Content area
            div()
                .p_4()
                .bg(rgb(0x1a1a2e))
                .rounded_md()
                .mb_3()
                .child(content),
        )
        .child(
            // Criteria
            div()
                .text_xs()
                .text_color(rgb(0x888888))
                .child(format!("✓ {}", criteria)),
        )
}

/// Grid wrapper for test cards
pub fn test_grid() -> gpui::Div {
    div().flex().flex_col().gap_4()
}

/// Row wrapper for multiple items
pub fn test_row() -> gpui::Div {
    div().flex().flex_row().flex_wrap().gap_4()
}

/// Standard colored box for visual tests
pub fn colored_box(color: impl Into<gpui::Rgba>, size: f32) -> gpui::Div {
    let color = color.into();
    div()
        .w(px(size))
        .h(px(size))
        .bg(color)
}

/// Labeled colored box
pub fn labeled_box(color: impl Into<gpui::Rgba>, label: &str) -> gpui::Div {
    let color = color.into();
    div()
        .flex()
        .items_center()
        .justify_center()
        .w(px(50.))
        .h(px(50.))
        .bg(color)
        .text_xs()
        .text_color(rgb(0xffffff))
        .child(label.to_string())
}
