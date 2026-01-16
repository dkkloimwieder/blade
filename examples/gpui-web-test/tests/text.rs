//! Text rendering tests (T01-T11)

use gpui::{div, px, rgb, IntoElement, ParentElement, Styled};
use super::test_card;

/// Helper function to create a test grid
fn test_grid() -> gpui::Div {
    div().flex().flex_col().gap_4()
}

pub fn render_text_tests() -> impl IntoElement {
    test_grid()
        // T01: Basic Text
        .child(test_card("T01", "Basic Text", "Text visible and readable",
            div().child("Hello World"),
        ))
        // T02: Text Color
        .child(test_card(
            "T02",
            "Text Color",
            "Red text visible on dark background",
            div()
                .text_color(rgb(0xef4444))
                .child("This text should be red"),
        ))
        // T03: Font Sizes
        .child(test_card("T03", "Font Sizes", "Clear size progression",
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(div().text_xs().child("text_xs - Extra Small"))
                .child(div().text_sm().child("text_sm - Small"))
                .child(div().child("default - Base"))
                .child(div().text_lg().child("text_lg - Large"))
                .child(div().text_xl().child("text_xl - Extra Large"))
                .child(div().text_2xl().child("text_2xl - 2X Large"))
                .child(div().text_3xl().child("text_3xl - 3X Large")),
        ))
        // T04: Font Weight
        .child(test_card("T04", "Font Weight", "Visible weight differences",
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::LIGHT)
                        .child("Light Weight"),
                )
                .child(
                    div()
                        .font_weight(gpui::FontWeight::NORMAL)
                        .child("Normal Weight"),
                )
                .child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child("Semibold Weight"),
                )
                .child(
                    div()
                        .font_weight(gpui::FontWeight::BOLD)
                        .child("Bold Weight"),
                )
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .child("Extra Bold Weight"),
                ),
        ))
        // T05: Text Alignment (simulated with flex)
        .child(test_card("T05", "Text Alignment", "Left, Center, Right aligned",
            div()
                .flex()
                .flex_col()
                .gap_2()
                .w(px(250.))
                .child(
                    div()
                        .w_full()
                        .bg(rgb(0x1a1a2e))
                        .p_2()
                        .flex()
                        .justify_start()
                        .child("Left aligned"),
                )
                .child(
                    div()
                        .w_full()
                        .bg(rgb(0x1a1a2e))
                        .p_2()
                        .flex()
                        .justify_center()
                        .child("Center aligned"),
                )
                .child(
                    div()
                        .w_full()
                        .bg(rgb(0x1a1a2e))
                        .p_2()
                        .flex()
                        .justify_end()
                        .child("Right aligned"),
                ),
        ))
        // T06: Line Height (simulated with different padding/spacing)
        .child(test_card(
            "T06",
            "Line Height",
            "Spacing difference visible between lines",
            div()
                .flex()
                .gap_4()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .p_2()
                        .bg(rgb(0x1a1a2e))
                        .child(div().text_xs().text_color(rgb(0x888888)).child("Tight"))
                        .child("Line 1")
                        .child("Line 2")
                        .child("Line 3"),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .p_2()
                        .bg(rgb(0x1a1a2e))
                        .child(div().text_xs().text_color(rgb(0x888888)).child("Loose"))
                        .child("Line 1")
                        .child("Line 2")
                        .child("Line 3"),
                ),
        ))
        // T07: Text Truncation
        .child(test_card(
            "T07",
            "Text Truncation",
            "Ellipsis at end (if supported)",
            div()
                .w(px(200.))
                .overflow_hidden()
                .bg(rgb(0x1a1a2e))
                .p_2()
                .truncate()
                .child("This is a very long text that should be truncated with an ellipsis at the end"),
        ))
        // T08: Line Clamp (simulated with fixed height)
        .child(test_card(
            "T08",
            "Line Clamp",
            "Max 2 lines visible (clipped)",
            div()
                .w(px(200.))
                .max_h(px(50.))
                .overflow_hidden()
                .bg(rgb(0x1a1a2e))
                .p_2()
                .child("This is a multi-line text that should be clamped to 2 lines maximum. Any additional content should be hidden."),
        ))
        // T09: Whitespace Nowrap
        .child(test_card(
            "T09",
            "Whitespace Nowrap",
            "Single line, no wrap",
            div()
                .w(px(150.))
                .overflow_hidden()
                .bg(rgb(0x1a1a2e))
                .p_2()
                .whitespace_nowrap()
                .child("This text should not wrap to multiple lines"),
        ))
        // T10: Text Background (simulated with inline div)
        .child(test_card(
            "T10",
            "Text Background",
            "Highlight behind text",
            div()
                .flex()
                .gap_1()
                .child("Normal text with ")
                .child(
                    div()
                        .bg(rgb(0xfbbf24))
                        .text_color(rgb(0x000000))
                        .px_1()
                        .rounded_sm()
                        .child("highlighted"),
                )
                .child(" text"),
        ))
        // T11: Mixed Content
        .child(test_card(
            "T11",
            "Mixed Content",
            "Text and colored divs render correctly together",
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child("Text above")
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(div().w(px(30.)).h(px(30.)).bg(rgb(0xef4444)))
                        .child(div().flex().items_center().child("Inline text"))
                        .child(div().w(px(30.)).h(px(30.)).bg(rgb(0x22c55e))),
                )
                .child("Text below"),
        ))
}
