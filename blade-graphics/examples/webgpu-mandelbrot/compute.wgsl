// Mandelbrot set compute shader
//
// Generates a fractal visualization using the escape-time algorithm

struct Params {
    center_x: f32,
    center_y: f32,
    zoom: f32,
    max_iterations: u32,
}

var<uniform> params: Params;
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

    // Map pixel to complex plane
    let aspect = f32(dims.x) / f32(dims.y);
    let uv = vec2<f32>(
        (f32(coord.x) / f32(dims.x) - 0.5) * aspect,
        f32(coord.y) / f32(dims.y) - 0.5
    );

    // Apply zoom and pan
    let c = vec2<f32>(
        uv.x / params.zoom + params.center_x,
        uv.y / params.zoom + params.center_y
    );

    // Mandelbrot iteration: z = z^2 + c
    var z = vec2<f32>(0.0, 0.0);
    var iteration: u32 = 0u;

    for (var i: u32 = 0u; i < params.max_iterations; i++) {
        // z^2 = (a+bi)^2 = a^2 - b^2 + 2abi
        let z_new = vec2<f32>(
            z.x * z.x - z.y * z.y + c.x,
            2.0 * z.x * z.y + c.y
        );
        z = z_new;

        // Check escape condition (|z| > 2)
        if (dot(z, z) > 4.0) {
            break;
        }
        iteration++;
    }

    // Color based on iteration count
    var color: vec3<f32>;
    if (iteration == params.max_iterations) {
        // Inside the set - black
        color = vec3<f32>(0.0, 0.0, 0.0);
    } else {
        // Outside - colorful gradient based on escape time
        let t = f32(iteration) / f32(params.max_iterations);
        // Use HSV for smooth color cycling
        let hue = t * 3.0 % 1.0;  // Cycle through colors
        let saturation = 0.8;
        let value = 1.0 - t * 0.3;  // Slight darkening for distant points
        color = hsv_to_rgb(hue, saturation, value);
    }

    textureStore(output_tex, coord, vec4<f32>(color, 1.0));
}
