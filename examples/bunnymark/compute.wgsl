//=============================================================================
// Compute Pipeline - Physics simulation on GPU
//=============================================================================

// Instance data - same structure as in shader.wgsl
struct InstanceData {
    position: vec2<f32>,
    velocity: vec2<f32>,
    color: u32,
    pad: u32,
};

struct SimParams {
    delta_time: f32,
    gravity: f32,
    bounds_width: f32,
    bounds_height: f32,
    sprite_half_size: f32,
    bunny_count: u32,
    _pad: vec2<f32>,
};

var<uniform> sim_params: SimParams;
var<storage, read_write> instances_rw: array<InstanceData>;

@compute @workgroup_size(256)
fn cs_update(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= sim_params.bunny_count) {
        return;
    }

    var bunny = instances_rw[i];
    let dt = sim_params.delta_time;

    // Update position
    bunny.position.x += bunny.velocity.x * dt;
    bunny.position.y += bunny.velocity.y * dt;

    // Apply gravity
    bunny.velocity.y += sim_params.gravity * dt;

    // Bounce off walls (horizontal)
    let half = sim_params.sprite_half_size;
    if (bunny.velocity.x > 0.0 && bunny.position.x + half > sim_params.bounds_width) {
        bunny.velocity.x = -bunny.velocity.x;
    }
    if (bunny.velocity.x < 0.0 && bunny.position.x - half < 0.0) {
        bunny.velocity.x = -bunny.velocity.x;
    }

    // Bounce off floor
    if (bunny.velocity.y < 0.0 && bunny.position.y < half) {
        bunny.velocity.y = -bunny.velocity.y;
    }

    instances_rw[i] = bunny;
}
