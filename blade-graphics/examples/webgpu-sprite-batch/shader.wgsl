// Sprite batch shader
//
// Efficient 2D sprite rendering using instancing with storage buffer.
// Each sprite has position, size, rotation, and color.

struct Globals {
    screen_size: vec2<f32>,
    time: f32,
    sprite_count: u32,
}

var<uniform> globals: Globals;

// Sprite instance data stored in storage buffer
struct SpriteData {
    position: vec2<f32>,
    size: vec2<f32>,
    rotation: f32,
    color: u32,
    _pad: vec2<f32>,
}

var<storage, read> sprites: array<SpriteData>;

// Per-vertex data (quad corners)
struct Vertex {
    pos: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

// Unpack RGBA from u32
fn unpack_color(packed: u32) -> vec4<f32> {
    return vec4<f32>(
        f32((packed >> 0u) & 0xFFu) / 255.0,
        f32((packed >> 8u) & 0xFFu) / 255.0,
        f32((packed >> 16u) & 0xFFu) / 255.0,
        f32((packed >> 24u) & 0xFFu) / 255.0
    );
}

@vertex
fn vs_main(vertex: Vertex, @builtin(instance_index) instance_id: u32) -> VertexOutput {
    let sprite = sprites[instance_id];

    // Apply rotation to local position
    let cos_r = cos(sprite.rotation);
    let sin_r = sin(sprite.rotation);
    let local_pos = vertex.pos; // -0.5 to 0.5
    let rotated = vec2<f32>(
        local_pos.x * cos_r - local_pos.y * sin_r,
        local_pos.x * sin_r + local_pos.y * cos_r
    );

    // Scale by sprite size
    let scaled = rotated * sprite.size;

    // Translate to world position
    let world_pos = scaled + sprite.position;

    // Convert to clip space (-1 to 1)
    // Screen coordinates: (0,0) at top-left, (width, height) at bottom-right
    let clip_pos = vec2<f32>(
        (world_pos.x / globals.screen_size.x) * 2.0 - 1.0,
        1.0 - (world_pos.y / globals.screen_size.y) * 2.0
    );

    // UV from local position
    let uv = local_pos + 0.5;

    var output: VertexOutput;
    output.position = vec4<f32>(clip_pos, 0.0, 1.0);
    output.uv = uv;
    output.color = unpack_color(sprite.color);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Simple colored quad with slight gradient for visual interest
    let brightness = 0.8 + 0.2 * (1.0 - length(input.uv - 0.5));
    return vec4<f32>(input.color.rgb * brightness, input.color.a);
}
