//! Visual attribute tests - Quad rendering (Q01-Q14)

use gpui::{div, px, rgb, Hsla, IntoElement, ParentElement, Styled};
use super::{test_card, test_grid, test_row, colored_box, labeled_box};

/// Render all quad tests
pub fn render_quad_tests() -> impl IntoElement {
    test_grid()
        // Q01: Solid Background
        .child(test_card("Q01", "Solid Background", "Red square visible",
            colored_box(rgb(0xff0000), 100.),
        ))
        // Q02: Background Colors
        .child(test_card(
            "Q02",
            "Background Colors",
            "All 6 colors distinct and correct",
            test_row()
                .child(labeled_box(rgb(0xff0000), "R"))
                .child(labeled_box(rgb(0x00ff00), "G"))
                .child(labeled_box(rgb(0x0000ff), "B"))
                .child(labeled_box(rgb(0xffff00), "Y"))
                .child(labeled_box(rgb(0x00ffff), "C"))
                .child(labeled_box(rgb(0xff00ff), "M")),
        ))
        // Q03: HSLA Colors
        .child(test_card(
            "Q03",
            "HSLA Colors",
            "Color wheel progression visible",
            test_row()
                .child(
                    div()
                        .w(px(40.))
                        .h(px(40.))
                        .bg(Hsla { h: 0.0, s: 0.8, l: 0.5, a: 1.0 }),
                )
                .child(
                    div()
                        .w(px(40.))
                        .h(px(40.))
                        .bg(Hsla { h: 0.166, s: 0.8, l: 0.5, a: 1.0 }),
                )
                .child(
                    div()
                        .w(px(40.))
                        .h(px(40.))
                        .bg(Hsla { h: 0.333, s: 0.8, l: 0.5, a: 1.0 }),
                )
                .child(
                    div()
                        .w(px(40.))
                        .h(px(40.))
                        .bg(Hsla { h: 0.5, s: 0.8, l: 0.5, a: 1.0 }),
                )
                .child(
                    div()
                        .w(px(40.))
                        .h(px(40.))
                        .bg(Hsla { h: 0.666, s: 0.8, l: 0.5, a: 1.0 }),
                )
                .child(
                    div()
                        .w(px(40.))
                        .h(px(40.))
                        .bg(Hsla { h: 0.833, s: 0.8, l: 0.5, a: 1.0 }),
                ),
        ))
        // Q04: Alpha/Opacity - Clear demonstration with labeled boxes
        .child(test_card(
            "Q04",
            "Alpha/Opacity",
            "Red boxes at 25%, 50%, 75%, 100% over striped bg",
            div()
                .flex()
                .flex_col()
                .gap(px(4.))
                // Row of opacity boxes over striped backgrounds
                .child(
                    div()
                        .flex()
                        .gap(px(8.))
                        // 25% opacity
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .items_center()
                                .child(
                                    div()
                                        .relative()
                                        .w(px(50.))
                                        .h(px(50.))
                                        // Striped background: alternating white/black
                                        .bg(rgb(0xFFFFFF))
                                        .child(
                                            div()
                                                .absolute()
                                                .top(px(0.))
                                                .left(px(0.))
                                                .w(px(25.))
                                                .h(px(50.))
                                                .bg(rgb(0x000000)),
                                        )
                                        // Red overlay at 25% opacity
                                        .child(
                                            div()
                                                .absolute()
                                                .top(px(5.))
                                                .left(px(5.))
                                                .w(px(40.))
                                                .h(px(40.))
                                                .bg(Hsla { h: 0.0, s: 1.0, l: 0.5, a: 0.25 }),
                                        ),
                                )
                                .child(div().text_sm().text_color(rgb(0xFFFFFF)).child("25%")),
                        )
                        // 50% opacity
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .items_center()
                                .child(
                                    div()
                                        .relative()
                                        .w(px(50.))
                                        .h(px(50.))
                                        .bg(rgb(0xFFFFFF))
                                        .child(
                                            div()
                                                .absolute()
                                                .top(px(0.))
                                                .left(px(0.))
                                                .w(px(25.))
                                                .h(px(50.))
                                                .bg(rgb(0x000000)),
                                        )
                                        .child(
                                            div()
                                                .absolute()
                                                .top(px(5.))
                                                .left(px(5.))
                                                .w(px(40.))
                                                .h(px(40.))
                                                .bg(Hsla { h: 0.0, s: 1.0, l: 0.5, a: 0.50 }),
                                        ),
                                )
                                .child(div().text_sm().text_color(rgb(0xFFFFFF)).child("50%")),
                        )
                        // 75% opacity
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .items_center()
                                .child(
                                    div()
                                        .relative()
                                        .w(px(50.))
                                        .h(px(50.))
                                        .bg(rgb(0xFFFFFF))
                                        .child(
                                            div()
                                                .absolute()
                                                .top(px(0.))
                                                .left(px(0.))
                                                .w(px(25.))
                                                .h(px(50.))
                                                .bg(rgb(0x000000)),
                                        )
                                        .child(
                                            div()
                                                .absolute()
                                                .top(px(5.))
                                                .left(px(5.))
                                                .w(px(40.))
                                                .h(px(40.))
                                                .bg(Hsla { h: 0.0, s: 1.0, l: 0.5, a: 0.75 }),
                                        ),
                                )
                                .child(div().text_sm().text_color(rgb(0xFFFFFF)).child("75%")),
                        )
                        // 100% opacity
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .items_center()
                                .child(
                                    div()
                                        .relative()
                                        .w(px(50.))
                                        .h(px(50.))
                                        .bg(rgb(0xFFFFFF))
                                        .child(
                                            div()
                                                .absolute()
                                                .top(px(0.))
                                                .left(px(0.))
                                                .w(px(25.))
                                                .h(px(50.))
                                                .bg(rgb(0x000000)),
                                        )
                                        .child(
                                            div()
                                                .absolute()
                                                .top(px(5.))
                                                .left(px(5.))
                                                .w(px(40.))
                                                .h(px(40.))
                                                .bg(Hsla { h: 0.0, s: 1.0, l: 0.5, a: 1.0 }),
                                        ),
                                )
                                .child(div().text_sm().text_color(rgb(0xFFFFFF)).child("100%")),
                        ),
                ),
        ))
        // Q05: Corner Radius - Uniform
        .child(test_card(
            "Q05",
            "Corner Radius - Uniform",
            "Uniformly rounded corners",
            div()
                .w(px(100.))
                .h(px(100.))
                .bg(rgb(0x6366f1))
                .rounded_lg(),
        ))
        // Q06: Corner Radius - Individual
        .child(test_card(
            "Q06",
            "Corner Radius - Individual",
            "Asymmetric rounding (top-left and bottom-right)",
            div()
                .w(px(100.))
                .h(px(100.))
                .bg(rgb(0x10b981))
                .rounded_tl_xl()
                .rounded_br_xl(),
        ))
        // Q07: Corner Radius - Pill
        .child(test_card(
            "Q07",
            "Corner Radius - Pill",
            "Pill/capsule shape",
            div()
                .w(px(150.))
                .h(px(50.))
                .bg(rgb(0xf59e0b))
                .rounded_full(),
        ))
        // Q08: Corner Radius - Circle
        .child(test_card("Q08", "Corner Radius - Circle", "Perfect circle",
            div()
                .w(px(80.))
                .h(px(80.))
                .bg(rgb(0xef4444))
                .rounded_full(),
        ))
        // Q09: Border - Solid
        .child(test_card(
            "Q09",
            "Border - Solid",
            "Visible border on all sides",
            div()
                .w(px(100.))
                .h(px(100.))
                .bg(rgb(0x1a1a2e))
                .border_2()
                .border_color(rgb(0x6366f1)),
        ))
        // Q10: Border - Per-Side Width
        .child(test_card(
            "Q10",
            "Border - Per-Side Width",
            "Visibly different thicknesses (1, 2, 4, 8 px)",
            div()
                .w(px(100.))
                .h(px(100.))
                .bg(rgb(0x1a1a2e))
                .border_t_1()
                .border_r_2()
                .border_b_4()
                .border_l_8()
                .border_color(rgb(0xf59e0b)),
        ))
        // Q11: Border - Color
        .child(test_card(
            "Q11",
            "Border - Color",
            "Border color distinct from background",
            div()
                .w(px(100.))
                .h(px(100.))
                .bg(rgb(0x3b82f6))
                .border_4()
                .border_color(rgb(0xfbbf24)),
        ))
        // Q12: Border + Radius
        .child(test_card(
            "Q12",
            "Border + Radius",
            "Border follows curve",
            div()
                .w(px(100.))
                .h(px(100.))
                .bg(rgb(0x1a1a2e))
                .border_4()
                .border_color(rgb(0xec4899))
                .rounded_xl(),
        ))
        // Q13: Content Masking
        .child(test_card(
            "Q13",
            "Content Masking",
            "Child clipped at parent bounds",
            div()
                .w(px(100.))
                .h(px(100.))
                .bg(rgb(0x1a1a2e))
                .overflow_hidden()
                .rounded_lg()
                .child(
                    div()
                        .w(px(150.))
                        .h(px(150.))
                        .bg(rgb(0x8b5cf6))
                        .mt(px(-25.))
                        .ml(px(-25.)),
                ),
        ))
        // Q14: Nested Quads
        .child(test_card(
            "Q14",
            "Nested Quads",
            "All 3 visible, proper z-order",
            div()
                .w(px(120.))
                .h(px(120.))
                .bg(rgb(0xef4444))
                .p_4()
                .child(
                    div()
                        .size_full()
                        .bg(rgb(0x22c55e))
                        .p_4()
                        .child(div().size_full().bg(rgb(0x3b82f6))),
                ),
        ))
}
