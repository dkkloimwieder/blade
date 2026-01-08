# Blade Examples

## WebGPU-Compatible Examples

These examples run in the browser via WebGPU:

```bash
RUSTFLAGS="--cfg blade_wgpu" cargo run-wasm --example <name>
```

| Example | Description | Features |
|---------|-------------|----------|
| mini | Minimal compute shader | Mipmap generation |
| bunnymark | Rendering benchmark | Compute physics, instanced rendering |
| particle | Particle system with UI | MSAA, egui, render-to-texture |
| frustum-cull | GPU frustum culling | Prefix sum, indirect draw, compute |
| frustum-cull-baseline | Culling baseline (no visibility) | Indirect draw reference |

## Native-Only Examples

These require Vulkan with ray tracing or the full Blade engine:

| Example | Description | Why Native-Only |
|---------|-------------|-----------------|
| init | Asset pipeline demo | Requires blade-render |
| ray-query | Hardware ray tracing | Vulkan RT extensions |
| scene | Full scene editor | blade-render + ray tracing |
| move | Physics movement | Full Blade engine |
| vehicle | Vehicle simulation | Full Blade engine |

## Dependency Matrix

| Example | graphics | macros | egui | asset | render | helper | lib |
|---------|----------|--------|------|-------|--------|--------|-----|
| mini | :white_check_mark: | | | | | | |
| bunnymark | :white_check_mark: | :white_check_mark: | | | | | |
| particle | :white_check_mark: | :white_check_mark: | :white_check_mark: | | | | |
| frustum-cull | :white_check_mark: | :white_check_mark: | | | | | |
| init | :white_check_mark: | :white_check_mark: | | :white_check_mark: | :white_check_mark: | | |
| ray-query | :white_check_mark: (RT) | :white_check_mark: | | | | | |
| scene | :white_check_mark: (RT) | :white_check_mark: | :white_check_mark: | :white_check_mark: | :white_check_mark: | :white_check_mark: | |
| vehicle | | | | | | | :white_check_mark: |
| move | | | | | | :white_check_mark: | :white_check_mark: |

RT = Requires ray tracing (Vulkan only)
