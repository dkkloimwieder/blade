# GPUI WASM Migration Status

*Last updated: 2026-01-11*

## Overview

Goal: Run GPUI-based UI in the browser using Blade's WebGPU backend.

## Current Status: BLOCKED

Blocked on dependency issues. Core infrastructure is in place but doesn't compile yet.

## Completed Work

### 1. Research (blade-7maq) ✅
- Documented GPUI architecture in `docs/research/gpui-wasm-feasibility.md`
- Identified Platform trait, dependencies, blockers
- Feasibility: MODERATE-HIGH

### 2. gpui-ce Fork (blade-fdp1) ✅
- Added gpui-ce as git submodule at `vendor/gpui-ce/`
- Fork: https://github.com/dkkloimwieder/gpui-ce
- Added to blade workspace

### 3. WASM Platform Stubs (blade-fdp1) ✅
- Created `src/platform/web/` module
- Stub `WebPlatform` implementation
- Added `#[cfg(target_arch = "wasm32")]` conditionals

### 4. Cargo.toml WASM Config (blade-t86a) ✅ (partial)
- Added `wasm` feature flag
- Added wasm-bindgen, web-sys, js-sys dependencies
- Moved libc, num_cpus, smol to native-only section

## Blocking Issues

### Primary Blocker: gpui_util depends on smol

```
errno (compile error on wasm32)
└── signal-hook-registry
    └── async-signal
        └── async-process
            └── smol
                └── gpui_util (crates.io)
                    └── gpui-ce
```

**Issue**: blade-nd1t

**Solution**: Fork gpui_util, replace smol with `agnostic_async_executor` which supports WASM via wasm-bindgen.

### Secondary: gpui_http_client tar support

```
async-std
└── zed-async-tar
    └── gpui_http_client
```

**Solution**: Make tar/archive features optional. HTTP itself works fine on WASM (reqwest auto-switches to fetch API).

## Dependency Compatibility

| Dependency | WASM Status | Notes |
|------------|-------------|-------|
| blade-graphics | ✅ Works | WebGPU backend exists |
| reqwest | ✅ Works | Auto-uses fetch on wasm32 |
| cosmic-text | ✅ Works | Needs embedded fonts |
| taffy (layout) | ✅ Works | Pure Rust |
| lyon (vector) | ✅ Works | Pure Rust |
| smol | ❌ Blocked | Use agnostic_async_executor |
| async-std | ❌ Blocked | Make optional |
| libc | ❌ N/A | Already excluded |
| num_cpus | ❌ N/A | Already excluded |

## Next Steps

1. **blade-nd1t**: Fork gpui_util
   - Replace smol with agnostic_async_executor
   - Make fs/command/shell_env features optional (don't work on WASM)
   - Publish as gpui_util_wasm or patch in Cargo.toml

2. **blade-t86a**: Update Cargo.toml to use forked gpui_util

3. **blade-lzpj**: Implement WebPlatform properly
   - Canvas-based window
   - Browser event handling
   - requestAnimationFrame loop

4. Connect to blade-graphics WebGPU backend

## Architecture

```
┌─────────────────────────────────────────────┐
│                  Browser                     │
├─────────────────────────────────────────────┤
│  gpui-ce (WASM build)                       │
│  ├── platform/web/     ← WebPlatform impl   │
│  ├── cosmic-text       ← embedded fonts     │
│  └── blade-graphics    ← WebGPU backend     │
├─────────────────────────────────────────────┤
│  wasm-bindgen / web-sys                     │
├─────────────────────────────────────────────┤
│  WebGPU API (browser native)                │
└─────────────────────────────────────────────┘
```

## Key Files

- `vendor/gpui-ce/` - gpui-ce submodule
- `vendor/gpui-ce/src/platform/web/` - WASM platform stubs
- `vendor/gpui-ce/Cargo.toml` - wasm feature flag
- `blade-graphics/src/webgpu/` - WebGPU backend (working)
- `docs/research/gpui-wasm-feasibility.md` - Initial research

## Related Issues

- blade-7maq: Research (closed)
- blade-fdp1: Fork + platform target (closed)
- blade-t86a: Cargo.toml config (closed, partial)
- blade-nd1t: Fork gpui_util (open, blocking)
- blade-lzpj: Web platform module (blocked)
