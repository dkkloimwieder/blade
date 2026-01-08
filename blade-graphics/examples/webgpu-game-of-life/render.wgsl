// Simple Game of Life render shader
// Green = alive, Dark = dead

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

var state_texture: texture_2d<f32>;
var state_sampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
    );

    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
    );

    var output: VertexOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    output.uv = uvs[vertex_index];
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let state = textureSample(state_texture, state_sampler, input.uv);

    // Simple two-color display
    let dead_color = vec3<f32>(0.1, 0.1, 0.15);
    let live_color = vec3<f32>(0.3, 0.9, 0.4);

    let color = select(dead_color, live_color, state.r > 0.5);
    return vec4<f32>(color, 1.0);
}
