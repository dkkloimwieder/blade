//=============================================================================
// Render Shader - Draws culled instances using visible_indices indirection
//=============================================================================

struct Globals {
    view_proj: mat4x4<f32>,
};

struct ObjectData {
    model: mat4x4<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<storage, read> objects: array<ObjectData>;
@group(0) @binding(2) var<storage, read> visible_indices: array<u32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

// Simple cube vertices (8 corners, 36 indices for 12 triangles)
const CUBE_POSITIONS: array<vec3<f32>, 8> = array<vec3<f32>, 8>(
    vec3<f32>(-0.5, -0.5, -0.5),
    vec3<f32>( 0.5, -0.5, -0.5),
    vec3<f32>( 0.5,  0.5, -0.5),
    vec3<f32>(-0.5,  0.5, -0.5),
    vec3<f32>(-0.5, -0.5,  0.5),
    vec3<f32>( 0.5, -0.5,  0.5),
    vec3<f32>( 0.5,  0.5,  0.5),
    vec3<f32>(-0.5,  0.5,  0.5),
);

const CUBE_INDICES: array<u32, 36> = array<u32, 36>(
    // Front
    0u, 1u, 2u, 2u, 3u, 0u,
    // Back
    5u, 4u, 7u, 7u, 6u, 5u,
    // Left
    4u, 0u, 3u, 3u, 7u, 4u,
    // Right
    1u, 5u, 6u, 6u, 2u, 1u,
    // Top
    3u, 2u, 6u, 6u, 7u, 3u,
    // Bottom
    4u, 5u, 1u, 1u, 0u, 4u,
);

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_idx: u32,
    @builtin(instance_index) instance_idx: u32,
) -> VertexOutput {
    // Look up which object this instance refers to (from compacted visible list)
    let obj_idx = visible_indices[instance_idx];
    let obj = objects[obj_idx];

    // Get vertex position from cube
    let corner_idx = CUBE_INDICES[vertex_idx];
    let local_pos = CUBE_POSITIONS[corner_idx];

    // Transform to world then clip space
    let world_pos = obj.model * vec4<f32>(local_pos, 1.0);
    let clip_pos = globals.view_proj * world_pos;

    var out: VertexOutput;
    out.position = clip_pos;
    out.color = obj.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
