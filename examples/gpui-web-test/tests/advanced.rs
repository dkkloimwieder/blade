//! Advanced interaction tests - Drag/Drop (D01-D05), Focus (F01-F04), Tooltips (TT01-TT03), Sprites (I01-I04)

use gpui::{div, img, prelude::*, px, rgb, Context, IntoElement, ParentElement, Pixels, Point, Render, RenderImage, Styled, StyledImage, Window};
use crate::TestHarness;
use super::test_card;
use std::sync::Arc;
use image::{RgbaImage, Frame};
use smallvec::SmallVec;

// =============================================================================
// DRAG ITEM - Data and Preview
// =============================================================================

/// Data passed during drag operations
#[derive(Clone, Copy)]
pub struct DragItem {
    pub id: usize,
    pub color: u32,
    pub label: &'static str,
    pub position: Point<Pixels>,
}

impl DragItem {
    pub fn new(id: usize, color: u32, label: &'static str) -> Self {
        Self {
            id,
            color,
            label,
            position: Point::default(),
        }
    }

    pub fn with_position(mut self, position: Point<Pixels>) -> Self {
        self.position = position;
        self
    }
}

/// Render the drag preview that follows the cursor
impl Render for DragItem {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let w = px(80.);
        let h = px(40.);

        // Position the preview centered on cursor
        div()
            .pl(self.position.x - w / 2.)
            .pt(self.position.y - h / 2.)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(w)
                    .h(h)
                    .bg(rgb(self.color))
                    .opacity(0.85)
                    .text_color(rgb(0xffffff))
                    .text_sm()
                    .rounded_md()
                    .child(self.label),
            )
    }
}

/// Custom drag preview item - demonstrates different preview style
#[derive(Clone, Copy)]
pub struct FancyDragItem {
    pub position: Point<Pixels>,
}

impl FancyDragItem {
    pub fn new() -> Self {
        Self { position: Point::default() }
    }

    pub fn with_position(mut self, position: Point<Pixels>) -> Self {
        self.position = position;
        self
    }
}

/// Fancy preview with icon and styled appearance
impl Render for FancyDragItem {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let w = px(120.);
        let h = px(50.);

        div()
            .pl(self.position.x - w / 2.)
            .pt(self.position.y - h / 2.)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .w(w)
                    .h(h)
                    .bg(rgb(0x8b5cf6))
                    .opacity(0.9)
                    .text_color(rgb(0xffffff))
                    .text_sm()
                    .rounded_lg()
                    .border_2()
                    .border_color(rgb(0xffffff))
                    .px_3()
                    .child(
                        div()
                            .text_xl()
                            .child("✦")
                    )
                    .child("Custom!"),
            )
    }
}

/// A different drag type that won't be accepted by DragItem drop zones
#[derive(Clone, Copy)]
pub struct RedDragItem {
    pub position: Point<Pixels>,
}

impl RedDragItem {
    pub fn new() -> Self {
        Self { position: Point::default() }
    }

    pub fn with_position(mut self, position: Point<Pixels>) -> Self {
        self.position = position;
        self
    }
}

impl Render for RedDragItem {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let w = px(80.);
        let h = px(40.);

        div()
            .pl(self.position.x - w / 2.)
            .pt(self.position.y - h / 2.)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(w)
                    .h(h)
                    .bg(rgb(0xef4444))
                    .opacity(0.85)
                    .text_color(rgb(0xffffff))
                    .text_sm()
                    .rounded_md()
                    .child("Red"),
            )
    }
}

/// Helper function to create a test grid
fn test_grid() -> gpui::Div {
    div().flex().flex_col().gap_4()
}

impl TestHarness {
    // =========================================================================
    // DRAG AND DROP TESTS (D01-D05)
    // =========================================================================

    pub fn render_drag_drop_tests(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let drag_pos = self.drag_position;
        let drop_received = self.drop_received.clone();

        // D01 drag item
        let d01_item = DragItem::new(1, 0x6366f1, "Drag me");
        // D03 drag item (green source)
        let d03_item = DragItem::new(3, 0x22c55e, "Source");
        // D05 drag item (orange)
        let d05_item = DragItem::new(5, 0xf59e0b, "Track");

        test_grid()
            // D01: Basic Drag
            .child(test_card("D01", "Basic Drag", "Follows cursor while dragging",
                div()
                    .flex()
                    .gap_4()
                    .items_center()
                    .child(
                        div()
                            .id("d01-drag")
                            .w(px(80.))
                            .h(px(60.))
                            .bg(rgb(0x6366f1))
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_grab()
                            .active(|style| style.cursor_grabbing())
                            .child("Drag me")
                            .on_drag(d01_item, |data: &DragItem, position, _, cx| {
                                cx.new(|_| data.with_position(position))
                            }),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x888888))
                            .child("Drag to see preview follow cursor"),
                    ),
            ))
            // D02: Drag Preview - demonstrates custom styled preview
            .child(test_card("D02", "Drag Preview", "Custom preview with icon and border",
                div()
                    .flex()
                    .gap_4()
                    .items_center()
                    .child(
                        div()
                            .id("d02-fancy")
                            .w(px(100.))
                            .h(px(50.))
                            .bg(rgb(0x8b5cf6))
                            .rounded_lg()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_grab()
                            .active(|style| style.cursor_grabbing())
                            .child("Fancy drag")
                            .on_drag(FancyDragItem::new(), |data: &FancyDragItem, position, _, cx| {
                                cx.new(|_| data.with_position(position))
                            }),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x888888))
                            .child("Drag to see custom preview with icon"),
                    ),
            ))
            // D03: Drop Target
            .child(test_card("D03", "Drop Target", "Visual feedback on drop",
                div()
                    .flex()
                    .gap_4()
                    .items_center()
                    .child(
                        div()
                            .id("d03-source")
                            .w(px(80.))
                            .h(px(60.))
                            .bg(rgb(0x22c55e))
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_grab()
                            .active(|style| style.cursor_grabbing())
                            .child("Source")
                            .on_drag(d03_item, |data: &DragItem, position, _, cx| {
                                cx.new(|_| data.with_position(position))
                            }),
                    )
                    .child(
                        div()
                            .text_2xl()
                            .text_color(rgb(0x888888))
                            .child("→"),
                    )
                    .child(
                        div()
                            .id("d03-target")
                            .w(px(120.))
                            .h(px(80.))
                            .bg(if drop_received.is_some() { rgb(0x22c55e) } else { rgb(0x1a1a2e) })
                            .border_2()
                            .border_dashed()
                            .border_color(if drop_received.is_some() { rgb(0x22c55e) } else { rgb(0x6366f1) })
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .on_drop(cx.listener(|this, data: &DragItem, _window, cx| {
                                this.drop_received = Some(data.label.to_string());
                                cx.notify();
                            }))
                            .child(if let Some(ref label) = drop_received {
                                format!("Got: {}", label)
                            } else {
                                "Drop here".to_string()
                            }),
                    ),
            ))
            // D04: Drop Rejection - Type-based acceptance
            .child(test_card("D04", "Drop Rejection", "Red item rejected by green zone",
                div()
                    .flex()
                    .gap_4()
                    .items_center()
                    .child(
                        div()
                            .id("d04-red-source")
                            .w(px(60.))
                            .h(px(50.))
                            .bg(rgb(0xef4444))
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_grab()
                            .active(|style| style.cursor_grabbing())
                            .text_sm()
                            .child("Red")
                            .on_drag(RedDragItem::new(), |data: &RedDragItem, position, _, cx| {
                                cx.new(|_| data.with_position(position))
                            }),
                    )
                    .child(
                        div()
                            .text_xl()
                            .text_color(rgb(0x888888))
                            .child("→"),
                    )
                    .child(
                        div()
                            .id("d04-green-target")
                            .w(px(100.))
                            .h(px(60.))
                            .bg(rgb(0x1a1a2e))
                            .border_2()
                            .border_dashed()
                            .border_color(rgb(0x22c55e))
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_sm()
                            // This only accepts DragItem, NOT RedDragItem
                            .on_drop(cx.listener(|this, _data: &DragItem, _window, cx| {
                                this.drop_received = Some("Green accepts!".to_string());
                                cx.notify();
                            }))
                            .child("Green zone"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x888888))
                            .child("Red won't drop here (wrong type)"),
                    ),
            ))
            // D05: Drag Move - Position tracking
            .child(test_card("D05", "Drag Position", "Track drag position",
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .id("d05-drag")
                            .w(px(80.))
                            .h(px(50.))
                            .bg(rgb(0xf59e0b))
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_grab()
                            .active(|style| style.cursor_grabbing())
                            .child("Track")
                            .on_drag(d05_item, |data: &DragItem, position, _, cx| {
                                cx.new(|_| data.with_position(position))
                            })
                            .on_drag_move(cx.listener(|this, event: &gpui::DragMoveEvent<DragItem>, _window, cx| {
                                let pos = event.event.position;
                                this.drag_position = Some((f32::from(pos.x), f32::from(pos.y)));
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .text_sm()
                            .child(if let Some((x, y)) = drag_pos {
                                format!("Pos: ({:.0}, {:.0})", x, y)
                            } else {
                                "Drag to track position".to_string()
                            }),
                    ),
            ))
    }

    // =========================================================================
    // FOCUS TESTS (F01-F04)
    // =========================================================================

    pub fn render_focus_tests(&mut self, _cx: &mut Context<Self>) -> impl IntoElement {
        test_grid()
            // F01: Focus State
            .child(test_card("F01", "Focus State", "Ring visible when focused",
                div()
                    .id("f01-focus")
                    .track_focus(&self.f01_focus)
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
                    .focus(|style| style.border_color(rgb(0x6366f1)))
                    .child("Click to focus"),
            ))
            // F02: Tab Navigation
            .child(test_card("F02", "Tab Navigation", "Tab cycles through elements",
                div()
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .id("f02-tab-1")
                            .track_focus(&self.f02_focus_1)
                            .w(px(60.))
                            .h(px(40.))
                            .bg(rgb(0x1a1a2e))
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .border_2()
                            .border_color(rgb(0x333355))
                            .focus(|style| style.bg(rgb(0x6366f1)).border_color(rgb(0x818cf8)))
                            .child("1"),
                    )
                    .child(
                        div()
                            .id("f02-tab-2")
                            .track_focus(&self.f02_focus_2)
                            .w(px(60.))
                            .h(px(40.))
                            .bg(rgb(0x1a1a2e))
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .border_2()
                            .border_color(rgb(0x333355))
                            .focus(|style| style.bg(rgb(0x6366f1)).border_color(rgb(0x818cf8)))
                            .child("2"),
                    )
                    .child(
                        div()
                            .id("f02-tab-3")
                            .track_focus(&self.f02_focus_3)
                            .w(px(60.))
                            .h(px(40.))
                            .bg(rgb(0x1a1a2e))
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .border_2()
                            .border_color(rgb(0x333355))
                            .focus(|style| style.bg(rgb(0x6366f1)).border_color(rgb(0x818cf8)))
                            .child("3"),
                    )
                    .child(
                        div()
                            .id("f02-tab-4")
                            .track_focus(&self.f02_focus_4)
                            .w(px(60.))
                            .h(px(40.))
                            .bg(rgb(0x1a1a2e))
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .border_2()
                            .border_color(rgb(0x333355))
                            .focus(|style| style.bg(rgb(0x6366f1)).border_color(rgb(0x818cf8)))
                            .child("4"),
                    ),
            ))
            // F03: Tab Index - NOT IMPLEMENTED
            // Note: tab_index requires FocusHandle methods not exposed in this API
            .child(test_card("F03", "Tab Index", "NOT IMPLEMENTED - requires FocusHandle.set_tab_index()",
                div()
                    .text_xs()
                    .text_color(rgb(0x888888))
                    .child("Tab index ordering not yet available in GPUI WASM"),
            ))
            // F04: Focus In/Out - Shows focus state text
            .child(test_card("F04", "Focus In/Out", "Text shows focus/blur events",
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .id("f04-focus")
                            .track_focus(&self.f04_focus)
                            .w(px(150.))
                            .h(px(50.))
                            .bg(rgb(0x1a1a2e))
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .border_2()
                            .border_color(rgb(0x333355))
                            .focus(|style| style.bg(rgb(0x22c55e)).border_color(rgb(0x22c55e)))
                            .child("Click me"),
                    )
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .bg(rgb(0x1a1a2e))
                            .rounded_md()
                            .child(if self.f04_focus_event.is_empty() {
                                "Event: (none)".to_string()
                            } else {
                                format!("Event: {}", self.f04_focus_event)
                            }),
                    ),
            ))
    }
}

// =============================================================================
// TOOLTIP VIEW
// =============================================================================

/// Simple tooltip content view
pub struct TooltipView {
    text: &'static str,
}

impl TooltipView {
    pub fn new(text: &'static str) -> Self {
        Self { text }
    }
}

impl Render for TooltipView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_3()
            .py_2()
            .bg(rgb(0x1f1f3a))
            .border_1()
            .border_color(rgb(0x4f4f6f))
            .rounded_md()
            .text_sm()
            .text_color(rgb(0xffffff))
            .child(self.text)
    }
}

// =============================================================================
// TOOLTIP TESTS (TT01-TT03)
// =============================================================================

impl TestHarness {
    pub fn render_tooltip_tests(&mut self, _cx: &mut Context<Self>) -> impl IntoElement {
        test_grid()
            // TT01: Basic Tooltip
            .child(test_card("TT01", "Basic Tooltip", "Tooltip appears on hover",
                div()
                    .id("tt01-target")
                    .w(px(150.))
                    .h(px(50.))
                    .bg(rgb(0x6366f1))
                    .rounded_md()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(0x818cf8)))
                    .child("Hover for tooltip")
                    .tooltip(|_window, cx| {
                        cx.new(|_| TooltipView::new("This is a tooltip!")).into()
                    }),
            ))
            // TT02: Tooltip with different content
            .child(test_card("TT02", "Tooltip Content", "Shows different tooltip text",
                div()
                    .id("tt02-target")
                    .w(px(150.))
                    .h(px(50.))
                    .bg(rgb(0x22c55e))
                    .rounded_md()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(0x4ade80)))
                    .child("Hover me too")
                    .tooltip(|_window, cx| {
                        cx.new(|_| TooltipView::new("Different tooltip content")).into()
                    }),
            ))
            // TT03: Hoverable Tooltip
            .child(test_card("TT03", "Hoverable Tooltip", "Can hover into the tooltip",
                div()
                    .id("tt03-target")
                    .w(px(150.))
                    .h(px(50.))
                    .bg(rgb(0xf59e0b))
                    .rounded_md()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(0xfbbf24)))
                    .child("Hoverable tooltip")
                    .hoverable_tooltip(|_window, cx| {
                        cx.new(|_| TooltipView::new("You can hover into this tooltip!")).into()
                    }),
            ))
    }
}

// =============================================================================
// SPRITE TESTS (I01-I04)
// =============================================================================

/// Create a colorful checkerboard image for testing
fn create_test_image() -> Arc<RenderImage> {
    let width = 64u32;
    let height = 64u32;
    let mut img_data = RgbaImage::new(width, height);

    // Create a colorful gradient/checkerboard pattern
    for y in 0..height {
        for x in 0..width {
            let cell_x = x / 8;
            let cell_y = y / 8;

            // Create a colorful pattern based on cell position
            // Using u8 values directly for colors
            let (r, g, b): (u8, u8, u8) = match (cell_x + cell_y) % 8 {
                0 => (255, 0, 0),      // Red
                1 => (0, 255, 0),      // Green
                2 => (0, 0, 255),      // Blue
                3 => (255, 255, 0),    // Yellow
                4 => (255, 0, 255),    // Magenta
                5 => (0, 255, 255),    // Cyan
                6 => (255, 128, 0),    // Orange
                7 => (128, 0, 255),    // Purple
                _ => (255, 255, 255),  // White
            };

            // BGRA format: GPUI texture expects BGRA8, but image::Rgba stores as RGBA in memory.
            // We need to swap R and B channels so when the raw bytes are read, they're in BGRA order.
            // image::Rgba([B, G, R, A]) will store bytes as [B, G, R, A] which is BGRA format.
            img_data.put_pixel(x, y, image::Rgba([b, g, r, 255u8]));
        }
    }

    log::info!("Created test image: {}x{}, {} bytes", width, height, img_data.as_raw().len());

    let frame = Frame::new(img_data);
    let frames: SmallVec<[Frame; 1]> = SmallVec::from_elem(frame, 1);
    Arc::new(RenderImage::new(frames))
}

pub fn render_sprite_tests() -> impl IntoElement {
    // Create test image once for reuse
    let test_image = create_test_image();
    let test_image_for_grayscale = test_image.clone();

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
        // I02: Polychrome Sprite - actual image
        .child(test_card("I02", "Polychrome Sprite", "Colors preserved",
            div()
                .flex()
                .items_center()
                .gap_4()
                .child(
                    div()
                        .bg(rgb(0x1a1a2e))
                        .rounded_md()
                        .p_2()
                        .child(
                            img(test_image.clone())
                                .w(px(64.))
                                .h(px(64.))
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x888888))
                        .child("64x64 colorful checkerboard"),
                ),
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
        // I04: Sprite Grayscale - same image with grayscale filter
        .child(test_card("I04", "Sprite Grayscale", "Desaturated",
            div()
                .flex()
                .items_center()
                .gap_4()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_1()
                        .child(
                            div()
                                .bg(rgb(0x1a1a2e))
                                .rounded_md()
                                .p_2()
                                .child(
                                    img(test_image_for_grayscale.clone())
                                        .w(px(64.))
                                        .h(px(64.))
                                ),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x888888))
                                .child("Original"),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_1()
                        .child(
                            div()
                                .bg(rgb(0x1a1a2e))
                                .rounded_md()
                                .p_2()
                                .child(
                                    img(test_image_for_grayscale)
                                        .w(px(64.))
                                        .h(px(64.))
                                        .grayscale(true)
                                ),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x888888))
                                .child("Grayscale"),
                        ),
                ),
        ))
}
