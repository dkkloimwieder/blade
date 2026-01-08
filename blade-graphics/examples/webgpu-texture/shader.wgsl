// Texture sampling shader for WebGPU
//
// Demonstrates:
// - Texture binding with @group/@binding
// - Sampler binding
// - textureSample() in fragment shader
// - UV coordinate interpolation

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// Texture and sampler bindings (group 0)
var sprite_texture: texture_2d<f32>;
var sprite_sampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Full-screen quad using two triangles
    // Vertices: 0,1,2 (first triangle), 3,4,5 (second triangle)
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),  // bottom-left
        vec2<f32>( 1.0, -1.0),  // bottom-right
        vec2<f32>(-1.0,  1.0),  // top-left
        vec2<f32>(-1.0,  1.0),  // top-left
        vec2<f32>( 1.0, -1.0),  // bottom-right
        vec2<f32>( 1.0,  1.0),  // top-right
    );

    // UV coordinates (0,0 at top-left, 1,1 at bottom-right)
    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),  // bottom-left
        vec2<f32>(1.0, 1.0),  // bottom-right
        vec2<f32>(0.0, 0.0),  // top-left
        vec2<f32>(0.0, 0.0),  // top-left
        vec2<f32>(1.0, 1.0),  // bottom-right
        vec2<f32>(1.0, 0.0),  // top-right
    );

    var output: VertexOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    output.uv = uvs[vertex_index];
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Sample the texture at the interpolated UV coordinate
    return textureSample(sprite_texture, sprite_sampler, input.uv);
}
