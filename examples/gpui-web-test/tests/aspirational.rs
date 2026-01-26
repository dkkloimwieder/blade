//! Underline rendering tests (U01-U03)
//! Now implemented in GPUI WASM!

use gpui::{
    div, px, rgb, IntoElement, ParentElement, Styled,
};
use super::test_card;

/// Helper function to create a test grid
fn test_grid() -> gpui::Div {
    div().flex().flex_col().gap_4()
}

// =============================================================================
// UNDERLINE TESTS (U01-U03) - IMPLEMENTED
// =============================================================================

pub fn render_aspirational_underlines() -> impl IntoElement {
    test_grid()
        // U01: Basic Underline
        .child(test_card("U01", "Basic Underline", "Line under text",
            div()
                .flex()
                .gap_4()
                .child(
                    div()
                        .underline()
                        .child("Underlined text"),
                )
                .child(
                    div()
                        .underline()
                        .text_decoration_2()
                        .child("Thick underline"),
                ),
        ))
        // U02: Wavy Underline
        .child(test_card("U02", "Wavy Underline", "Wavy line visible",
            div()
                .flex()
                .gap_4()
                .child(
                    div()
                        .underline()
                        .text_decoration_wavy()
                        .text_decoration_color(rgb(0xef4444))
                        .child("Wavy red underline"),
                )
                .child(
                    div()
                        .underline()
                        .text_decoration_wavy()
                        .text_decoration_color(rgb(0x22c55e))
                        .child("Wavy green underline"),
                ),
        ))
        // U03: Underline Color
        .child(test_card("U03", "Underline Color", "Colored underline",
            div()
                .flex()
                .gap_4()
                .child(
                    div()
                        .underline()
                        .text_decoration_color(rgb(0x3b82f6))
                        .child("Blue underline"),
                )
                .child(
                    div()
                        .underline()
                        .text_decoration_color(rgb(0xf59e0b))
                        .text_decoration_2()
                        .child("Orange thick underline"),
                ),
        ))
}
