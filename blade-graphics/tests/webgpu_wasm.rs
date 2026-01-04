//! WebGPU WASM tests using wasm-bindgen-test
//!
//! Run with:
//!   RUSTFLAGS="--cfg blade_wgpu" wasm-pack test --headless --chrome blade-graphics
//!
//! Note: Context creation tests require WebGPU support in the browser.

#![cfg(all(target_arch = "wasm32", blade_wgpu))]

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// Test basic WGSL shader parsing using naga (doesn't require GPU)
#[wasm_bindgen_test]
fn test_shader_parse() {
    console_error_panic_hook::set_once();

    let shader_source = r#"
        @vertex
        fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
            return vec4<f32>(0.0, 0.0, 0.0, 1.0);
        }

        @fragment
        fn fs_main() -> @location(0) vec4<f32> {
            return vec4<f32>(1.0, 0.0, 0.0, 1.0);
        }
    "#;

    // Parse the shader using naga
    let module = naga::front::wgsl::parse_str(shader_source)
        .expect("Failed to parse shader");

    // Verify entry points exist
    assert_eq!(module.entry_points.len(), 2, "Should have 2 entry points");

    let ep_names: Vec<&str> = module.entry_points.iter().map(|ep| ep.name.as_str()).collect();
    assert!(ep_names.contains(&"vs_main"), "Should have vs_main entry point");
    assert!(ep_names.contains(&"fs_main"), "Should have fs_main entry point");
}

/// Test shader with bindings
#[wasm_bindgen_test]
fn test_shader_with_bindings() {
    console_error_panic_hook::set_once();

    let shader_source = r#"
        struct Uniforms {
            transform: mat4x4<f32>,
        }

        @group(0) @binding(0) var<uniform> uniforms: Uniforms;
        @group(0) @binding(1) var tex: texture_2d<f32>;
        @group(0) @binding(2) var samp: sampler;

        @fragment
        fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
            let _ = uniforms.transform;
            return textureSample(tex, samp, uv);
        }
    "#;

    let module = naga::front::wgsl::parse_str(shader_source)
        .expect("Failed to parse shader with bindings");

    // Verify global variables exist
    assert!(module.global_variables.len() >= 3, "Should have at least 3 global variables");
}

/// Test ContextDesc can be created with default values
#[wasm_bindgen_test]
fn test_context_desc_default() {
    let desc = blade_graphics::ContextDesc::default();
    assert!(!desc.validation);
    assert!(!desc.presentation);
    assert!(!desc.capture);
    assert!(!desc.overlay);
    assert!(!desc.timing);
    assert_eq!(desc.device_id, 0);
}
