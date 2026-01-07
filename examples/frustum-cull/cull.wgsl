//=============================================================================
// GPU Frustum Culling with Prefix Sum Compaction
//=============================================================================
// Three-pass algorithm for stable, deterministic output ordering:
// Pass 1 (cs_cull): Test visibility, compute local prefix sums, store workgroup totals
// Pass 2 (cs_scan_workgroups): Prefix sum of workgroup totals
// Pass 3 (cs_scatter): Scatter visible objects using global prefix sum indices

const WORKGROUP_SIZE: u32 = 256u;
const MAX_WORKGROUPS: u32 = 64u;  // Supports up to 16384 objects

struct Frustum {
    planes: array<vec4<f32>, 6>,
};

struct BoundingSphere {
    center: vec3<f32>,
    radius: f32,
};

struct DrawIndirect {
    vertex_count: u32,
    instance_count: u32,  // Non-atomic for final write
    first_vertex: u32,
    first_instance: u32,
};

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

// Intermediate buffers for prefix sum
@group(0) @binding(5) var<storage, read_write> visibility: array<u32>;  // 0 or 1 per object
@group(0) @binding(6) var<storage, read_write> local_prefix: array<u32>;  // Per-object local prefix
@group(0) @binding(7) var<storage, read_write> workgroup_totals: array<u32>;  // Sum per workgroup
@group(0) @binding(8) var<storage, read_write> workgroup_offsets: array<u32>;  // Prefix sum of totals

// Workgroup shared memory for prefix sum
var<workgroup> shared_data: array<u32, WORKGROUP_SIZE>;
var<workgroup> shared_scan: array<u32, 64>;  // For workgroup totals scan

fn sphere_vs_frustum(sphere: BoundingSphere, frust: Frustum) -> bool {
    for (var i = 0u; i < 6u; i++) {
        let plane = frust.planes[i];
        let dist = dot(plane.xyz, sphere.center) + plane.w;
        if (dist < -sphere.radius) {
            return false;
        }
    }
    return true;
}

// Pass 1: Compute visibility and local prefix sums within each workgroup
@compute @workgroup_size(256)
fn cs_cull(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) wg_id: vec3<u32>,
) {
    let obj_idx = global_id.x;
    let lid = local_id.x;
    let wg = wg_id.x;

    // Load visibility (1 = visible, 0 = culled)
    var vis = 0u;
    if (obj_idx < params.object_count) {
        let sphere = bounds[obj_idx];
        vis = select(0u, 1u, sphere_vs_frustum(sphere, frustum));
        visibility[obj_idx] = vis;
    }

    // Workgroup-level inclusive prefix sum (Hillis-Steele)
    shared_data[lid] = vis;
    workgroupBarrier();

    for (var stride = 1u; stride < WORKGROUP_SIZE; stride *= 2u) {
        var val = shared_data[lid];
        if (lid >= stride) {
            val += shared_data[lid - stride];
        }
        workgroupBarrier();
        shared_data[lid] = val;
        workgroupBarrier();
    }

    // Store local prefix (exclusive: subtract own visibility)
    if (obj_idx < params.object_count) {
        local_prefix[obj_idx] = shared_data[lid] - vis;
    }

    // Last thread stores workgroup total
    if (lid == WORKGROUP_SIZE - 1u) {
        workgroup_totals[wg] = shared_data[lid];
    }
}

// Pass 2: Prefix sum of workgroup totals (single workgroup, up to 64 workgroups)
@compute @workgroup_size(64)
fn cs_scan_workgroups(
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    let lid = local_id.x;
    let num_workgroups = (params.object_count + WORKGROUP_SIZE - 1u) / WORKGROUP_SIZE;

    // Load workgroup total into shared memory
    var val = 0u;
    if (lid < num_workgroups) {
        val = workgroup_totals[lid];
    }
    shared_scan[lid] = val;
    workgroupBarrier();

    // Inclusive prefix sum (Hillis-Steele)
    for (var stride = 1u; stride < 64u; stride *= 2u) {
        var sum = shared_scan[lid];
        if (lid >= stride) {
            sum += shared_scan[lid - stride];
        }
        workgroupBarrier();
        shared_scan[lid] = sum;
        workgroupBarrier();
    }

    // Store exclusive prefix (offset for this workgroup)
    if (lid < num_workgroups) {
        workgroup_offsets[lid] = shared_scan[lid] - val;
    }

    // Last active thread stores total visible count
    if (lid == num_workgroups - 1u) {
        indirect.instance_count = shared_scan[lid];
    }
}

// Pass 3: Scatter visible objects to compacted output
@compute @workgroup_size(256)
fn cs_scatter(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(workgroup_id) wg_id: vec3<u32>,
) {
    let obj_idx = global_id.x;
    let wg = wg_id.x;

    if (obj_idx >= params.object_count) {
        return;
    }

    // Only scatter visible objects
    if (visibility[obj_idx] != 0u) {
        // Global output index = workgroup offset + local prefix
        let out_idx = workgroup_offsets[wg] + local_prefix[obj_idx];
        visible_indices[out_idx] = obj_idx;
    }
}
