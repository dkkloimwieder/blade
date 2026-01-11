// Mandelbrot set compute shader
//
// Generates a fractal visualization using the escape-time algorithm

struct Params {
    center_x: f32,
    center_y: f32,
    zoom: f32,
    max_iterations: u32,
}

@group(0) @binding(0)
var<uniform> params: Params;
@group(0) @binding(1)
var output_tex: texture_storage_2d<rgba8unorm, write>;

// HSV to RGB conversion for colorful output
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> vec3<f32> {
    let c = v * s;
    let hp = h * 6.0;
    let x = c * (1.0 - abs(hp % 2.0 - 1.0));
    let m = v - c;

    var rgb: vec3<f32>;
    if (hp < 1.0) {
        rgb = vec3<f32>(c, x, 0.0);
    } else if (hp < 2.0) {
        rgb = vec3<f32>(x, c, 0.0);
    } else if (hp < 3.0) {
        rgb = vec3<f32>(0.0, c, x);
    } else if (hp < 4.0) {
        rgb = vec3<f32>(0.0, x, c);
    } else if (hp < 5.0) {
        rgb = vec3<f32>(x, 0.0, c);
    } else {
        rgb = vec3<f32>(c, 0.0, x);
    }

    return rgb + vec3<f32>(m, m, m);
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(output_tex);
    let coord = vec2<i32>(global_id.xy);

    // Bounds check
    if (coord.x >= i32(dims.x) || coord.y >= i32(dims.y)) {
        return;
    }

    // Map pixel to normalized coordinates [-0.5, 0.5]
    let aspect = f32(dims.x) / f32(dims.y);
    let px = (f32(coord.x) / f32(dims.x) - 0.5) * aspect;
    let py = f32(coord.y) / f32(dims.y) - 0.5;

    // Scale by zoom and offset by center
    // Higher zoom = smaller region = more detail
    let scale = 2.0 / params.zoom;  // At zoom=1, we see range of 2 units
    let c = vec2<f32>(
        px * scale + params.center_x,
        py * scale + params.center_y
    );

    // Mandelbrot iteration
    var z = vec2<f32>(0.0, 0.0);
    var iteration: u32 = 0u;
    let max_iter = 256u;

    for (var i: u32 = 0u; i < max_iter; i++) {
        let z_new = vec2<f32>(
            z.x * z.x - z.y * z.y + c.x,
            2.0 * z.x * z.y + c.y
        );
        z = z_new;

        if (dot(z, z) > 4.0) {
            break;
        }
        iteration++;
    }

    // Color based on iteration count
    var color: vec3<f32>;
    if (iteration == max_iter) {
        color = vec3<f32>(0.0, 0.0, 0.0);
    } else {
        let t = f32(iteration) / f32(max_iter);
        let hue = t * 3.0 % 1.0;
        color = hsv_to_rgb(hue, 0.8, 1.0 - t * 0.3);
    }

    textureStore(output_tex, coord, vec4<f32>(color, 1.0));
}
