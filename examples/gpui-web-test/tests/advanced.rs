//! Advanced interaction tests - Drag/Drop (D01-D05), Focus (F01-F04), Tooltips (TT01-TT03), Sprites (I01-I04)

use gpui::{div, prelude::*, px, rgb, Context, IntoElement, ParentElement, Styled};
use crate::TestHarness;
use super::test_card;

/// Helper function to create a test grid
fn test_grid() -> gpui::Div {
    div().flex().flex_col().gap_4()
}

impl TestHarness {
    // =========================================================================
    // DRAG AND DROP TESTS (D01-D05)
    // =========================================================================

    pub fn render_drag_drop_tests(&mut self, _cx: &mut Context<Self>) -> impl IntoElement {
        let drag_pos = self.drag_position;
        let drop_received = self.drop_received.clone();

        test_grid()
            // D01: Basic Drag
            .child(test_card("D01", "Basic Drag", "Follows cursor while dragging",
                div()
                    .flex()
                    .gap_4()
                    .child(
                        div()
                            .id("drag-me")
                            .w(px(80.))
                            .h(px(60.))
                            .bg(rgb(0x6366f1))
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_grab()
                            .active(|style| style.cursor_grabbing())
                            .child("Drag me"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x888888))
                            .child("(Use .on_drag() for full drag support)"),
                    ),
            ))
            // D02: Drag Preview
            .child(test_card("D02", "Drag Preview", "Preview renders during drag",
                div()
                    .text_xs()
                    .text_color(rgb(0x888888))
                    .child("Drag preview is rendered via the closure in on_drag()"),
            ))
            // D03: Drop Target
            .child(test_card("D03", "Drop Target", "Visual feedback on drop",
                div()
                    .flex()
                    .gap_4()
                    .child(
                        div()
                            .w(px(80.))
                            .h(px(60.))
                            .bg(rgb(0x22c55e))
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_grab()
                            .child("Source"),
                    )
                    .child(
                        div()
                            .text_2xl()
                            .text_color(rgb(0x888888))
                            .child("→"),
                    )
                    .child(
                        div()
                            .w(px(120.))
                            .h(px(80.))
                            .bg(rgb(0x1a1a2e))
                            .border_2()
                            .border_dashed()
                            .border_color(rgb(0x6366f1))
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(if drop_received.is_some() {
                                "Dropped!"
                            } else {
                                "Drop here"
                            }),
                    ),
            ))
            // D04: Drop Rejection
            .child(test_card("D04", "Drop Rejection", "Rejection indicator",
                div()
                    .text_xs()
                    .text_color(rgb(0x888888))
                    .child("Use type checking in on_drop() to accept/reject"),
            ))
            // D05: Drag Move
            .child(test_card("D05", "Drag Position", "Track drag position",
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .w(px(150.))
                            .h(px(80.))
                            .bg(rgb(0x1a1a2e))
                            .border_2()
                            .border_color(rgb(0xf59e0b))
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child("Drag area"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .child(if let Some((x, y)) = drag_pos {
                                format!("Pos: ({:.0}, {:.0})", x, y)
                            } else {
                                "No drag".to_string()
                            }),
                    ),
            ))
    }

    // =========================================================================
    // FOCUS TESTS (F01-F04)
    // =========================================================================

    pub fn render_focus_tests(&mut self, _cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focused_element;

        test_grid()
            // F01: Focus State
            .child(test_card("F01", "Focus State", "Ring visible when focused",
                div()
                    .w(px(150.))
                    .h(px(50.))
                    .bg(rgb(0x1a1a2e))
                    .rounded_md()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .border_2()
                    .border_color(rgb(0x333355))
                    .focus(|style| style.border_color(rgb(0x6366f1)).shadow_md())
                    .child("Click to focus"),
            ))
            // F02: Tab Navigation
            .child(test_card("F02", "Tab Navigation", "Tab cycles through elements",
                div()
                    .flex()
                    .gap_2()
                    .children((1..=4).map(|i| {
                        let is_focused = focused == Some(i);
                        div()
                            .w(px(60.))
                            .h(px(40.))
                            .bg(if is_focused { rgb(0x6366f1) } else { rgb(0x1a1a2e) })
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .border_2()
                            .border_color(if is_focused { rgb(0x818cf8) } else { rgb(0x333355) })
                            .child(format!("{}", i))
                    })),
            ))
            // F03: Tab Index
            .child(test_card("F03", "Tab Index", "Custom tab order",
                div()
                    .text_xs()
                    .text_color(rgb(0x888888))
                    .child("Use .tab_index() on FocusHandle"),
            ))
            // F04: Focus In/Out
            .child(test_card("F04", "Focus In/Out", "Events fire correctly",
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .w(px(100.))
                            .h(px(50.))
                            .bg(rgb(0x1a1a2e))
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .border_2()
                            .border_color(rgb(0x333355))
                            .focus(|style| style.bg(rgb(0x22c55e)).border_color(rgb(0x22c55e)))
                            .child("Focus me"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x888888))
                            .child("(Use on_focus_in/on_focus_out listeners)"),
                    ),
            ))
    }
}

// =============================================================================
// TOOLTIP TESTS (TT01-TT03)
// =============================================================================

pub fn render_tooltip_tests() -> impl IntoElement {
    test_grid()
        // TT01: Basic Tooltip
        .child(test_card("TT01", "Basic Tooltip", "Tooltip appears on hover",
            div()
                .w(px(150.))
                .h(px(50.))
                .bg(rgb(0x6366f1))
                .rounded_md()
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(|style| style.bg(rgb(0x818cf8)))
                .child("Hover for tooltip"),
        ))
        // TT02: Tooltip Delay
        .child(test_card("TT02", "Tooltip Delay", "Respects delay timing",
            div()
                .text_xs()
                .text_color(rgb(0x888888))
                .child("Use .tooltip() with custom delay configuration"),
        ))
        // TT03: Hoverable Tooltip
        .child(test_card("TT03", "Hoverable Tooltip", "Tooltip stays while hovered",
            div()
                .text_xs()
                .text_color(rgb(0x888888))
                .child("Configure tooltip to be interactive/hoverable"),
        ))
}

// =============================================================================
// SPRITE TESTS (I01-I04)
// =============================================================================

pub fn render_sprite_tests() -> impl IntoElement {
    test_grid()
        // I01: Monochrome Sprite
        .child(test_card("I01", "Monochrome Sprite", "Renders with tint color",
            div()
                .flex()
                .items_center()
                .gap_4()
                .child(
                    // Simulated icon using Unicode
                    div()
                        .w(px(48.))
                        .h(px(48.))
                        .bg(rgb(0x1a1a2e))
                        .rounded_md()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_2xl()
                        .text_color(rgb(0x6366f1))
                        .child("★"),
                )
                .child(
                    div()
                        .w(px(48.))
                        .h(px(48.))
                        .bg(rgb(0x1a1a2e))
                        .rounded_md()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_2xl()
                        .text_color(rgb(0x22c55e))
                        .child("★"),
                )
                .child(
                    div()
                        .w(px(48.))
                        .h(px(48.))
                        .bg(rgb(0x1a1a2e))
                        .rounded_md()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_2xl()
                        .text_color(rgb(0xef4444))
                        .child("★"),
                ),
        ))
        // I02: Polychrome Sprite
        .child(test_card("I02", "Polychrome Sprite", "Colors preserved",
            div()
                .text_xs()
                .text_color(rgb(0x888888))
                .child("Use img() or svg() element for polychrome images"),
        ))
        // I03: Sprite Opacity
        .child(test_card("I03", "Sprite Opacity", "Transparency visible",
            div()
                .flex()
                .gap_4()
                .child(
                    div()
                        .w(px(48.))
                        .h(px(48.))
                        .bg(rgb(0x1a1a2e))
                        .rounded_md()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_2xl()
                        .text_color(rgb(0xfbbf24))
                        .opacity(0.25)
                        .child("●"),
                )
                .child(
                    div()
                        .w(px(48.))
                        .h(px(48.))
                        .bg(rgb(0x1a1a2e))
                        .rounded_md()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_2xl()
                        .text_color(rgb(0xfbbf24))
                        .opacity(0.5)
                        .child("●"),
                )
                .child(
                    div()
                        .w(px(48.))
                        .h(px(48.))
                        .bg(rgb(0x1a1a2e))
                        .rounded_md()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_2xl()
                        .text_color(rgb(0xfbbf24))
                        .opacity(0.75)
                        .child("●"),
                )
                .child(
                    div()
                        .w(px(48.))
                        .h(px(48.))
                        .bg(rgb(0x1a1a2e))
                        .rounded_md()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_2xl()
                        .text_color(rgb(0xfbbf24))
                        .child("●"),
                ),
        ))
        // I04: Sprite Grayscale
        .child(test_card("I04", "Sprite Grayscale", "Desaturated",
            div()
                .text_xs()
                .text_color(rgb(0x888888))
                .child("Grayscale filter not yet available in GPUI WASM"),
        ))
}
