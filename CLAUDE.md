# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Blade is a Rust graphics library providing a layered rendering stack: low-level GPU abstractions (blade-graphics) scaling up to a full engine with ray-tracing and physics (Rapier3D). Shaders are written in WGSL and compiled via Naga to SPIR-V/MSL/GLSL depending on backend.

## Build Commands

```bash
# Build
cargo build                              # Debug build
cargo build --release                    # Release build

# Test
cargo test --workspace --all-features    # Full test suite (CI equivalent)
cargo test -p blade-render --no-default-features  # Test specific crate without defaults
cargo test -p blade-graphics             # Test a single crate

# Format
cargo fmt                                # Format code
cargo fmt -- --check                     # Check formatting (CI uses this)

# Run examples
cargo run --release --example bunnymark  # Rendering benchmark
cargo run --example mini                 # Minimal compute shader
cargo run --example scene                # Full scene editor with physics
cargo run --example ray-query            # Hardware ray tracing

# Alternative backends
RUSTFLAGS="--cfg gles" cargo build       # Build with OpenGL ES backend
RUSTFLAGS="--cfg vulkan" cargo build     # Force Vulkan backend

# WebAssembly
cargo run-wasm --example bunnymark
```

## Architecture

### Crate Hierarchy

```
blade (v0.3.0) - Full engine with physics (Rapier3D)
├── blade-graphics (v0.7.0) - Low-level GPU abstraction
│   ├── vulkan/ - Default on Linux/Windows/Android
│   ├── metal/  - macOS/iOS
│   └── gles/   - WebGL2/fallback
├── blade-render (v0.4.0) - Ray-traced renderer [Vulkan only]
├── blade-asset (v0.2.0) - Task-parallel asset pipeline (Choir scheduler)
├── blade-egui (v0.6.0) - egui 0.32 integration
├── blade-helpers (v0.1.0) - Camera, HUD utilities
├── blade-macros (v0.3.0) - #[derive(ShaderData)], #[derive(Vertex)]
└── blade-util (v0.3.0) - General utilities
```

### Backend Selection

Backends are selected at compile time via conditional compilation in `blade-graphics/src/lib.rs`:
- Metal: macOS, iOS (unless `--cfg vulkan` or `--cfg gles`)
- Vulkan: Linux, Windows, Android, FreeBSD (default)
- GLES: WebAssembly, or when `--cfg gles` is set

### Key Patterns

**Command encoding** follows a hierarchical pattern:
```
CommandEncoder
├── TransferCommandEncoder  (copy operations)
├── ComputeCommandEncoder   (compute dispatches)
│   └── PipelineEncoder     (bound pipeline state)
├── RenderCommandEncoder    (rendering passes)
│   └── PipelineEncoder     (bound pipeline state)
└── AccelerationStructureCommandEncoder (ray tracing builds)
```

**Synchronization** uses timeline semaphores:
- `context.submit(&mut encoder)` returns a `SyncPoint`
- `context.wait_for(&sync_point, timeout_ms)` blocks until completion

**Shader data binding** via `ShaderData` trait or `#[derive(ShaderData)]` macro from blade-macros.

## Platform Support

| Feature | Vulkan | Metal | GLES |
|---------|--------|-------|------|
| Full Engine | Yes | No | No |
| Compute | Yes | Yes | No |
| Ray Tracing | Yes | No | No |

The full Blade engine (with blade-render) requires Vulkan with hardware ray tracing.

## Task Tracking (bd/beads) - MANDATORY

### ⚠️ CRITICAL: CREATE BEADS ISSUE BEFORE DOING ANY WORK

**Before writing ANY code or making ANY changes:**
1. Create a beads issue for the work: `bd create --title="..." --type=task`
2. Mark it in progress: `bd update <id> --status=in_progress`
3. THEN start working

**NO EXCEPTIONS.** Every code change, bug fix, or task must have a beads issue FIRST.

Do NOT use TodoWrite or any other task tracking - ONLY beads.

Run `bd prime` for workflow context, or `bd hooks install` to auto-inject at session start.

### Commands

```bash
bd ready                                 # Find unblocked work
bd create --title="..." --type=task --priority=2  # Create issue BEFORE starting work
bd update <id> --status=in_progress      # Mark as in progress
bd close <id> --reason="..."             # Complete work with reason
bd sync                                  # Sync with git (run at session end)
```

### Workflow

1. **Before any work**: `bd create` → `bd update --status=in_progress`
2. **During work**: Make granular issues for sub-problems as they arise
3. **Completing work**: `bd close` each issue with a reason
4. **Session end**: `git pull --rebase && bd sync && git push`

## Key Files

- `blade-graphics/src/lib.rs` - Public GPU API, types, traits
- `blade-graphics/src/shader.rs` - Shader compilation and binding resolution
- `blade-graphics/src/vulkan/command.rs` - Vulkan command encoding (~1100 lines)
- `blade-render/src/render/mod.rs` - High-level ray-traced renderer
- `docs/ARCHITECTURE.md` - Detailed architecture documentation
