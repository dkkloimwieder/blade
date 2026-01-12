# GPUI WASM Migration Status

*Last updated: 2026-01-11*

## Overview

Goal: Run GPUI-based UI in the browser using Blade's WebGPU backend.

## Current Status: COMPILES

gpui-ce now compiles for `wasm32-unknown-unknown`! The dependency issues have been resolved and the library structure is in place. Next step is implementing the actual WebPlatform functionality.

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

### 4. Cargo.toml WASM Config (blade-t86a) ✅
- Added `wasm` feature flag
- Added wasm-bindgen, web-sys, js-sys dependencies
- Moved libc, num_cpus, smol to native-only section
- Added getrandom 0.3 with wasm_js feature for rand 0.9 compatibility
- Added uuid with js feature for WASM randomness

### 5. gpui_util_wasm Crate (blade-nd1t) ✅
- Created `vendor/gpui-util-wasm/` - WASM-compatible utility library
- Compiles for both native and wasm32-unknown-unknown
- Contains all required types:
  - `ArcCow` with full trait implementations (PartialOrd, Ord, etc.)
  - `ResultExt`, `TryFutureExt` traits for error logging
  - `Deferred`, `defer()` for deferred execution
  - `post_inc`, `measure`, `debug_panic!` utilities
  - HTTP stubs (`Uri`, `StatusCode`, `HeaderValue`, `HttpClient`)
  - Path, serde, size, time utilities

### 6. gpui-ce WASM Compatibility ✅
- Conditional imports for `util` crate (`crate::util::` pattern)
- http_client moved to native-only dependencies
- HTTP image loading gated behind cfg(not(wasm32)) - returns error on WASM
- Platform-specific types (SmolTimer, FutureExt) gated
- Executor realtime priority disabled on WASM (single-threaded)
- `PlatformScreenCaptureFrame` stub for WASM

## Build Commands

```bash
# Build for WASM
cargo check -p gpui-ce --no-default-features --features wasm --target wasm32-unknown-unknown

# Build for native (without wayland/x11)
cargo check -p gpui-ce --no-default-features

# Note: default features (wayland, x11) have pre-existing ashpd dependency issue
```

## Known Limitations (WASM)

1. **HTTP Image Loading**: Not implemented yet. URLs return an error. Use embedded resources instead.
2. **Screen Capture**: Not supported (stub type `()`).
3. **Realtime Priority**: Disabled (WASM is single-threaded).
4. **Timer**: `smol::Timer` not available - need alternative timer implementation.
5. **Filesystem**: No filesystem access on WASM.

## Dependency Compatibility

| Dependency | WASM Status | Notes |
|------------|-------------|-------|
| blade-graphics | ✅ Works | WebGPU backend exists |
| cosmic-text | ✅ Works | Needs embedded fonts |
| taffy (layout) | ✅ Works | Pure Rust |
| lyon (vector) | ✅ Works | Pure Rust |
| rand | ✅ Works | getrandom 0.3 with wasm_js |
| uuid | ✅ Works | js feature enabled |
| gpui_util | ⚠️ Native only | gpui_util_wasm provides WASM subset |
| http_client | ⚠️ Native only | Stubs provided for WASM |
| smol | ❌ Native only | Excluded from WASM build |
| libc, num_cpus | ❌ Native only | Excluded from WASM build |

## Next Steps

### 1. blade-lzpj: Implement WebPlatform
- Canvas-based window management
- Browser event handling (keyboard, mouse, touch)
- requestAnimationFrame render loop
- Clipboard integration via web-sys

### 2. Connect to blade-graphics WebGPU
- Initialize WebGPU context from canvas
- Create rendering surface
- Hook up Scene rendering

### 3. Text Rendering
- Embed fonts in WASM binary
- Initialize cosmic-text with embedded fonts

### 4. HTTP Image Loading (Optional)
- Implement fetch-based image loading using web-sys
- Convert Response to image bytes

## Architecture

```
┌─────────────────────────────────────────────┐
│                  Browser                     │
├─────────────────────────────────────────────┤
│  gpui-ce (WASM build)                       │
│  ├── platform/web/     ← WebPlatform impl   │
│  ├── gpui_util_wasm    ← WASM utilities     │
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
- `vendor/gpui-ce/Cargo.toml` - wasm feature flag, dependencies
- `vendor/gpui-ce/src/http_stubs.rs` - HTTP client stubs for WASM
- `vendor/gpui-util-wasm/` - WASM-compatible util library
- `blade-graphics/src/webgpu/` - WebGPU backend (working)
- `.cargo/config.toml` - getrandom wasm_js backend config

## Related Issues

- blade-7maq: Research (closed)
- blade-fdp1: Fork + platform target (closed)
- blade-t86a: Cargo.toml config (closed)
- blade-nd1t: Fork gpui_util (closed)
- blade-lzpj: Web platform module (ready to start)
