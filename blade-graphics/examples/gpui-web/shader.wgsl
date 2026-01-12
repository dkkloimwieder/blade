// GPUI-style quad shader for WebGPU
//
// Renders rectangles with:
// - Rounded corners (per-corner radius via SDF)
// - Solid backgrounds
// - Borders with configurable width
// - Anti-aliased edges

struct Globals {
    viewport_size: vec2<f32>,
    _pad: vec2<f32>,
}

var<uniform> globals: Globals;

// Per-quad instance data (stored in storage buffer)
struct Quad {
    // Bounds: x, y, width, height (in pixels, screen space)
    bounds: vec4<f32>,
    // Background color (packed RGBA)
    background: u32,
    // Border color (packed RGBA)
    border_color: u32,
    // Border widths: top, right, bottom, left
    border_widths: vec4<f32>,
    // Corner radii: top-left, top-right, bottom-right, bottom-left
    corner_radii: vec4<f32>,
}

var<storage, read> quads: array<Quad>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local_pos: vec2<f32>,      // Position within quad (-half_size to +half_size)
    @location(1) half_size: vec2<f32>,      // Half of quad dimensions
    @location(2) background: vec4<f32>,     // Unpacked background color
    @location(3) border_color: vec4<f32>,   // Unpacked border color
    @location(4) corner_radii: vec4<f32>,   // Corner radii
    @location(5) border_width: f32,         // Average border width (simplified)
}

// Unpack RGBA from u32 (little-endian: R in lowest byte)
fn unpack_color(packed: u32) -> vec4<f32> {
    return vec4<f32>(
        f32((packed >> 0u) & 0xFFu) / 255.0,
        f32((packed >> 8u) & 0xFFu) / 255.0,
        f32((packed >> 16u) & 0xFFu) / 255.0,
        f32((packed >> 24u) & 0xFFu) / 255.0
    );
}

// Quad vertices: 6 vertices for 2 triangles (triangle list)
// Vertex index maps to corners: 0=BL, 1=BR, 2=TL, 3=TL, 4=BR, 5=TR
fn vertex_position(vertex_index: u32) -> vec2<f32> {
    switch vertex_index {
        case 0u: { return vec2<f32>(0.0, 1.0); }  // bottom-left
        case 1u: { return vec2<f32>(1.0, 1.0); }  // bottom-right
        case 2u: { return vec2<f32>(0.0, 0.0); }  // top-left
        case 3u: { return vec2<f32>(0.0, 0.0); }  // top-left
        case 4u: { return vec2<f32>(1.0, 1.0); }  // bottom-right
        case 5u: { return vec2<f32>(1.0, 0.0); }  // top-right
        default: { return vec2<f32>(0.0, 0.0); }
    }
}

@vertex
fn vs_quad(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32
) -> VertexOutput {
    let quad = quads[instance_index];

    // Get normalized vertex position (0-1)
    let unit_pos = vertex_position(vertex_index);

    // Compute world position
    let world_pos = quad.bounds.xy + unit_pos * quad.bounds.zw;

    // Convert to clip space: (0,0) top-left, (viewport) bottom-right -> NDC
    let clip_pos = vec2<f32>(
        (world_pos.x / globals.viewport_size.x) * 2.0 - 1.0,
        1.0 - (world_pos.y / globals.viewport_size.y) * 2.0
    );

    // Local position relative to quad center
    let half_size = quad.bounds.zw * 0.5;
    let local_pos = (unit_pos - 0.5) * quad.bounds.zw;

    // Average border width for simplified rendering
    let avg_border = (quad.border_widths.x + quad.border_widths.y +
                      quad.border_widths.z + quad.border_widths.w) * 0.25;

    var output: VertexOutput;
    output.position = vec4<f32>(clip_pos, 0.0, 1.0);
    output.local_pos = local_pos;
    output.half_size = half_size;
    output.background = unpack_color(quad.background);
    output.border_color = unpack_color(quad.border_color);
    output.corner_radii = quad.corner_radii;
    output.border_width = avg_border;
    return output;
}

// Signed distance field for rounded rectangle
// Returns negative inside, positive outside
fn sdf_rounded_rect(point: vec2<f32>, half_size: vec2<f32>, radii: vec4<f32>) -> f32 {
    // Select corner radius based on quadrant
    // radii: x=top-left, y=top-right, z=bottom-right, w=bottom-left
    let r = select(
        select(radii.z, radii.w, point.x < 0.0),  // bottom: z or w
        select(radii.y, radii.x, point.x < 0.0),  // top: y or x
        point.y < 0.0  // y < 0 means top half (inverted coords)
    );

    // SDF for rounded rectangle
    let q = abs(point) - half_size + r;
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - r;
}

@fragment
fn fs_quad(input: VertexOutput) -> @location(0) vec4<f32> {
    let half_size = input.half_size;
    let border_width = input.border_width;

    // Compute SDF distance
    let distance = sdf_rounded_rect(input.local_pos, half_size, input.corner_radii);

    // Anti-aliased edge (1 pixel transition)
    let edge_aa = 1.0 - smoothstep(-0.5, 0.5, distance);

    // Early discard for fully outside pixels
    if edge_aa < 0.001 {
        discard;
    }

    // Border computation
    var final_color: vec4<f32>;
    if border_width > 0.0 {
        // Inner edge of border
        let inner_distance = sdf_rounded_rect(
            input.local_pos,
            half_size - border_width,
            max(input.corner_radii - border_width, vec4<f32>(0.0))
        );
        let in_border = smoothstep(-0.5, 0.5, inner_distance);

        // Mix background and border based on position
        final_color = mix(input.background, input.border_color, in_border);
    } else {
        final_color = input.background;
    }

    // Apply edge anti-aliasing and premultiply alpha
    let alpha = final_color.a * edge_aa;
    return vec4<f32>(final_color.rgb * alpha, alpha);
}
