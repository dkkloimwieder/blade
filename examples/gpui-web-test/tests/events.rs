//! Event handling tests - Mouse (E01-E12), Keyboard (K01-K06), Scroll (SC01-SC03)

use gpui::{div, prelude::*, px, rgb, Context, IntoElement, MouseButton, ParentElement, Styled};
use crate::TestHarness;
use super::test_card;

/// Helper function to create a test grid
fn test_grid() -> gpui::Div {
    div().flex().flex_col().gap_4()
}

impl TestHarness {
    // =========================================================================
    // MOUSE EVENT TESTS (E01-E12)
    // =========================================================================

    pub fn render_mouse_event_tests(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let click_count = self.click_count;
        let double_click_count = self.double_click_count;
        let right_click_count = self.right_click_count;
        let mouse_pos = self.mouse_position;

        test_grid()
            // E01: Mouse Down
            .child(test_card("E01", "Mouse Down", "Color changes on press",
                div()
                    .id("e01-press")
                    .w(px(120.))
                    .h(px(60.))
                    .bg(rgb(0x6366f1))
                    .rounded_md()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .active(|style| style.bg(rgb(0xef4444)))
                    .child("Press me"),
            ))
            // E02: Mouse Up
            .child(test_card("E02", "Mouse Up", "Count increases on mouse up",
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .id("e02-release")
                            .w(px(120.))
                            .h(px(60.))
                            .bg(rgb(0x22c55e))
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|style| style.bg(rgb(0x16a34a)))
                            .on_mouse_up(MouseButton::Left, cx.listener(|this, _event, _window, cx| {
                                this.mouse_up_count += 1;
                                cx.notify();
                            }))
                            .child("Release me"),
                    )
                    .child(
                        div()
                            .text_xl()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child(format!("Up: {}", self.mouse_up_count)),
                    ),
            ))
            // E03: Click Counter
            .child(test_card("E03", "Click", "Count increases on click",
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
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _event, _window, cx| {
                                this.click_count += 1;
                                cx.notify();
                            }))
                            .child("Click me"),
                    )
                    .child(
                        div()
                            .text_xl()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child(format!("Count: {}", click_count)),
                    ),
            ))
            // E04: Double Click
            .child(test_card("E04", "Double Click", "Detects double-click",
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .id("e04-double")
                            .w(px(120.))
                            .h(px(60.))
                            .bg(rgb(0xf59e0b))
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|style| style.bg(rgb(0xfbbf24)))
                            .on_click(cx.listener(|this, event: &gpui::ClickEvent, _window, cx| {
                                if event.click_count() >= 2 {
                                    this.double_click_count += 1;
                                    cx.notify();
                                }
                            }))
                            .child("Double-click"),
                    )
                    .child(
                        div()
                            .text_xl()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child(format!("Double: {}", double_click_count)),
                    ),
            ))
            // E05: Right Click
            .child(test_card("E05", "Right Click", "Different from left-click",
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .w(px(120.))
                            .h(px(60.))
                            .bg(rgb(0xec4899))
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|style| style.bg(rgb(0xf472b6)))
                            .on_mouse_down(MouseButton::Right, cx.listener(|this, _event, _window, cx| {
                                this.right_click_count += 1;
                                cx.notify();
                            }))
                            .child("Right-click"),
                    )
                    .child(
                        div()
                            .text_xl()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child(format!("Right: {}", right_click_count)),
                    ),
            ))
            // E06: Mouse Move
            .child(test_card("E06", "Mouse Move", "Coordinates update on move",
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .w(px(200.))
                            .h(px(100.))
                            .bg(rgb(0x1a1a2e))
                            .border_2()
                            .border_color(rgb(0x6366f1))
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _window, cx| {
                                this.mouse_position = (f32::from(event.position.x), f32::from(event.position.y));
                                cx.notify();
                            }))
                            .child("Move mouse here"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .child(format!("X: {:.0}, Y: {:.0}", mouse_pos.0, mouse_pos.1)),
                    ),
            ))
            // E07: Mouse Enter (via hover style)
            .child(test_card("E07", "Mouse Enter", "Visual change on enter (via .hover())",
                div()
                    .w(px(150.))
                    .h(px(60.))
                    .bg(rgb(0x1a1a2e))
                    .border_2()
                    .border_color(rgb(0x22c55e))
                    .rounded_md()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(0x22c55e)))
                    .child("Hover to enter"),
            ))
            // E08: Mouse Leave (via hover style)
            .child(test_card("E08", "Mouse Leave", "Visual change on leave (via .hover())",
                div()
                    .w(px(150.))
                    .h(px(60.))
                    .bg(rgb(0xef4444))
                    .border_2()
                    .border_color(rgb(0xef4444))
                    .rounded_md()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgb(0xffffff))
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(0x1a1a2e)))
                    .child("Hover then leave"),
            ))
            // E09: Hover State
            .child(test_card("E09", "Hover State", "Automatic color change with .hover()",
                div()
                    .w(px(150.))
                    .h(px(60.))
                    .bg(rgb(0x3b82f6))
                    .rounded_md()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(0x60a5fa)))
                    .child("Hover me"),
            ))
            // E10: Mouse Down Outside (simplified - shows concept)
            .child(test_card("E10", "Mouse Down Outside", "Detects click outside (conceptual)",
                div()
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .w(px(100.))
                            .h(px(60.))
                            .bg(rgb(0x8b5cf6))
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child("Target"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x888888))
                            .child("(Requires on_mouse_down_out)"),
                    ),
            ))
            // E11: Capture vs Bubble (conceptual)
            .child(test_card("E11", "Event Propagation", "Nested elements respond to events",
                div()
                    .w(px(150.))
                    .h(px(100.))
                    .bg(rgb(0xef4444))
                    .rounded_md()
                    .p_2()
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(0xfca5a5)))
                    .child(
                        div()
                            .w_full()
                            .h_full()
                            .bg(rgb(0x22c55e))
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|style| style.bg(rgb(0x86efac)))
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .bg(rgb(0x3b82f6))
                                    .rounded_md()
                                    .cursor_pointer()
                                    .hover(|style| style.bg(rgb(0x93c5fd)))
                                    .child("Inner"),
                            ),
                    ),
            ))
            // E12: Prevent Default (conceptual)
            .child(test_card("E12", "Stop Propagation", "Inner doesn't bubble to outer",
                div()
                    .text_xs()
                    .text_color(rgb(0x888888))
                    .child("Use .stop_propagation() on events to prevent bubbling"),
            ))
    }

    // =========================================================================
    // KEYBOARD EVENT TESTS (K01-K06)
    // =========================================================================

    pub fn render_keyboard_tests(&mut self, _cx: &mut Context<Self>) -> impl IntoElement {
        let last_key = self.last_key.clone();
        let modifiers = self.active_modifiers.clone();
        let key_action = self.key_action_fired;
        let ctrl_s = self.ctrl_s_fired;

        test_grid()
            // K01: Key Down
            .child(test_card("K01", "Key Down", "Shows key name (use global keybindings)",
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x888888))
                            .child("Focus the window and press keys"),
                    )
                    .child(
                        div()
                            .px_4()
                            .py_2()
                            .bg(rgb(0x1a1a2e))
                            .rounded_md()
                            .text_lg()
                            .child(if last_key.is_empty() {
                                "No key pressed".to_string()
                            } else {
                                format!("Last key: {}", last_key)
                            }),
                    ),
            ))
            // K02: Key Up
            .child(test_card("K02", "Key Up", "Shows released state",
                div()
                    .text_xs()
                    .text_color(rgb(0x888888))
                    .child("Key up events are captured similarly to key down"),
            ))
            // K03: Modifier Keys
            .child(test_card("K03", "Modifier Keys", "Shows held modifiers",
                div()
                    .px_4()
                    .py_2()
                    .bg(rgb(0x1a1a2e))
                    .rounded_md()
                    .child(if modifiers.is_empty() {
                        "No modifiers".to_string()
                    } else {
                        format!("Active: {}", modifiers)
                    }),
            ))
            // K04: Key Binding
            .child(test_card("K04", "Key Binding", "Press 'T' to trigger action",
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .px_4()
                            .py_2()
                            .bg(if key_action { rgb(0x22c55e) } else { rgb(0x1a1a2e) })
                            .rounded_md()
                            .child(if key_action { "T key fired!" } else { "Press 'T'" }),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x888888))
                            .child("(Bound via cx.bind_keys)"),
                    ),
            ))
            // K05: Key Context
            .child(test_card("K05", "Key Context", "Different actions per context",
                div()
                    .text_xs()
                    .text_color(rgb(0x888888))
                    .child("Key contexts allow different keybindings in different UI areas"),
            ))
            // K06: Shortcut Combo
            .child(test_card("K06", "Shortcut Combo", "Press Ctrl+S",
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .px_4()
                            .py_2()
                            .bg(if ctrl_s { rgb(0x6366f1) } else { rgb(0x1a1a2e) })
                            .rounded_md()
                            .child(if ctrl_s { "Ctrl+S fired!" } else { "Press Ctrl+S" }),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x888888))
                            .child("(Bound via cx.bind_keys)"),
                    ),
            ))
    }

    // =========================================================================
    // SCROLL/WHEEL TESTS (SC01-SC03)
    // =========================================================================

    pub fn render_scroll_tests(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let scroll_delta = self.scroll_delta;
        let scroll_pos = self.scroll_position;

        test_grid()
            // SC01: Scroll Wheel
            .child(test_card("SC01", "Scroll Wheel", "Delta values update on scroll",
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .w(px(200.))
                            .h(px(100.))
                            .bg(rgb(0x1a1a2e))
                            .border_2()
                            .border_color(rgb(0x6366f1))
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .on_scroll_wheel(cx.listener(|this, event: &gpui::ScrollWheelEvent, _window, cx| {
                                let delta = event.delta.pixel_delta(px(1.));
                                this.scroll_delta = (f32::from(delta.x), f32::from(delta.y));
                                cx.notify();
                            }))
                            .child("Scroll here"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .child(format!("Delta: ({:.1}, {:.1})", scroll_delta.0, scroll_delta.1)),
                    ),
            ))
            // SC02: Scroll Container
            .child(test_card("SC02", "Scroll Container", "Content is scrollable",
                div()
                    .id("scroll-container")
                    .w(px(200.))
                    .h(px(150.))
                    .overflow_scroll()
                    .bg(rgb(0x1a1a2e))
                    .rounded_md()
                    .p_2()
                    .children((1..=20).map(|i| {
                        div()
                            .py_1()
                            .border_b_1()
                            .border_color(rgb(0x333355))
                            .child(format!("Item {}", i))
                    })),
            ))
            // SC03: Scroll Position
            .child(test_card("SC03", "Scroll Position", "Position updates on scroll",
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .id("scroll-position-container")
                            .w(px(200.))
                            .h(px(100.))
                            .overflow_scroll()
                            .bg(rgb(0x1a1a2e))
                            .rounded_md()
                            .p_2()
                            .on_scroll_wheel(cx.listener(|this, event: &gpui::ScrollWheelEvent, _window, cx| {
                                this.scroll_position += f32::from(event.delta.pixel_delta(px(1.)).y);
                                this.scroll_position = this.scroll_position.max(0.0);
                                cx.notify();
                            }))
                            .children((1..=15).map(|i| {
                                div()
                                    .py_1()
                                    .child(format!("Row {}", i))
                            })),
                    )
                    .child(
                        div()
                            .text_sm()
                            .child(format!("Scroll: {:.0}px", scroll_pos)),
                    ),
            ))
    }
}
