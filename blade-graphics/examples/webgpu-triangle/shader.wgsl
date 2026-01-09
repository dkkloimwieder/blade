// Minimal triangle shader for WebGPU testing
// Supports runtime color tinting via uniform

struct Uniforms {
    color_tint: vec4<f32>,
}

var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Hardcoded triangle vertices
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 0.5),    // top
        vec2<f32>(-0.5, -0.5),  // bottom-left
        vec2<f32>(0.5, -0.5),   // bottom-right
    );
    var colors = array<vec3<f32>, 3>(
        vec3<f32>(1.0, 0.0, 0.0),  // red
        vec3<f32>(0.0, 1.0, 0.0),  // green
        vec3<f32>(0.0, 0.0, 1.0),  // blue
    );

    var output: VertexOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    output.color = colors[vertex_index];
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Mix vertex color with tint based on tint alpha
    // tint.a = 0: pure vertex colors, tint.a = 1: pure tint color
    let vertex_color = vec4<f32>(input.color, 1.0);
    let tinted = mix(vertex_color.rgb, uniforms.color_tint.rgb, uniforms.color_tint.a);
    return vec4<f32>(tinted, 1.0);
}
