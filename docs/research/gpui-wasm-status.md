# GPUI WASM Migration Status

*Last updated: 2026-01-12*

## Overview

Goal: Run GPUI-based UI in the browser using Blade's WebGPU backend.

## Current Status: INPUT EVENTS WIRED

gpui-ce now compiles for `wasm32-unknown-unknown` with WebGPU rendering and browser input handling! The web platform includes:
- `WebRenderer` using blade-graphics WebGPU backend
- Full browser event handling (mouse, keyboard, scroll, focus)
- DOM event listeners via wasm-bindgen Closure
- requestAnimationFrame render loop

Next step is implementing full scene rendering (primitives, shaders, text).

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

### 8. WebGPU Renderer Integration (blade-zpdc) ✅
- `platform/web/renderer.rs` - `WebRenderer` using blade-graphics WebGPU backend
  - Async GPU context initialization via `Context::init_async`
  - Surface creation from canvas element
  - Frame acquisition and presentation
  - Basic render pass (clear screen)
  - Single-threaded design (no Send+Sync needed for WASM)
- Local blade-graphics path dependencies in Cargo.toml
  - `blade-graphics`, `blade-util`, `blade-macros` now use workspace versions
- `.cargo/config.toml` updated with `blade_wgpu` cfg for WebGPU backend

### 9. Browser Input Events (blade-up8c) ✅
- `platform/web/events.rs` - Browser event to GPUI event conversion
  - `modifiers_from_mouse_event` / `modifiers_from_keyboard_event` - Extract GPUI Modifiers
  - `mouse_button_from_browser` - Convert browser button numbers to MouseButton
  - `mouse_down_from_browser` / `mouse_up_from_browser` - Mouse click events
  - `mouse_move_from_browser` / `mouse_exit_from_browser` - Mouse movement events
  - `scroll_wheel_from_browser` - Wheel/scroll events with delta mode handling
  - `key_down_from_browser` / `key_up_from_browser` - Keyboard events with Keystroke
  - `is_modifier_key` / `modifiers_changed_from_keyboard` - Modifier key handling
- `platform/web/window.rs` - WebWindow event dispatch methods
  - `dispatch_input` - Send PlatformInput through input_callback
  - `handle_mouse_down` / `handle_mouse_up` - With click count tracking
  - `handle_mouse_move_event` / `handle_wheel` - Movement and scroll
  - `handle_key_down` / `handle_key_up` - Keyboard with modifier detection
  - `handle_mouse_enter` / `handle_mouse_leave` - Hover status
  - `handle_focus` / `handle_blur` - Active status
- Added web-sys features: `Event`, `EventTarget`, `AddEventListenerOptions`

### 10. Event Listener Wiring (blade-8i3f) ✅
- `platform/web/event_listeners.rs` - Attach DOM event listeners to canvas
  - `setup_event_listeners()` - Attach all listeners to canvas, returns EventListeners struct
  - `start_animation_loop()` - Start requestAnimationFrame render loop
  - Mouse listeners: mousedown, mouseup, mousemove, mouseenter, mouseleave, wheel
  - Keyboard listeners: keydown, keyup (canvas must be focusable via tabindex)
  - Focus listeners: focus, blur
  - Window resize listener
  - Context menu prevention for right-click
- Uses wasm-bindgen `Closure` pattern with stored references to keep closures alive
- Performance API for timestamp in click count detection

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

### 1. Full Scene Rendering
- Implement scene primitive rendering in WebRenderer
- Port shader pipelines from BladeRenderer
- Texture atlas integration with WebGPU
- Glyph rendering for text

### 2. Text Rendering
- Embed fonts in WASM binary
- Initialize cosmic-text with embedded fonts
- Replace NoopTextSystem with real text rendering

### 3. Clipboard Integration (Optional)
- Use navigator.clipboard API via web-sys
- Async read/write with Promises

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
- `vendor/gpui-ce/src/platform/web/` - WASM platform implementation
  - `platform.rs` - WebPlatform, WebDisplay, WebKeyboardLayout
  - `window.rs` - WebWindow, WebAtlas, event dispatch methods
  - `dispatcher.rs` - WebDispatcher for task scheduling
  - `renderer.rs` - WebRenderer using blade-graphics WebGPU
  - `events.rs` - Browser event to GPUI event conversion
  - `event_listeners.rs` - DOM event listener attachment via wasm-bindgen Closure
- `vendor/gpui-ce/Cargo.toml` - wasm feature flag, path dependencies to blade
- `vendor/gpui-ce/src/http_stubs.rs` - HTTP client stubs for WASM
- `vendor/gpui-util-wasm/` - WASM-compatible util library
- `blade-graphics/src/webgpu/` - WebGPU backend (working)
- `.cargo/config.toml` - getrandom wasm_js + blade_wgpu cfg

## Related Issues

- blade-7maq: Research (closed)
- blade-fdp1: Fork + platform target (closed)
- blade-t86a: Cargo.toml config (closed)
- blade-nd1t: Fork gpui_util (closed)
- blade-lzpj: Web platform module (closed)
- blade-zpdc: Connect GPUI to blade-graphics WebGPU (closed)
- blade-up8c: Browser input events (closed)
- blade-8i3f: Port GPUI event handling (closed)
- blade-y2lh: Text rendering (open)
- blade-0p1e: Minimal GPUI component demo (open)
