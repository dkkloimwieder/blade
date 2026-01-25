//! Aspirational tests - features not yet implemented
//! U01-U03: Underline rendering

use gpui::{
    div, px, rgb, IntoElement, ParentElement, Styled,
};
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
                .child(div().text_2xl().child("!"))
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
                                .child("These features are not yet available in GPUI WASM"),
                        ),
                ),
        )
}

// =============================================================================
// UNDERLINE TESTS (U01-U03) - NOT IMPLEMENTED
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
                        .child("Expected: Squiggly red line"),
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
}
