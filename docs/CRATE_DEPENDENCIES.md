# Blade Crate Dependency Graph

> Internal crate relationships and dependency analysis

---

## 1. Crate Hierarchy

```
┌─────────────────────────────────────────────────────────────┐
│                         blade (v0.3.0)                       │
│            Full engine with physics (Rapier3D)               │
└─────────────────────────────────────────────────────────────┘
          │         │          │           │          │
          ▼         ▼          ▼           ▼          ▼
   ┌──────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐
   │  blade-  │ │ blade- │ │ blade- │ │ blade- │ │ blade- │
   │  render  │ │  egui  │ │ asset  │ │helpers │ │  util  │
   │ (v0.4.0) │ │(v0.6.0)│ │(v0.2.0)│ │(v0.1.0)│ │(v0.3.0)│
   │ [native] │ │        │ │        │ │        │ │        │
   └──────────┘ └────────┘ └────────┘ └────────┘ └────────┘
        │            │                     │          │
        │            │                     │          │
        ▼            ▼                     ▼          ▼
   ┌─────────────────────────────────────────────────────┐
   │              blade-graphics (v0.7.0)                 │
   │         Low-level GPU abstraction layer              │
   └─────────────────────────────────────────────────────┘
                              │
                              ▼
                    ┌──────────────────┐
                    │   blade-macros   │
                    │    (v0.3.0)      │
                    │   [proc-macro]   │
                    └──────────────────┘
```

---

## 2. Crate Descriptions

| Crate | Version | Purpose | Platform |
|-------|---------|---------|----------|
| `blade` | 0.3.0 | Full engine with physics, scene management | All |
| `blade-graphics` | 0.7.0 | Low-level GPU abstraction (Vulkan/Metal/GLES/WebGPU) | All |
| `blade-render` | 0.4.0 | Ray-traced renderer, asset loading | Native only |
| `blade-asset` | 0.2.0 | Task-parallel asset pipeline (Choir scheduler) | All |
| `blade-egui` | 0.6.0 | egui 0.32 integration | All |
| `blade-helpers` | 0.1.0 | Camera controls, HUD utilities | Native only |
| `blade-macros` | 0.3.0 | Proc macros: `#[derive(ShaderData)]`, `#[derive(Vertex)]` | All |
| `blade-util` | 0.3.0 | Buffer belt, general utilities | All |

---

## 3. Dependency Matrix

### Internal Dependencies

| Crate | Graphics | Render | Asset | Egui | Helpers | Macros | Util |
|-------|----------|--------|-------|------|---------|--------|------|
| blade | ✓ | ✓* | ✓ | ✓ | ✓ | | ✓ |
| blade-render | ✓ | - | ✓ | | | ✓ | |
| blade-egui | ✓ | | | - | | ✓ | ✓ |
| blade-helpers | | ✓ | | | - | | |
| blade-util | ✓ | | | | | | - |
| blade-asset | | | - | | | | |
| blade-macros | | | | | | - | |
| blade-graphics | | | | | | | |

*`blade-render` is only included on native platforms (not WASM)

### Key Workspace Dependencies

| Dependency | Version | Used By |
|------------|---------|---------|
| `naga` | 28.0 | blade-graphics (shader compilation) |
| `wgpu` | 28.0 | blade-graphics (WebGPU backend) |
| `egui` | 0.32 | blade, blade-egui |
| `choir` | 0.7 | blade, blade-asset, blade-render |
| `bytemuck` | 1.x | All crates (safe transmutes) |
| `glam` | 0.30 | blade-render, blade-helpers |
| `winit` | 0.30 | blade, blade-helpers |
| `rapier3d` | 0.23 | blade (physics) |

---

## 4. Layer Architecture

### Layer 0: Foundation (no internal deps)
- **blade-graphics**: GPU abstraction, backends (Vulkan/Metal/GLES/WebGPU)
- **blade-macros**: Proc macros for derive
- **blade-asset**: Asset pipeline (depends only on choir)

### Layer 1: Utilities (depends on Layer 0)
- **blade-util**: Buffer belt, helpers (depends on blade-graphics)

### Layer 2: Rendering (depends on Layers 0-1)
- **blade-render**: Ray-traced renderer (depends on blade-graphics, blade-asset, blade-macros)
- **blade-egui**: GUI rendering (depends on blade-graphics, blade-macros, blade-util)

### Layer 3: Integration (depends on all layers)
- **blade-helpers**: Camera, HUD (depends on blade-render)
- **blade**: Full engine (depends on everything)

---

## 5. Platform Availability

```
                    Native              WASM
                ┌───────────────┬───────────────┐
blade-graphics  │      ✓        │       ✓       │
blade-macros    │      ✓        │       ✓       │
blade-asset     │      ✓        │       ✓       │
blade-util      │      ✓        │       ✓       │
blade-egui      │      ✓        │       ✓       │
blade-render    │      ✓        │       ✗       │
blade-helpers   │      ✓        │       ✗       │
blade           │      ✓        │   partial*    │
                └───────────────┴───────────────┘

* blade on WASM excludes blade-render and blade-helpers
```

---

## 6. Build Configurations

### Default (Native Vulkan/Metal)
```bash
cargo build                    # Uses native backend
```

### WebGPU Backend
```bash
RUSTFLAGS="--cfg blade_wgpu" cargo build
```

### GLES Backend (Legacy)
```bash
RUSTFLAGS="--cfg gles" cargo build
```

### WASM Target
```bash
cargo build --target wasm32-unknown-unknown
# Automatically uses GLES or WebGPU (with blade_wgpu flag)
```

---

## 7. Feature Flags

### blade-graphics
| Backend | Condition | Description |
|---------|-----------|-------------|
| Vulkan | `cfg(vulkan)` or default on Linux/Windows | Native Vulkan |
| Metal | default on macOS/iOS | Apple Metal |
| GLES | `cfg(gles)` or WASM default | OpenGL ES 3.0 |
| WebGPU | `cfg(blade_wgpu)` | wgpu-based backend |

### blade-render
| Feature | Description |
|---------|-------------|
| `asset_pipeline` | Full asset loading (default) |
| `gltf` | glTF model loading |
| `jpeg` | JPEG texture support |
| `exr` | HDR image support |

---

## 8. Circular Dependency Prevention

The crate structure explicitly prevents circular dependencies:

1. **blade-graphics** has NO internal dependencies
2. **blade-macros** is a proc-macro crate (can't have runtime deps)
3. **blade-util** only depends on blade-graphics
4. **blade-render** and **blade-egui** are peers (neither depends on the other)
5. **blade-helpers** depends on blade-render (one-way)
6. **blade** is the top-level aggregator

This allows incremental compilation and clear API boundaries.

---

## 9. Adding New Crates

When adding a new crate to the workspace:

1. Add to `Cargo.toml` workspace members
2. Determine the layer (0-3) based on dependencies
3. Use workspace dependencies for common crates
4. Add platform conditions if needed (`cfg(not(target_arch = "wasm32"))`)
5. Update this document

---

## 10. External Dependency Versions

All workspace dependencies are pinned in the root `Cargo.toml`:

```toml
[workspace.dependencies]
naga = "28.0"
wgpu = "28"
egui = "0.32"
glam = "0.30"
winit = "0.30"
bytemuck = "1"
choir = "0.7"
```

This ensures consistent versions across all crates.

---

*Generated for blade workspace v0.3.0*
