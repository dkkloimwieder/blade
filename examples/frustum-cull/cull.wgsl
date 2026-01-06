//=============================================================================
// GPU Frustum Culling Compute Shader
//=============================================================================
// Tests bounding spheres against view frustum planes.
// Visible objects are compacted into output buffer for indirect drawing.

// Frustum plane: xyz = normal, w = distance from origin
struct Frustum {
    planes: array<vec4<f32>, 6>,  // left, right, bottom, top, near, far
};

// Object bounding sphere: xyz = center, w = radius
struct BoundingSphere {
    center: vec3<f32>,
    radius: f32,
};

// Indirect draw arguments (matches WebGPU DrawIndirect struct)
struct DrawIndirect {
    vertex_count: u32,
    instance_count: atomic<u32>,
    first_vertex: u32,
    first_instance: u32,
};

// Cull parameters
struct CullParams {
    object_count: u32,
    vertices_per_object: u32,
    _pad: vec2<u32>,
};

// Bindings
@group(0) @binding(0) var<uniform> frustum: Frustum;
@group(0) @binding(1) var<uniform> params: CullParams;
@group(0) @binding(2) var<storage, read> bounds: array<BoundingSphere>;
@group(0) @binding(3) var<storage, read_write> indirect: DrawIndirect;
@group(0) @binding(4) var<storage, read_write> visible_indices: array<u32>;

// Test if bounding sphere is inside or intersects frustum
fn sphere_vs_frustum(sphere: BoundingSphere, frustum: Frustum) -> bool {
    // Test against each plane
    // Plane equation: dot(normal, point) + d >= -radius means visible
    for (var i = 0u; i < 6u; i++) {
        let plane = frustum.planes[i];
        let dist = dot(plane.xyz, sphere.center) + plane.w;
        if (dist < -sphere.radius) {
            return false;  // Completely outside this plane
        }
    }
    return true;  // Inside or intersecting all planes
}

@compute @workgroup_size(256)
fn cs_cull(@builtin(global_invocation_id) id: vec3<u32>) {
    let obj_idx = id.x;
    if (obj_idx >= params.object_count) {
        return;
    }

    let sphere = bounds[obj_idx];

    if (sphere_vs_frustum(sphere, frustum)) {
        // Object is visible - atomically add to output
        let out_idx = atomicAdd(&indirect.instance_count, 1u);
        visible_indices[out_idx] = obj_idx;
    }
}

// Reset indirect buffer before culling
@compute @workgroup_size(1)
fn cs_reset() {
    atomicStore(&indirect.instance_count, 0u);
}
