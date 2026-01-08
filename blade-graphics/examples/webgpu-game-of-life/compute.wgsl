// Game of Life compute shader
//
// Standard Conway's Game of Life rules:
// 1. Live cell with 2-3 neighbors survives
// 2. Dead cell with exactly 3 neighbors becomes alive
// 3. All other cells die

var input_tex: texture_storage_2d<rgba8unorm, read>;
var output_tex: texture_storage_2d<rgba8unorm, write>;

fn count_neighbors(pos: vec2<i32>, dims: vec2<i32>) -> u32 {
    var count: u32 = 0u;

    for (var dy: i32 = -1; dy <= 1; dy++) {
        for (var dx: i32 = -1; dx <= 1; dx++) {
            if (dx == 0 && dy == 0) {
                continue;
            }

            let nx = (pos.x + dx + dims.x) % dims.x;
            let ny = (pos.y + dy + dims.y) % dims.y;

            let neighbor = textureLoad(input_tex, vec2<i32>(nx, ny));
            if (neighbor.r > 0.5) {
                count += 1u;
            }
        }
    }

    return count;
}

@compute @workgroup_size(8, 8)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = vec2<i32>(textureDimensions(input_tex));
    let pos = vec2<i32>(global_id.xy);

    if (pos.x >= dims.x || pos.y >= dims.y) {
        return;
    }

    let current = textureLoad(input_tex, pos);
    let is_alive = current.r > 0.5;
    let neighbors = count_neighbors(pos, dims);

    var next_alive = false;
    if (is_alive) {
        // Live cell survives with 2-3 neighbors
        next_alive = (neighbors == 2u || neighbors == 3u);
    } else {
        // Dead cell becomes alive with exactly 3 neighbors
        next_alive = (neighbors == 3u);
    }

    let value = select(0.0, 1.0, next_alive);
    textureStore(output_tex, pos, vec4<f32>(value, value, value, 1.0));
}
