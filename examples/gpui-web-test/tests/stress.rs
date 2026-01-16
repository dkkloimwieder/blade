//! Stress tests (ST01-ST08) - Performance testing

use gpui::{div, prelude::*, px, rgb, Hsla, Context, IntoElement, ParentElement, Styled};
use crate::TestHarness;
use super::test_card;

/// Helper function to create a test grid
fn test_grid() -> gpui::Div {
    div().flex().flex_col().gap_4()
}

impl TestHarness {
    pub fn render_stress_tests(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let counter = self.stress_counter;

        test_grid()
            .child(
                div()
                    .p_4()
                    .bg(rgb(0x1a1a2e))
                    .rounded_lg()
                    .mb_4()
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x888888))
                            .child("Performance tests to verify GPUI WASM handles load gracefully"),
                    ),
            )
            // ST01: 100 Quads
            .child(test_card("ST01", "100 Quads", "All render, smooth interaction",
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .w(px(320.))
                    .children((0..100).map(|i| {
                        let hue = (i as f32 / 100.0) * 360.0 / 360.0;
                        div()
                            .w(px(28.))
                            .h(px(28.))
                            .bg(Hsla { h: hue, s: 0.7, l: 0.5, a: 1.0 })
                    })),
            ))
            // ST02: 500 Quads
            .child(test_card("ST02", "500 Quads", "All render, acceptable FPS",
                div()
                    .flex()
                    .flex_wrap()
                    .gap_px()
                    .w(px(400.))
                    .max_h(px(200.))
                    .overflow_hidden()
                    .children((0..500).map(|i| {
                        let hue = (i as f32 / 500.0) * 360.0 / 360.0;
                        div()
                            .w(px(14.))
                            .h(px(14.))
                            .bg(Hsla { h: hue, s: 0.6, l: 0.5, a: 1.0 })
                    })),
            ))
            // ST03: 1000 Quads
            .child(test_card("ST03", "1000 Quads", "All render, measure FPS",
                div()
                    .flex()
                    .flex_wrap()
                    .gap_px()
                    .w(px(500.))
                    .max_h(px(200.))
                    .overflow_hidden()
                    .children((0..1000).map(|i| {
                        let hue = (i as f32 / 1000.0) * 360.0 / 360.0;
                        div()
                            .w(px(12.))
                            .h(px(12.))
                            .bg(Hsla { h: hue, s: 0.5, l: 0.5, a: 1.0 })
                    })),
            ))
            // ST04: Nested Depth 20
            .child(test_card("ST04", "Nested Depth 20", "All visible, no overflow",
                render_nested_divs(20),
            ))
            // ST05: 100 Text Elements
            .child(test_card("ST05", "100 Text Elements", "All readable",
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .w(px(400.))
                    .max_h(px(150.))
                    .overflow_hidden()
                    .text_xs()
                    .children((1..=100).map(|i| {
                        div()
                            .px_1()
                            .bg(rgb(0x1a1a2e))
                            .rounded_sm()
                            .child(format!("T{}", i))
                    })),
            ))
            // ST06: Rapid State Changes
            .child(test_card("ST06", "Rapid State Changes", "Counter increments smoothly",
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .w(px(100.))
                            .h(px(60.))
                            .bg(rgb(0x6366f1))
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|style| style.bg(rgb(0x818cf8)))
                            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _event, _window, cx| {
                                this.stress_counter += 1;
                                cx.notify();
                            }))
                            .child("Click fast"),
                    )
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child(format!("{}", counter)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x888888))
                            .child("(Click rapidly to stress state updates)"),
                    ),
            ))
            // ST07: Many Hover Targets
            .child(test_card("ST07", "Many Hover Targets", "Hover responds quickly",
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .w(px(350.))
                    .children((0..100).map(|i| {
                        let hue = (i as f32 / 100.0) * 360.0 / 360.0;
                        div()
                            .w(px(30.))
                            .h(px(30.))
                            .bg(Hsla { h: hue, s: 0.5, l: 0.4, a: 1.0 })
                            .rounded_sm()
                            .cursor_pointer()
                            .hover(|style| style.bg(Hsla { h: hue, s: 0.8, l: 0.6, a: 1.0 }))
                    })),
            ))
            // ST08: Large Text Block
            .child(test_card("ST08", "Large Text Block", "Renders without hang",
                div()
                    .id("large-text-scroll")
                    .w(px(400.))
                    .max_h(px(150.))
                    .overflow_scroll()
                    .bg(rgb(0x1a1a2e))
                    .rounded_md()
                    .p_2()
                    .text_xs()
                    .child(generate_large_text()),
            ))
    }
}

/// Generate nested divs to depth N
fn render_nested_divs(depth: usize) -> impl IntoElement {
    fn build_nested(remaining: usize, hue: f32) -> gpui::Div {
        if remaining == 0 {
            div()
                .w(px(20.))
                .h(px(20.))
                .bg(Hsla { h: hue, s: 0.8, l: 0.5, a: 1.0 })
                .rounded_sm()
        } else {
            let next_hue = (hue + 0.05) % 1.0;
            div()
                .p_1()
                .bg(Hsla { h: hue, s: 0.3, l: 0.2, a: 1.0 })
                .rounded_sm()
                .child(build_nested(remaining - 1, next_hue))
        }
    }

    build_nested(depth, 0.0)
}

/// Generate a large block of text
fn generate_large_text() -> String {
    let words = [
        "Lorem", "ipsum", "dolor", "sit", "amet", "consectetur", "adipiscing", "elit",
        "sed", "do", "eiusmod", "tempor", "incididunt", "ut", "labore", "et", "dolore",
        "magna", "aliqua", "Ut", "enim", "ad", "minim", "veniam", "quis", "nostrud",
        "exercitation", "ullamco", "laboris", "nisi", "aliquip", "ex", "ea", "commodo",
        "consequat", "Duis", "aute", "irure", "in", "reprehenderit", "voluptate",
    ];

    let mut text = String::with_capacity(10000);
    for i in 0..1500 {
        text.push_str(words[i % words.len()]);
        text.push(' ');
        if i % 15 == 14 {
            text.push('\n');
        }
    }
    text
}
