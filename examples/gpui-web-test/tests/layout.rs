//! Layout tests - Flexbox (L01-L19), Sizing (S01-S06), Spacing (SP01-SP07)

use gpui::{div, px, rgb, IntoElement, ParentElement, Styled};
use super::{test_card, test_grid, colored_box, labeled_box};

// =============================================================================
// FLEXBOX TESTS (L01-L19)
// =============================================================================

pub fn render_layout_flex_tests() -> impl IntoElement {
    test_grid()
        // L01: Flex Row
        .child(test_card("L01", "Flex Row", "Horizontal arrangement",
            div()
                .flex()
                .flex_row()
                .gap_2()
                .child(labeled_box(rgb(0xef4444), "1"))
                .child(labeled_box(rgb(0x22c55e), "2"))
                .child(labeled_box(rgb(0x3b82f6), "3")),
        ))
        // L02: Flex Column
        .child(test_card("L02", "Flex Column", "Vertical arrangement",
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(labeled_box(rgb(0xef4444), "1"))
                .child(labeled_box(rgb(0x22c55e), "2"))
                .child(labeled_box(rgb(0x3b82f6), "3")),
        ))
        // L03: Flex Row Reverse
        .child(test_card("L03", "Flex Row Reverse", "3-2-1 order",
            div()
                .flex()
                .flex_row_reverse()
                .gap_2()
                .child(labeled_box(rgb(0xef4444), "1"))
                .child(labeled_box(rgb(0x22c55e), "2"))
                .child(labeled_box(rgb(0x3b82f6), "3")),
        ))
        // L04: Flex Column Reverse
        .child(test_card("L04", "Flex Column Reverse", "3-2-1 top to bottom",
            div()
                .flex()
                .flex_col_reverse()
                .gap_2()
                .child(labeled_box(rgb(0xef4444), "1"))
                .child(labeled_box(rgb(0x22c55e), "2"))
                .child(labeled_box(rgb(0x3b82f6), "3")),
        ))
        // L05: Justify Start
        .child(test_card("L05", "Justify Start", "Aligned to start",
            div()
                .flex()
                .justify_start()
                .gap_2()
                .w(px(300.))
                .h(px(60.))
                .bg(rgb(0x1a1a2e))
                .p_2()
                .child(colored_box(rgb(0x6366f1), 40.))
                .child(colored_box(rgb(0x6366f1), 40.))
                .child(colored_box(rgb(0x6366f1), 40.)),
        ))
        // L06: Justify End
        .child(test_card("L06", "Justify End", "Aligned to end",
            div()
                .flex()
                .justify_end()
                .gap_2()
                .w(px(300.))
                .h(px(60.))
                .bg(rgb(0x1a1a2e))
                .p_2()
                .child(colored_box(rgb(0x6366f1), 40.))
                .child(colored_box(rgb(0x6366f1), 40.))
                .child(colored_box(rgb(0x6366f1), 40.)),
        ))
        // L07: Justify Center
        .child(test_card("L07", "Justify Center", "Centered in container",
            div()
                .flex()
                .justify_center()
                .gap_2()
                .w(px(300.))
                .h(px(60.))
                .bg(rgb(0x1a1a2e))
                .p_2()
                .child(colored_box(rgb(0x6366f1), 40.))
                .child(colored_box(rgb(0x6366f1), 40.))
                .child(colored_box(rgb(0x6366f1), 40.)),
        ))
        // L08: Justify Space Between
        .child(test_card(
            "L08",
            "Justify Space Between",
            "Equal space between, none at edges",
            div()
                .flex()
                .justify_between()
                .w(px(300.))
                .h(px(60.))
                .bg(rgb(0x1a1a2e))
                .p_2()
                .child(colored_box(rgb(0x6366f1), 40.))
                .child(colored_box(rgb(0x6366f1), 40.))
                .child(colored_box(rgb(0x6366f1), 40.)),
        ))
        // L09: Justify Space Around
        .child(test_card(
            "L09",
            "Justify Space Around",
            "Equal space around each",
            div()
                .flex()
                .justify_around()
                .w(px(300.))
                .h(px(60.))
                .bg(rgb(0x1a1a2e))
                .p_2()
                .child(colored_box(rgb(0x6366f1), 40.))
                .child(colored_box(rgb(0x6366f1), 40.))
                .child(colored_box(rgb(0x6366f1), 40.)),
        ))
        // L10: Justify Space Evenly (not available in gpui, use around as alternative)
        .child(test_card(
            "L10",
            "Justify Space Evenly",
            "Equal space everywhere (using around)",
            div()
                .flex()
                .justify_around()
                .w(px(300.))
                .h(px(60.))
                .bg(rgb(0x1a1a2e))
                .p_2()
                .child(colored_box(rgb(0x10b981), 40.))
                .child(colored_box(rgb(0x10b981), 40.))
                .child(colored_box(rgb(0x10b981), 40.)),
        ))
        // L11: Items Start
        .child(test_card("L11", "Items Start", "Tops aligned",
            div()
                .flex()
                .items_start()
                .gap_2()
                .w(px(200.))
                .h(px(100.))
                .bg(rgb(0x1a1a2e))
                .p_2()
                .child(div().w(px(40.)).h(px(30.)).bg(rgb(0xef4444)))
                .child(div().w(px(40.)).h(px(50.)).bg(rgb(0x22c55e)))
                .child(div().w(px(40.)).h(px(70.)).bg(rgb(0x3b82f6))),
        ))
        // L12: Items End
        .child(test_card("L12", "Items End", "Bottoms aligned",
            div()
                .flex()
                .items_end()
                .gap_2()
                .w(px(200.))
                .h(px(100.))
                .bg(rgb(0x1a1a2e))
                .p_2()
                .child(div().w(px(40.)).h(px(30.)).bg(rgb(0xef4444)))
                .child(div().w(px(40.)).h(px(50.)).bg(rgb(0x22c55e)))
                .child(div().w(px(40.)).h(px(70.)).bg(rgb(0x3b82f6))),
        ))
        // L13: Items Center
        .child(test_card("L13", "Items Center", "Vertically centered",
            div()
                .flex()
                .items_center()
                .gap_2()
                .w(px(200.))
                .h(px(100.))
                .bg(rgb(0x1a1a2e))
                .p_2()
                .child(div().w(px(40.)).h(px(30.)).bg(rgb(0xef4444)))
                .child(div().w(px(40.)).h(px(50.)).bg(rgb(0x22c55e)))
                .child(div().w(px(40.)).h(px(70.)).bg(rgb(0x3b82f6))),
        ))
        // L14: Items Stretch (not explicitly available, showing default behavior)
        .child(test_card(
            "L14",
            "Items Stretch",
            "All same height as container",
            div()
                .flex()
                .gap_2()
                .w(px(200.))
                .h(px(80.))
                .bg(rgb(0x1a1a2e))
                .p_2()
                .child(div().w(px(40.)).h_full().bg(rgb(0xef4444)))
                .child(div().w(px(40.)).h_full().bg(rgb(0x22c55e)))
                .child(div().w(px(40.)).h_full().bg(rgb(0x3b82f6))),
        ))
        // L15: Flex Grow
        .child(test_card(
            "L15",
            "Flex Grow",
            "Middle fills remaining space",
            div()
                .flex()
                .gap_2()
                .w(px(300.))
                .h(px(60.))
                .bg(rgb(0x1a1a2e))
                .p_2()
                .child(colored_box(rgb(0xef4444), 40.))
                .child(div().flex_1().h(px(40.)).bg(rgb(0x22c55e)))
                .child(colored_box(rgb(0x3b82f6), 40.)),
        ))
        // L16: Flex Shrink
        .child(test_card(
            "L16",
            "Flex Shrink",
            "All shrink proportionally in narrow container",
            div()
                .flex()
                .w(px(120.))
                .h(px(60.))
                .bg(rgb(0x1a1a2e))
                .p_2()
                .overflow_hidden()
                .child(div().flex_shrink().w(px(80.)).h(px(40.)).bg(rgb(0xef4444)))
                .child(div().flex_shrink().w(px(80.)).h(px(40.)).bg(rgb(0x22c55e)))
                .child(div().flex_shrink().w(px(80.)).h(px(40.)).bg(rgb(0x3b82f6))),
        ))
        // L17: Flex Wrap
        .child(test_card(
            "L17",
            "Flex Wrap",
            "Wraps to multiple rows",
            div()
                .flex()
                .flex_wrap()
                .gap_2()
                .w(px(180.))
                .bg(rgb(0x1a1a2e))
                .p_2()
                .children((1..=10).map(|i| {
                    div()
                        .w(px(40.))
                        .h(px(40.))
                        .bg(rgb(0x6366f1))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_xs()
                        .child(format!("{}", i))
                })),
        ))
        // L18: Gap
        .child(test_card("L18", "Gap", "20px space between",
            div()
                .flex()
                .gap(px(20.))
                .bg(rgb(0x1a1a2e))
                .p_2()
                .child(colored_box(rgb(0x6366f1), 40.))
                .child(colored_box(rgb(0x6366f1), 40.))
                .child(colored_box(rgb(0x6366f1), 40.)),
        ))
        // L19: Nested Flex
        .child(test_card(
            "L19",
            "Nested Flex",
            "Correct nested layout",
            div()
                .flex()
                .flex_row()
                .gap_2()
                .bg(rgb(0x1a1a2e))
                .p_2()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(colored_box(rgb(0xef4444), 30.))
                        .child(colored_box(rgb(0xef4444), 30.)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .gap_2()
                                .child(colored_box(rgb(0x22c55e), 30.))
                                .child(colored_box(rgb(0x22c55e), 30.)),
                        )
                        .child(colored_box(rgb(0x3b82f6), 30.)),
                ),
        ))
}

// =============================================================================
// SIZING TESTS (S01-S06)
// =============================================================================

pub fn render_sizing_tests() -> impl IntoElement {
    test_grid()
        // S01: Fixed Width/Height
        .child(test_card("S01", "Fixed Width/Height", "Exactly 100x50",
            div()
                .w(px(100.))
                .h(px(50.))
                .bg(rgb(0x6366f1)),
        ))
        // S02: Percentage Width
        .child(test_card(
            "S02",
            "Percentage Width",
            "Half of parent width",
            div()
                .w(px(200.))
                .h(px(60.))
                .bg(rgb(0x1a1a2e))
                .p_2()
                .child(div().w_1_2().h_full().bg(rgb(0x22c55e))),
        ))
        // S03: Min Width
        .child(test_card("S03", "Min Width", "At least 100px wide",
            div()
                .flex()
                .bg(rgb(0x1a1a2e))
                .p_2()
                .child(
                    div()
                        .min_w(px(100.))
                        .h(px(40.))
                        .bg(rgb(0xf59e0b))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_xs()
                        .child("min-w"),
                ),
        ))
        // S04: Max Width
        .child(test_card("S04", "Max Width", "At most 100px wide",
            div()
                .flex()
                .bg(rgb(0x1a1a2e))
                .p_2()
                .child(
                    div()
                        .max_w(px(100.))
                        .h(px(40.))
                        .bg(rgb(0xec4899))
                        .overflow_hidden()
                        .flex()
                        .items_center()
                        .text_xs()
                        .child("This is very long text that should be clipped"),
                ),
        ))
        // S05: Size Full
        .child(test_card("S05", "Size Full", "Fills container",
            div()
                .w(px(150.))
                .h(px(80.))
                .bg(rgb(0x1a1a2e))
                .p_2()
                .child(div().size_full().bg(rgb(0x8b5cf6))),
        ))
        // S06: Aspect Ratio (not directly available, simulating with fixed sizes)
        .child(test_card(
            "S06",
            "Aspect Ratio",
            "200x100 (2:1 ratio)",
            div()
                .w(px(200.))
                .h(px(100.))
                .bg(rgb(0x06b6d4))
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .child("2:1"),
        ))
}

// =============================================================================
// SPACING TESTS (SP01-SP07)
// =============================================================================

pub fn render_spacing_tests() -> impl IntoElement {
    test_grid()
        // SP01: Padding All
        .child(test_card("SP01", "Padding All", "16px padding visible",
            div()
                .w(px(100.))
                .h(px(100.))
                .bg(rgb(0x1a1a2e))
                .p_4()
                .child(div().size_full().bg(rgb(0x6366f1))),
        ))
        // SP02: Padding Horizontal
        .child(test_card(
            "SP02",
            "Padding Horizontal",
            "Only left/right padding",
            div()
                .w(px(120.))
                .h(px(60.))
                .bg(rgb(0x1a1a2e))
                .px_4()
                .child(div().size_full().bg(rgb(0x22c55e))),
        ))
        // SP03: Padding Vertical
        .child(test_card(
            "SP03",
            "Padding Vertical",
            "Only top/bottom padding",
            div()
                .w(px(60.))
                .h(px(100.))
                .bg(rgb(0x1a1a2e))
                .py_4()
                .child(div().size_full().bg(rgb(0xef4444))),
        ))
        // SP04: Padding Individual
        .child(test_card(
            "SP04",
            "Padding Individual",
            "Different padding per side (t:4, r:8, b:12, l:16)",
            div()
                .w(px(120.))
                .h(px(100.))
                .bg(rgb(0x1a1a2e))
                .pt_1()
                .pr_2()
                .pb_3()
                .pl_4()
                .child(div().size_full().bg(rgb(0xf59e0b))),
        ))
        // SP05: Margin All
        .child(test_card("SP05", "Margin All", "16px margin visible",
            div()
                .w(px(120.))
                .h(px(120.))
                .bg(rgb(0x1a1a2e))
                .child(
                    div()
                        .w(px(60.))
                        .h(px(60.))
                        .bg(rgb(0x8b5cf6))
                        .m_4(),
                ),
        ))
        // SP06: Margin Auto (centering)
        .child(test_card(
            "SP06",
            "Margin Auto",
            "Horizontally centered",
            div()
                .w(px(200.))
                .h(px(80.))
                .bg(rgb(0x1a1a2e))
                .flex()
                .child(
                    div()
                        .w(px(60.))
                        .h(px(40.))
                        .bg(rgb(0xec4899))
                        .mx_auto(),
                ),
        ))
        // SP07: Negative Margin (overlap)
        .child(test_card(
            "SP07",
            "Negative Margin",
            "Elements overlap",
            div()
                .flex()
                .bg(rgb(0x1a1a2e))
                .p_4()
                .child(div().w(px(60.)).h(px(60.)).bg(rgb(0xef4444)))
                .child(
                    div()
                        .w(px(60.))
                        .h(px(60.))
                        .bg(rgb(0x22c55e))
                        .ml(px(-20.)),
                )
                .child(
                    div()
                        .w(px(60.))
                        .h(px(60.))
                        .bg(rgb(0x3b82f6))
                        .ml(px(-20.)),
                ),
        ))
}
