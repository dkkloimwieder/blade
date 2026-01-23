//! Aspirational tests - Features not yet implemented in GPUI WASM
//! Paths (P01-P04), Underlines (U01-U05)

use gpui::{
    canvas, div, point, px, rgb, IntoElement, ParentElement, PathBuilder, Pixels, Point, Styled,
};
use super::test_card;

/// Create a filled triangle path
fn create_triangle(p1: Point<Pixels>, p2: Point<Pixels>, p3: Point<Pixels>) -> gpui::Path<Pixels> {
    let mut builder = PathBuilder::fill();
    builder.move_to(p1);
    builder.line_to(p2);
    builder.line_to(p3);
    builder.close();
    builder.build().unwrap()
}

/// Canvas element that draws a purple triangle
fn triangle_canvas() -> impl IntoElement {
    canvas(
        move |_bounds, _window, _cx| {},
        move |bounds, _, window, _cx| {
            let center_x = bounds.origin.x + bounds.size.width / 2.0;
            let center_y = bounds.origin.y + bounds.size.height / 2.0;

            // Draw a triangle pointing up
            let triangle = create_triangle(
                point(center_x, center_y - px(30.)),           // Top
                point(center_x - px(30.), center_y + px(20.)), // Bottom left
                point(center_x + px(30.), center_y + px(20.)), // Bottom right
            );
            window.paint_path(triangle, rgb(0x6366f1)); // Purple
        },
    )
    .w(px(100.))
    .h(px(80.))
    .bg(rgb(0x1a1a2e))
    .rounded_md()
}

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

#[allow(dead_code)]
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

// =============================================================================
// PATH TESTS (P01-P04) - NOT IMPLEMENTED
// =============================================================================

pub fn render_aspirational_paths() -> impl IntoElement {
    test_grid()
        // P01: Triangle - NOW IMPLEMENTED
        .child(test_card("P01", "Triangle", "Path with 3 vertices",
            div()
                .flex()
                .gap_4()
                .child(triangle_canvas())
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x888888))
                        .flex()
                        .items_center()
                        .child("Filled triangle using PathBuilder"),
                ),
        ))
        .child(not_implemented_banner())
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
