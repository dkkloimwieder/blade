# GPUI WASM Migration Status

*Last updated: 2026-01-12*

## Overview

Goal: Run GPUI-based UI in the browser using Blade's WebGPU backend.

## Current Status: WEB PLATFORM IMPLEMENTED

gpui-ce now compiles for `wasm32-unknown-unknown` with a functional web platform implementation! The core platform traits are implemented - next step is connecting to blade-graphics WebGPU and adding full browser event handling.

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

### 7. Web Platform Module (blade-lzpj) ✅
- `platform/web/window.rs` - `WebWindow` implementing `PlatformWindow` trait
  - Canvas-based window management with bounds tracking
  - Callback registration for resize, input, active status, etc.
  - `raw_window_handle` implementation for WebGPU surface creation
  - Device pixel ratio detection from browser
  - `WebAtlas` implementing `PlatformAtlas` for sprite management
- `platform/web/dispatcher.rs` - `WebDispatcher` implementing `PlatformDispatcher`
  - Task queue for main thread execution
  - `dispatch_after` using `setTimeout` via wasm-bindgen
  - Single-threaded execution model for WASM
- `platform/web/platform.rs` - `WebPlatform` implementing `Platform` trait
  - Window creation and management
  - Display bounds from browser viewport
  - Cursor style, clipboard (stubs for now)
  - Prefers-color-scheme detection for dark mode

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

### 1. Connect to blade-graphics WebGPU
- Initialize WebGPU context from canvas element
- Create rendering surface using `Context::create_surface_from_canvas`
- Hook up BladeRenderer for Scene rendering
- Integrate with WebWindow draw() callback

### 2. Browser Event Handling
- Mouse events (click, move, wheel) via web-sys EventListener
- Keyboard events (keydown, keyup, input)
- Touch events for mobile support
- requestAnimationFrame render loop integration

### 3. Text Rendering
- Embed fonts in WASM binary
- Initialize cosmic-text with embedded fonts
- Replace NoopTextSystem with real text rendering

### 4. Clipboard Integration
- Use navigator.clipboard API via web-sys
- Async read/write with Promises

### 5. HTTP Image Loading (Optional)
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
- `vendor/gpui-ce/src/platform/web/` - WASM platform implementation
  - `platform.rs` - WebPlatform, WebDisplay, WebKeyboardLayout
  - `window.rs` - WebWindow, WebAtlas
  - `dispatcher.rs` - WebDispatcher for task scheduling
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
- blade-lzpj: Web platform module (closed)
- blade-zpdc: Connect GPUI to blade-graphics WebGPU (open)
