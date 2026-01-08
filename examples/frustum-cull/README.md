# GPU Frustum Culling Example

Demonstrates GPU-driven frustum culling using compute shaders with parallel prefix sum compaction.

## What It Does

Renders 8,000 cubes (20x20x20 grid) with GPU-based visibility testing. Cubes outside the camera's view frustum are culled entirely - they don't consume any draw call overhead.

## Algorithm: 3-Pass Parallel Prefix Sum

The culling uses a 3-pass compute shader algorithm for **stable, deterministic compaction**:

```
Pass 1: cs_cull (32 workgroups x 256 threads)
├── Test each object against 6 frustum planes
├── Compute workgroup-local prefix sums using shared memory
└── Store workgroup totals

Pass 2: cs_scan_workgroups (1 workgroup x 64 threads)
├── Prefix sum of workgroup totals
├── Compute global offsets per workgroup
└── Set indirect.instance_count = total visible

Pass 3: cs_scatter (32 workgroups x 256 threads)
└── Scatter visible object indices to compacted output
```

The vertex shader then uses `visible_indices[instance_idx]` to look up object data.

## Why Not atomicAdd?

A simpler approach would use `atomicAdd(&count, 1)` to get output slots:

```wgsl
// BROKEN - causes visual artifacts!
let slot = atomicAdd(&indirect.instance_count, 1u);
visible_indices[slot] = obj_idx;
```

This causes **visual artifacts** because GPU thread execution order is non-deterministic. The same `instance_idx` maps to different objects each frame, causing cubes to flash and swap positions.

The prefix sum approach computes deterministic output indices based on object index, not thread scheduling.

## Running

```bash
# Native (Vulkan/Metal)
cargo run --example frustum-cull

# WebAssembly (WebGPU)
RUSTFLAGS="--cfg blade_wgpu" cargo run-wasm --example frustum-cull
```

## Files

| File | Description |
|------|-------------|
| `main.rs` | Application setup, buffer creation, render loop |
| `cull.wgsl` | 3-pass compute shaders (cull, scan, scatter) |
| `render.wgsl` | Vertex/fragment shaders with instanced rendering |

## GPU Resources

| Buffer | Size | Purpose |
|--------|------|---------|
| `objects` | 625 KB | Model matrices + colors (8000 objects) |
| `bounds` | 125 KB | Bounding spheres for culling |
| `visible_indices` | 31 KB | Compacted visible object indices |
| `visibility` | 31 KB | Per-object visibility flags |
| `local_prefix` | 31 KB | Workgroup-local prefix sums |
| `workgroup_totals` | 128 B | Sum per workgroup (32 values) |
| `workgroup_offsets` | 128 B | Prefix sum of totals |
| `indirect` | 16 B | DrawIndirect struct |

Total: ~0.83 MB GPU memory

## Performance

With frustum culling enabled, only visible cubes are processed by the vertex shader. The indirect draw call reads `instance_count` directly from GPU memory, avoiding CPU readback.

Typical culling rates: 30-70% of cubes culled depending on camera angle.
