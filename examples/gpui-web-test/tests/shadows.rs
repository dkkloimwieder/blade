//! Shadow tests (SH01-SH05) - Implemented in GPUI WASM

use gpui::{div, hsla, point, px, rgb, BoxShadow, IntoElement, ParentElement, Styled};
use super::test_card;

/// Helper function to create a test grid
fn test_grid() -> gpui::Div {
    div().flex().flex_col().gap_4()
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

fn shadow_box_multi() -> gpui::Div {
    div()
        .p_8()
        .bg(rgb(0xffffff))
        .child(
            div()
                .w(px(100.))
                .h(px(80.))
                .bg(rgb(0x4a4a8e))
                .rounded_md()
                .shadow(vec![
                    // Outer soft shadow
                    BoxShadow {
                        color: hsla(0.0, 0.0, 0.0, 0.15),
                        offset: point(px(0.), px(10.)),
                        blur_radius: px(20.),
                        spread_radius: px(0.),
                    },
                    // Inner sharp shadow
                    BoxShadow {
                        color: hsla(0.0, 0.0, 0.0, 0.3),
                        offset: point(px(0.), px(2.)),
                        blur_radius: px(4.),
                        spread_radius: px(0.),
                    },
                ])
        )
}

fn shadow_box_glow() -> gpui::Div {
    div()
        .p_8()
        .bg(rgb(0x1a1a2e))
        .child(
            div()
                .w(px(100.))
                .h(px(80.))
                .bg(rgb(0x4a4a8e))
                .rounded_md()
                .shadow(vec![
                    // Colored glow effect
                    BoxShadow {
                        color: hsla(0.7, 0.8, 0.5, 0.6),
                        offset: point(px(0.), px(0.)),
                        blur_radius: px(20.),
                        spread_radius: px(2.),
                    },
                ])
        )
}

// =============================================================================
// SHADOW TESTS (SH01-SH05) - IMPLEMENTED
// =============================================================================

pub fn render_shadows() -> impl IntoElement {
    test_grid()
        // SH01: Basic Shadow
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
                        .child("Soft shadow around box"),
                ),
        ))
        // SH02: Shadow Color
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
        // SH03: Shadow Offset
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
        // SH04: Shadow Blur
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
        // SH05: Multiple Shadows
        .child(test_card("SH05", "Multiple Shadows", "Stacked shadows",
            div()
                .flex()
                .gap_4()
                .child(shadow_box_multi())
                .child(shadow_box_glow())
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x888888))
                        .flex()
                        .items_center()
                        .child("Layered shadow + glow effect"),
                ),
        ))
}
