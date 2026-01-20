//! Aspirational tests - Features not yet implemented in GPUI WASM
//! Shadows (SH01-SH05), Paths (P01-P04), Underlines (U01-U05)

use gpui::{div, hsla, point, px, rgb, BoxShadow, IntoElement, ParentElement, Styled};
use super::test_card;

/// Helper function to create a test grid
fn test_grid() -> gpui::Div {
    div().flex().flex_col().gap_4()
}

fn not_implemented_banner() -> impl IntoElement {
    div()
        .w_full()
        .p_4()
        .mb_4()
        .bg(rgb(0x422006))
        .border_2()
        .border_color(rgb(0xfbbf24))
        .rounded_lg()
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .text_2xl()
                        .child("⚠"),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(rgb(0xfbbf24))
                                .child("NOT IMPLEMENTED"),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0xfcd34d))
                                .child("These features are aspirational and not yet available in GPUI WASM"),
                        ),
                ),
        )
}

fn placeholder_visual(label: &str) -> impl IntoElement {
    div()
        .w(px(100.))
        .h(px(80.))
        .bg(rgb(0x1a1a2e))
        .border_2()
        .border_dashed()
        .border_color(rgb(0x444466))
        .rounded_md()
        .flex()
        .items_center()
        .justify_center()
        .text_xs()
        .text_color(rgb(0x666688))
        .child(label.to_string())
}

fn shadow_box() -> gpui::Div {
    div()
        .p_8()
        .bg(rgb(0xffffff))
        .child(
            div()
                .w(px(100.))
                .h(px(80.))
                .bg(rgb(0x4a4a8e))
                .rounded_md()
                .shadow_lg()
        )
}

fn shadow_box_colored(color: gpui::Hsla) -> gpui::Div {
    div()
        .p_8()
        .bg(rgb(0xffffff))
        .child(
            div()
                .w(px(100.))
                .h(px(80.))
                .bg(rgb(0x4a4a8e))
                .rounded_md()
                .shadow(vec![BoxShadow {
                    color,
                    offset: point(px(0.), px(4.)),
                    blur_radius: px(15.),
                    spread_radius: px(0.),
                }])
        )
}

fn shadow_box_offset(x: f32, y: f32) -> gpui::Div {
    div()
        .p_8()
        .bg(rgb(0xffffff))
        .child(
            div()
                .w(px(100.))
                .h(px(80.))
                .bg(rgb(0x4a4a8e))
                .rounded_md()
                .shadow(vec![BoxShadow {
                    color: hsla(0.0, 0.0, 0.0, 0.3),
                    offset: point(px(x), px(y)),
                    blur_radius: px(12.),
                    spread_radius: px(0.),
                }])
        )
}

fn shadow_box_blur(blur: f32) -> gpui::Div {
    div()
        .p_8()
        .bg(rgb(0xffffff))
        .child(
            div()
                .w(px(80.))
                .h(px(60.))
                .bg(rgb(0x4a4a8e))
                .rounded_md()
                .shadow(vec![BoxShadow {
                    color: hsla(0.0, 0.0, 0.0, 0.35),
                    offset: point(px(0.), px(4.)),
                    blur_radius: px(blur),
                    spread_radius: px(0.),
                }])
        )
}

// =============================================================================
// SHADOW TESTS (SH01-SH05) - NOT IMPLEMENTED
// =============================================================================

pub fn render_aspirational_shadows() -> impl IntoElement {
    test_grid()
        // SH01: Basic Shadow - NOW IMPLEMENTED
        .child(test_card("SH01", "Basic Shadow", "Shadow visible around element",
            div()
                .flex()
                .gap_4()
                .child(shadow_box())
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x888888))
                        .flex()
                        .items_center()
                        .child("Expected: Soft shadow around box"),
                ),
        ))
        // SH02: Shadow Color - NOW IMPLEMENTED
        .child(test_card("SH02", "Shadow Color", "Colored shadow",
            div()
                .flex()
                .gap_4()
                .child(shadow_box_colored(hsla(0.6, 0.8, 0.4, 0.5))) // Blue shadow
                .child(shadow_box_colored(hsla(0.0, 0.8, 0.5, 0.4))) // Red shadow
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x888888))
                        .flex()
                        .items_center()
                        .child("Blue and red shadows"),
                ),
        ))
        // SH03: Shadow Offset - NOW IMPLEMENTED
        .child(test_card("SH03", "Shadow Offset", "Shadow with x/y offset",
            div()
                .flex()
                .gap_4()
                .child(shadow_box_offset(8., 8.))   // Down-right
                .child(shadow_box_offset(-8., 8.))  // Down-left
                .child(shadow_box_offset(0., 12.))  // Straight down
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x888888))
                        .flex()
                        .items_center()
                        .child("Shadows: down-right, down-left, straight down"),
                ),
        ))
        // SH04: Shadow Blur - NOW IMPLEMENTED
        .child(test_card("SH04", "Shadow Blur", "Varying blur radius",
            div()
                .flex()
                .gap_4()
                .child(shadow_box_blur(2.))
                .child(shadow_box_blur(8.))
                .child(shadow_box_blur(16.))
                .child(shadow_box_blur(24.))
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x888888))
                        .flex()
                        .items_center()
                        .child("Blur: 2px, 8px, 16px, 24px"),
                ),
        ))
        .child(not_implemented_banner())
        // SH05: Multiple Shadows
        .child(test_card("SH05", "Multiple Shadows", "Stacked shadows",
            div()
                .flex()
                .gap_4()
                .child(placeholder_visual("multi"))
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x888888))
                        .flex()
                        .items_center()
                        .child("Expected: Inner + outer shadow"),
                ),
        ))
}

// =============================================================================
// PATH TESTS (P01-P04) - NOT IMPLEMENTED
// =============================================================================

pub fn render_aspirational_paths() -> impl IntoElement {
    test_grid()
        .child(not_implemented_banner())
        // P01: Triangle
        .child(test_card("P01", "Triangle", "Path with 3 vertices",
            div()
                .flex()
                .gap_4()
                .child(
                    // ASCII art representation of expected output
                    div()
                        .w(px(100.))
                        .h(px(80.))
                        .bg(rgb(0x1a1a2e))
                        .rounded_md()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_3xl()
                        .text_color(rgb(0x6366f1))
                        .child("▲"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x888888))
                        .flex()
                        .items_center()
                        .child("Expected: Filled triangle"),
                ),
        ))
        // P02: Custom Polygon
        .child(test_card("P02", "Custom Polygon", "5-sided polygon",
            div()
                .flex()
                .gap_4()
                .child(
                    div()
                        .w(px(100.))
                        .h(px(80.))
                        .bg(rgb(0x1a1a2e))
                        .rounded_md()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_3xl()
                        .text_color(rgb(0x22c55e))
                        .child("⬠"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x888888))
                        .flex()
                        .items_center()
                        .child("Expected: Pentagon shape"),
                ),
        ))
        // P03: Curved Path
        .child(test_card("P03", "Curved Path", "Bezier curve",
            div()
                .flex()
                .gap_4()
                .child(
                    div()
                        .w(px(100.))
                        .h(px(80.))
                        .bg(rgb(0x1a1a2e))
                        .rounded_md()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_3xl()
                        .text_color(rgb(0xf59e0b))
                        .child("〰"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x888888))
                        .flex()
                        .items_center()
                        .child("Expected: Smooth curve"),
                ),
        ))
        // P04: Path Fill
        .child(test_card("P04", "Path Fill", "Filled custom shape",
            div()
                .flex()
                .gap_4()
                .child(
                    div()
                        .w(px(100.))
                        .h(px(80.))
                        .bg(rgb(0x1a1a2e))
                        .rounded_md()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_3xl()
                        .text_color(rgb(0xec4899))
                        .child("♥"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x888888))
                        .flex()
                        .items_center()
                        .child("Expected: Filled heart shape"),
                ),
        ))
}

// =============================================================================
// UNDERLINE TESTS (U01-U05) - NOT IMPLEMENTED
// =============================================================================

pub fn render_aspirational_underlines() -> impl IntoElement {
    test_grid()
        .child(not_implemented_banner())
        // U01: Basic Underline
        .child(test_card("U01", "Basic Underline", "Line under text",
            div()
                .flex()
                .gap_4()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child("Underlined text")
                        .child(
                            div()
                                .w_full()
                                .h(px(1.))
                                .bg(rgb(0xeaeaea))
                                .mt(px(-2.)),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x888888))
                        .flex()
                        .items_center()
                        .child("(simulated with div)"),
                ),
        ))
        // U02: Wavy Underline
        .child(test_card("U02", "Wavy Underline", "Wavy line visible",
            div()
                .flex()
                .gap_4()
                .child(
                    div()
                        .child("Wavy underline")
                        .text_color(rgb(0xef4444)),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x888888))
                        .flex()
                        .items_center()
                        .child("Expected: Squiggly red line (spell-check style)"),
                ),
        ))
        // U03: Underline Color
        .child(test_card("U03", "Underline Color", "Colored underline",
            div()
                .flex()
                .gap_4()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child("Blue underline")
                        .child(
                            div()
                                .w_full()
                                .h(px(2.))
                                .bg(rgb(0x3b82f6))
                                .mt(px(-2.)),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x888888))
                        .flex()
                        .items_center()
                        .child("(simulated with div)"),
                ),
        ))
        // U04: Underline Thickness
        .child(test_card("U04", "Underline Thickness", "Varying thickness",
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child("Thin (1px)")
                        .child(div().w_full().h(px(1.)).bg(rgb(0xeaeaea)).mt(px(-2.))),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child("Medium (2px)")
                        .child(div().w_full().h(px(2.)).bg(rgb(0xeaeaea)).mt(px(-2.))),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child("Thick (4px)")
                        .child(div().w_full().h(px(4.)).bg(rgb(0xeaeaea)).mt(px(-2.))),
                ),
        ))
        // U05: Strikethrough
        .child(test_card("U05", "Strikethrough", "Line through text",
            div()
                .flex()
                .gap_4()
                .child(
                    div()
                        .relative()
                        .child("Strikethrough text")
                        .child(
                            div()
                                .absolute()
                                .w_full()
                                .h(px(1.))
                                .bg(rgb(0xeaeaea))
                                .top(px(10.)),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x888888))
                        .flex()
                        .items_center()
                        .child("(simulated with positioned div)"),
                ),
        ))
}
