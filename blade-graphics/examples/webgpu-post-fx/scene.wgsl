// Scene rendering shader - renders to offscreen texture
//
// Draws a colorful spinning triangle

struct Uniforms {
    time: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let time = uniforms.time;

    // Base triangle vertices
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 0.6),
        vec2<f32>(-0.5, -0.4),
        vec2<f32>(0.5, -0.4),
    );

    // Rotate the triangle
    let angle = time;
    let c = cos(angle);
    let s = sin(angle);
    let p = positions[vertex_index];
    let rotated = vec2<f32>(
        p.x * c - p.y * s,
        p.x * s + p.y * c
    );

    // Animated colors
    var colors = array<vec3<f32>, 3>(
        vec3<f32>(sin(time) * 0.5 + 0.5, 0.2, 0.8),
        vec3<f32>(0.2, sin(time + 2.0) * 0.5 + 0.5, 0.3),
        vec3<f32>(0.9, 0.6, sin(time + 4.0) * 0.5 + 0.5),
    );

    var output: VertexOutput;
    output.position = vec4<f32>(rotated, 0.0, 1.0);
    output.color = colors[vertex_index];
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
