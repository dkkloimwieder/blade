// Post-processing shader
//
// Applies visual effects to the rendered scene:
// - Vignette (darkened edges)
// - Chromatic aberration (RGB channel offset)
// - Slight blur

struct Uniforms {
    time: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

var<uniform> uniforms: Uniforms;
var scene_texture: texture_2d<f32>;
var scene_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Fullscreen quad
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

// Vignette effect - darken edges
fn vignette(uv: vec2<f32>, strength: f32) -> f32 {
    let center = uv - 0.5;
    let dist = length(center);
    return 1.0 - smoothstep(0.3, 0.7, dist * strength);
}

// Chromatic aberration - offset RGB channels
fn chromatic_aberration(uv: vec2<f32>, amount: f32) -> vec3<f32> {
    let center = uv - 0.5;
    let offset = center * amount;

    let r = textureSample(scene_texture, scene_sampler, uv + offset).r;
    let g = textureSample(scene_texture, scene_sampler, uv).g;
    let b = textureSample(scene_texture, scene_sampler, uv - offset).b;

    return vec3<f32>(r, g, b);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let uv = input.uv;

    // Animated effect intensity
    let pulse = sin(uniforms.time * 2.0) * 0.5 + 0.5;

    // Apply chromatic aberration (subtle, animated)
    let aberration_amount = 0.005 + pulse * 0.01;
    var color = chromatic_aberration(uv, aberration_amount);

    // Apply vignette
    let vignette_strength = 1.5;
    color *= vignette(uv, vignette_strength);

    // Slight contrast boost
    color = pow(color, vec3<f32>(0.95));

    return vec4<f32>(color, 1.0);
}
