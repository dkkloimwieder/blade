// Group 0: Static resources (cached bind group)
var sprite_texture: texture_2d<f32>;
var sprite_sampler: sampler;

// Instance data stored in storage buffer
struct InstanceData {
    position: vec2<f32>,
    velocity: vec2<f32>,  // unused in shader, but part of buffer layout
    color: u32,
    pad: u32,
};
var<storage, read> instances: array<InstanceData>;

// Group 1: Per-frame uniform (recreated each frame)
struct Globals {
    mvp_transform: mat4x4<f32>,
    sprite_size: vec2<f32>,
};
var<uniform> globals: Globals;

// Per-vertex data (quad corners)
struct Vertex {
    pos: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
}

fn unpack_color(raw: u32) -> vec4<f32> {
    //TODO: https://github.com/gfx-rs/naga/issues/2188
    //return unpack4x8unorm(raw);
    return vec4<f32>((vec4<u32>(raw) >> vec4<u32>(0u, 8u, 16u, 24u)) & vec4<u32>(0xFFu)) / 255.0;
}

@vertex
fn vs_main(vertex: Vertex, @builtin(instance_index) instance_id: u32) -> VertexOutput {
    let instance = instances[instance_id];
    let tc = vertex.pos;
    let offset = tc * globals.sprite_size;
    let pos = globals.mvp_transform * vec4<f32>(instance.position + offset, 0.0, 1.0);
    let color = unpack_color(instance.color);
    return VertexOutput(pos, tc, color);
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return vertex.color * textureSampleLevel(sprite_texture, sprite_sampler, vertex.tex_coords, 0.0);
}
