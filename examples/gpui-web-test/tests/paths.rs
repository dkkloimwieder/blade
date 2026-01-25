//! Path rendering tests - P01-P07
//! P01-P02: Filled polygons (straight lines)
//! P03-P04: Filled shapes with bezier curves
//! P05: Stroked (unfilled) paths
//! P06-P07: Line segments (straight and curved)

use gpui::{
    canvas, div, point, px, rgb, IntoElement, ParentElement, PathBuilder, Pixels, Point, Styled,
};
use super::test_card;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Create a filled triangle path
fn create_filled_triangle(p1: Point<Pixels>, p2: Point<Pixels>, p3: Point<Pixels>) -> gpui::Path<Pixels> {
    let mut builder = PathBuilder::fill();
    builder.move_to(p1);
    builder.line_to(p2);
    builder.line_to(p3);
    builder.close();
    builder.build().unwrap()
}

/// Create a stroked triangle path (outline only)
fn create_stroked_triangle(p1: Point<Pixels>, p2: Point<Pixels>, p3: Point<Pixels>, width: Pixels) -> gpui::Path<Pixels> {
    let mut builder = PathBuilder::stroke(width);
    builder.move_to(p1);
    builder.line_to(p2);
    builder.line_to(p3);
    builder.close();
    builder.build().unwrap()
}

/// Create a filled regular polygon path with n sides
fn create_filled_polygon(center: Point<Pixels>, radius: Pixels, sides: usize) -> gpui::Path<Pixels> {
    let mut builder = PathBuilder::fill();
    let angle_step = std::f32::consts::TAU / sides as f32;
    let start_angle = -std::f32::consts::FRAC_PI_2;

    for i in 0..sides {
        let angle = start_angle + angle_step * i as f32;
        let r: f32 = radius.into();
        let x = center.x + px(r * angle.cos());
        let y = center.y + px(r * angle.sin());
        if i == 0 {
            builder.move_to(point(x, y));
        } else {
            builder.line_to(point(x, y));
        }
    }
    builder.close();
    builder.build().unwrap()
}

/// Create a stroked polygon outline
fn create_stroked_polygon(center: Point<Pixels>, radius: Pixels, sides: usize, width: Pixels) -> gpui::Path<Pixels> {
    let mut builder = PathBuilder::stroke(width);
    let angle_step = std::f32::consts::TAU / sides as f32;
    let start_angle = -std::f32::consts::FRAC_PI_2;

    for i in 0..sides {
        let angle = start_angle + angle_step * i as f32;
        let r: f32 = radius.into();
        let x = center.x + px(r * angle.cos());
        let y = center.y + px(r * angle.sin());
        if i == 0 {
            builder.move_to(point(x, y));
        } else {
            builder.line_to(point(x, y));
        }
    }
    builder.close();
    builder.build().unwrap()
}

/// Create a filled circle using 4 cubic bezier curves
/// Uses the standard bezier circle approximation: control point offset = radius * 0.5523
fn create_filled_circle(center: Point<Pixels>, radius: Pixels) -> gpui::Path<Pixels> {
    let mut builder = PathBuilder::fill();
    let r: f32 = radius.into();
    let k = r * 0.5523; // Magic number for circle approximation

    let cx = center.x;
    let cy = center.y;

    // Start at top of circle
    builder.move_to(point(cx, cy - radius));

    // Top to right (clockwise)
    builder.cubic_bezier_to(
        point(cx + radius, cy),           // end: right
        point(cx + px(k), cy - radius),   // ctrl1
        point(cx + radius, cy - px(k)),   // ctrl2
    );

    // Right to bottom
    builder.cubic_bezier_to(
        point(cx, cy + radius),           // end: bottom
        point(cx + radius, cy + px(k)),   // ctrl1
        point(cx + px(k), cy + radius),   // ctrl2
    );

    // Bottom to left
    builder.cubic_bezier_to(
        point(cx - radius, cy),           // end: left
        point(cx - px(k), cy + radius),   // ctrl1
        point(cx - radius, cy + px(k)),   // ctrl2
    );

    // Left to top
    builder.cubic_bezier_to(
        point(cx, cy - radius),           // end: top
        point(cx - radius, cy - px(k)),   // ctrl1
        point(cx - px(k), cy - radius),   // ctrl2
    );

    builder.close();
    builder.build().unwrap()
}

/// Create a stroked circle outline
fn create_stroked_circle(center: Point<Pixels>, radius: Pixels, width: Pixels) -> gpui::Path<Pixels> {
    let mut builder = PathBuilder::stroke(width);
    let r: f32 = radius.into();
    let k = r * 0.5523;

    let cx = center.x;
    let cy = center.y;

    builder.move_to(point(cx, cy - radius));

    builder.cubic_bezier_to(
        point(cx + radius, cy),
        point(cx + px(k), cy - radius),
        point(cx + radius, cy - px(k)),
    );

    builder.cubic_bezier_to(
        point(cx, cy + radius),
        point(cx + radius, cy + px(k)),
        point(cx + px(k), cy + radius),
    );

    builder.cubic_bezier_to(
        point(cx - radius, cy),
        point(cx - px(k), cy + radius),
        point(cx - radius, cy + px(k)),
    );

    builder.cubic_bezier_to(
        point(cx, cy - radius),
        point(cx - radius, cy - px(k)),
        point(cx - px(k), cy - radius),
    );

    builder.close();
    builder.build().unwrap()
}

/// Create a simple quadratic bezier curve (stroked)
fn create_quadratic_curve(start: Point<Pixels>, end: Point<Pixels>, ctrl: Point<Pixels>, width: Pixels) -> gpui::Path<Pixels> {
    let mut builder = PathBuilder::stroke(width);
    builder.move_to(start);
    builder.curve_to(end, ctrl);  // curve_to(end, control)
    builder.build().unwrap()
}

/// Create a straight line segment (stroked)
fn create_line_segment(start: Point<Pixels>, end: Point<Pixels>, width: Pixels) -> gpui::Path<Pixels> {
    let mut builder = PathBuilder::stroke(width);
    builder.move_to(start);
    builder.line_to(end);
    builder.build().unwrap()
}

// =============================================================================
// CANVAS ELEMENTS
// =============================================================================

/// P01: Filled triangle
fn p01_filled_triangle() -> impl IntoElement {
    canvas(
        move |_bounds, _window, _cx| {},
        move |bounds, _, window, _cx| {
            let cx = bounds.origin.x + bounds.size.width / 2.0;
            let cy = bounds.origin.y + bounds.size.height / 2.0;

            let triangle = create_filled_triangle(
                point(cx, cy - px(25.)),           // Top
                point(cx - px(25.), cy + px(20.)), // Bottom left
                point(cx + px(25.), cy + px(20.)), // Bottom right
            );
            window.paint_path(triangle, rgb(0x6366f1)); // Purple
        },
    )
    .w(px(80.))
    .h(px(70.))
    .bg(rgb(0x1a1a2e))
    .rounded_md()
}

/// P02: Filled pentagon
fn p02_filled_pentagon() -> impl IntoElement {
    canvas(
        move |_bounds, _window, _cx| {},
        move |bounds, _, window, _cx| {
            let cx = bounds.origin.x + bounds.size.width / 2.0;
            let cy = bounds.origin.y + bounds.size.height / 2.0;

            let pentagon = create_filled_polygon(point(cx, cy), px(25.), 5);
            window.paint_path(pentagon, rgb(0x22c55e)); // Green
        },
    )
    .w(px(80.))
    .h(px(70.))
    .bg(rgb(0x1a1a2e))
    .rounded_md()
}

/// P03: Filled circle (cubic beziers)
fn p03_filled_circle() -> impl IntoElement {
    canvas(
        move |_bounds, _window, _cx| {},
        move |bounds, _, window, _cx| {
            let cx = bounds.origin.x + bounds.size.width / 2.0;
            let cy = bounds.origin.y + bounds.size.height / 2.0;

            let circle = create_filled_circle(point(cx, cy), px(25.));
            window.paint_path(circle, rgb(0xf59e0b)); // Orange
        },
    )
    .w(px(80.))
    .h(px(70.))
    .bg(rgb(0x1a1a2e))
    .rounded_md()
}

/// P04: Filled hexagon
fn p04_filled_hexagon() -> impl IntoElement {
    canvas(
        move |_bounds, _window, _cx| {},
        move |bounds, _, window, _cx| {
            let cx = bounds.origin.x + bounds.size.width / 2.0;
            let cy = bounds.origin.y + bounds.size.height / 2.0;

            let hexagon = create_filled_polygon(point(cx, cy), px(25.), 6);
            window.paint_path(hexagon, rgb(0xec4899)); // Pink
        },
    )
    .w(px(80.))
    .h(px(70.))
    .bg(rgb(0x1a1a2e))
    .rounded_md()
}

/// P05a: Stroked triangle outline
fn p05a_stroked_triangle() -> impl IntoElement {
    canvas(
        move |_bounds, _window, _cx| {},
        move |bounds, _, window, _cx| {
            let cx = bounds.origin.x + bounds.size.width / 2.0;
            let cy = bounds.origin.y + bounds.size.height / 2.0;

            let triangle = create_stroked_triangle(
                point(cx, cy - px(25.)),
                point(cx - px(25.), cy + px(20.)),
                point(cx + px(25.), cy + px(20.)),
                px(3.),
            );
            window.paint_path(triangle, rgb(0x6366f1)); // Purple
        },
    )
    .w(px(80.))
    .h(px(70.))
    .bg(rgb(0x1a1a2e))
    .rounded_md()
}

/// P05b: Stroked circle outline
fn p05b_stroked_circle() -> impl IntoElement {
    canvas(
        move |_bounds, _window, _cx| {},
        move |bounds, _, window, _cx| {
            let cx = bounds.origin.x + bounds.size.width / 2.0;
            let cy = bounds.origin.y + bounds.size.height / 2.0;

            let circle = create_stroked_circle(point(cx, cy), px(25.), px(3.));
            window.paint_path(circle, rgb(0xf59e0b)); // Orange
        },
    )
    .w(px(80.))
    .h(px(70.))
    .bg(rgb(0x1a1a2e))
    .rounded_md()
}

/// P05c: Stroked pentagon outline
fn p05c_stroked_pentagon() -> impl IntoElement {
    canvas(
        move |_bounds, _window, _cx| {},
        move |bounds, _, window, _cx| {
            let cx = bounds.origin.x + bounds.size.width / 2.0;
            let cy = bounds.origin.y + bounds.size.height / 2.0;

            let pentagon = create_stroked_polygon(point(cx, cy), px(25.), 5, px(3.));
            window.paint_path(pentagon, rgb(0x22c55e)); // Green
        },
    )
    .w(px(80.))
    .h(px(70.))
    .bg(rgb(0x1a1a2e))
    .rounded_md()
}

/// P06: Straight line segments
fn p06_straight_lines() -> impl IntoElement {
    canvas(
        move |_bounds, _window, _cx| {},
        move |bounds, _, window, _cx| {
            let x = bounds.origin.x + px(10.);
            let y = bounds.origin.y + px(15.);
            let w = bounds.size.width - px(20.);

            // Horizontal line
            let line1 = create_line_segment(
                point(x, y),
                point(x + w, y),
                px(2.),
            );
            window.paint_path(line1, rgb(0xef4444)); // Red

            // Diagonal line
            let line2 = create_line_segment(
                point(x, y + px(20.)),
                point(x + w, y + px(40.)),
                px(2.),
            );
            window.paint_path(line2, rgb(0x22c55e)); // Green

            // Vertical line
            let line3 = create_line_segment(
                point(x + w / 2.0, y + px(50.)),
                point(x + w / 2.0, y + px(70.)),
                px(2.),
            );
            window.paint_path(line3, rgb(0x3b82f6)); // Blue
        },
    )
    .w(px(120.))
    .h(px(100.))
    .bg(rgb(0x1a1a2e))
    .rounded_md()
}

/// P07: Curved line segments (quadratic bezier)
fn p07_curved_lines() -> impl IntoElement {
    canvas(
        move |_bounds, _window, _cx| {},
        move |bounds, _, window, _cx| {
            let x = bounds.origin.x + px(10.);
            let y = bounds.origin.y + px(20.);
            let w = bounds.size.width - px(20.);

            // Curve bowing up
            let curve1 = create_quadratic_curve(
                point(x, y + px(20.)),           // start
                point(x + w, y + px(20.)),       // end
                point(x + w / 2.0, y - px(10.)), // control (above line = bow up)
                px(2.),
            );
            window.paint_path(curve1, rgb(0xef4444)); // Red

            // Curve bowing down
            let curve2 = create_quadratic_curve(
                point(x, y + px(50.)),           // start
                point(x + w, y + px(50.)),       // end
                point(x + w / 2.0, y + px(80.)), // control (below line = bow down)
                px(2.),
            );
            window.paint_path(curve2, rgb(0x22c55e)); // Green

            // S-curve (two quadratics)
            let curve3a = create_quadratic_curve(
                point(x, y + px(90.)),
                point(x + w / 2.0, y + px(90.)),
                point(x + w / 4.0, y + px(70.)),
                px(2.),
            );
            window.paint_path(curve3a, rgb(0x3b82f6)); // Blue

            let curve3b = create_quadratic_curve(
                point(x + w / 2.0, y + px(90.)),
                point(x + w, y + px(90.)),
                point(x + w * 3.0 / 4.0, y + px(110.)),
                px(2.),
            );
            window.paint_path(curve3b, rgb(0x3b82f6)); // Blue
        },
    )
    .w(px(120.))
    .h(px(130.))
    .bg(rgb(0x1a1a2e))
    .rounded_md()
}

/// Helper function to create a test grid
fn test_grid() -> gpui::Div {
    div().flex().flex_col().gap_4()
}

// =============================================================================
// PATH TESTS (P01-P07)
// =============================================================================

pub fn render_path_tests() -> impl IntoElement {
    test_grid()
        // P01: Filled triangle
        .child(test_card("P01", "Filled Triangle", "3 vertices, filled",
            div()
                .flex()
                .gap_4()
                .items_center()
                .child(p01_filled_triangle())
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x888888))
                        .child("Purple filled triangle"),
                ),
        ))
        // P02: Filled pentagon
        .child(test_card("P02", "Filled Pentagon", "5 vertices, filled",
            div()
                .flex()
                .gap_4()
                .items_center()
                .child(p02_filled_pentagon())
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x888888))
                        .child("Green filled pentagon"),
                ),
        ))
        // P03: Filled circle (beziers)
        .child(test_card("P03", "Filled Circle", "Cubic bezier curves",
            div()
                .flex()
                .gap_4()
                .items_center()
                .child(p03_filled_circle())
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x888888))
                        .child("Orange circle via 4 cubic beziers"),
                ),
        ))
        // P04: Filled hexagon
        .child(test_card("P04", "Filled Hexagon", "6 vertices, filled",
            div()
                .flex()
                .gap_4()
                .items_center()
                .child(p04_filled_hexagon())
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x888888))
                        .child("Pink filled hexagon"),
                ),
        ))
        // P05: Stroked outlines
        .child(test_card("P05", "Stroked Outlines", "Unfilled shapes",
            div()
                .flex()
                .gap_4()
                .items_center()
                .child(p05a_stroked_triangle())
                .child(p05b_stroked_circle())
                .child(p05c_stroked_pentagon())
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x888888))
                        .child("3px stroke width"),
                ),
        ))
        // P06: Straight line segments
        .child(test_card("P06", "Line Segments", "Straight lines",
            div()
                .flex()
                .gap_4()
                .items_center()
                .child(p06_straight_lines())
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .text_xs()
                        .text_color(rgb(0x888888))
                        .child("Red: horizontal")
                        .child("Green: diagonal")
                        .child("Blue: vertical"),
                ),
        ))
        // P07: Curved line segments
        .child(test_card("P07", "Curved Lines", "Quadratic beziers",
            div()
                .flex()
                .gap_4()
                .items_center()
                .child(p07_curved_lines())
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .text_xs()
                        .text_color(rgb(0x888888))
                        .child("Red: curve up")
                        .child("Green: curve down")
                        .child("Blue: S-curve"),
                ),
        ))
}
