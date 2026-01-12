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
    let c = vec2<f32>(
        px * 2.0 / params.zoom + params.center_x,
        py * 2.0 / params.zoom + params.center_y
    );

    // Mandelbrot iteration
    var z = vec2<f32>(0.0, 0.0);
    var escape_iter: u32 = params.max_iterations;

    for (var i: u32 = 0u; i < params.max_iterations; i++) {
        z = vec2<f32>(z.x*z.x - z.y*z.y + c.x, 2.0*z.x*z.y + c.y);
        if (dot(z, z) > 4.0) {
            escape_iter = i;
            break;
        }
    }

    // Color based on escape iteration
    var color: vec4<f32>;
    if (escape_iter == params.max_iterations) {
        // Never escaped - in the Mandelbrot set - black
        color = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    } else {
        // Escaped - color cycles every ~50 iterations
        let hue = fract(f32(escape_iter) * 0.02);
        let rgb = hsv_to_rgb(hue, 0.85, 1.0);
        color = vec4<f32>(rgb, 1.0);
    }

    textureStore(output_tex, coord, color);
}
