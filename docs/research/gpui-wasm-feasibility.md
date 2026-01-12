# GPUI WASM Research: Architecture and Feasibility Analysis

*Research completed: 2026-01-11*
*Related issue: blade-7maq*

## Executive Summary

**Feasibility: MODERATE-HIGH** - GPUI WASM is achievable but requires significant work across 4 major areas:
1. Platform abstraction (new Web platform implementation)
2. Rendering (Blade WebGPU already exists - major advantage)
3. Text rendering (cosmic-text needs WASM config)
4. Async runtime (smol → browser async bridge)

---

## 1. GPUI Architecture Overview

### Crate Location
- **Repository**: `github.com/zed-industries/zed/crates/gpui`
- **Current Version**: 0.2.x (pre-1.0, active development)

### Architectural Layers

GPUI operates across three registers:

1. **Entity-Based State Management** - Application state via smart pointers (Rc-like)
2. **High-Level Declarative Views** - Views implementing `Render` trait, Tailwind-like styling
3. **Low-Level Imperative Elements** - Direct control for specialized rendering (lists, editors)

### Core Source Files
- `crates/gpui/src/app.rs` - Application entry point
- `crates/gpui/src/window.rs` - Window management
- `crates/gpui/src/platform.rs` - Platform trait (lines 1-200)
- `crates/gpui/src/scene.rs` - Rendering scene graph
- `crates/gpui/src/platform/blade/renderer.rs` - Blade GPU renderer

---

## 2. Platform Abstraction Layer

### Platform Trait
The `Platform` trait in `crates/gpui/src/platform.rs` abstracts:
- Window creation/management
- Event loop
- Display enumeration
- Input handling
- Clipboard
- System services

### Current Implementations
| Platform | File | Rendering Backend |
|----------|------|-------------------|
| macOS | `platform/mac/platform.rs` | Metal |
| Linux/Wayland | `platform/linux/wayland/client.rs` | Blade (Vulkan) |
| Linux/X11 | `platform/linux/x11/client.rs` | Blade (Vulkan) |
| Windows | `platform/windows/platform.rs` | DirectWrite |

### WASM Requirements
A new `platform/web/` module would need to implement:
- Canvas-based window abstraction
- Browser event loop integration (requestAnimationFrame)
- DOM event handling (mouse, keyboard, touch)
- Browser clipboard API
- Browser-compatible async executor

---

## 3. Dependencies Analysis

### Total Dependencies: ~80+

### WASM-Compatible (Core)
- `taffy` (layout) - Pure Rust ✓
- `smallvec`, `slotmap` - Pure Rust ✓
- `lyon` (vector graphics) - Pure Rust ✓
- `resvg`, `usvg` (SVG) - Pure Rust ✓
- `serde`, `serde_json` - Pure Rust ✓
- `futures` - WASM compatible ✓

### Platform-Specific (Blockers)

#### Text Rendering
| Platform | System |
|----------|--------|
| macOS | `core-text` (CoreText) |
| Linux | `cosmic-text` + FontConfig |
| Windows | `DirectWriteTextSystem` |
| **WASM** | Need: `cosmic-text` with embedded fonts |

**cosmic-text WASM Status**:
- Has `no_std` support
- Requires embedded fonts (no system font access)
- Tested working in WASM environments (Bevy uses it)

#### Async Runtime
| Platform | Runtime |
|----------|---------|
| Native | `smol` |
| **WASM** | Need: `wasm-bindgen-futures` or custom bridge |

**smol WASM Status**:
- Does NOT support browser WASM
- Missing `AsRawFd/AsRawSocket` types
- Alternative: `wasm-rs-async-executor` or direct JS integration

#### Windowing Dependencies (BLOCK WASM)
- `cocoa`, `core-foundation` (macOS only)
- `wayland-client`, `x11rb` (Linux only)
- `windows` crate (Windows only)
- `objc`, `objc2` (macOS/iOS)

### Rendering: **SOLVED**
| Platform | Backend |
|----------|---------|
| macOS | Metal (or Blade) |
| Linux | Blade (Vulkan) |
| **WASM** | **Blade WebGPU** ✓ |

---

## 4. Blade WebGPU Backend (Key Advantage)

### Location
`blade-graphics/src/webgpu/` - Already implemented!

### Selection
Via cfg flags in `blade-graphics/src/lib.rs`:
```rust
#[cfg_attr(all(blade_wgpu, not(gles)), path = "webgpu/mod.rs")]
```

### Features
- Uses `wgpu` crate with slotmap handles
- Async initialization for WASM (`Context::init_async()`)
- GPU timing with async buffer mapping
- Triple-buffered uniform data
- Bind group caching

### WASM Surface Creation
Already implemented in `webgpu/surface.rs` - creates surface from canvas element.

---

## 5. Required Changes for GPUI WASM

### A. New Web Platform Module
**Location**: `crates/gpui/src/platform/web/`
**Components**:
1. `platform.rs` - Implement `Platform` trait for browsers
2. `window.rs` - Canvas-based window abstraction
3. `events.rs` - DOM event → GPUI event translation
4. `clipboard.rs` - Navigator clipboard API wrapper

### B. Text System for WASM
**Approach**: Configure `cosmic-text` for WASM
1. Embed default fonts (Inter, system fallbacks)
2. Use `cosmic-text` with `no_std` features
3. Implement font loading from HTTP/bundled assets

### C. Async Runtime Bridge
**Options**:
1. **wasm-bindgen-futures**: Standard approach
   - `spawn_local()` for async tasks
   - Works with existing `smol` patterns
2. **Custom scheduler**: Mirror browser event loop
   - `requestAnimationFrame` for render loop
   - `setTimeout` for timers

### D. Blade Integration
**Status**: Already compatible
- GPUI uses Blade on Linux
- Blade has WebGPU backend
- Just need build configuration

---

## 6. Implementation Phases

### Phase 1: Platform Stub
- Fork GPUI
- Add `#[cfg(target_arch = "wasm32")]` platform module
- Stub implementations returning minimal functionality

### Phase 2: Rendering Connection
- Connect GPUI's Blade renderer to blade-graphics WebGPU
- Test basic shape rendering in browser

### Phase 3: Event System
- Browser events → GPUI events
- Mouse, keyboard, resize, focus

### Phase 4: Text Rendering
- Configure cosmic-text for WASM
- Embed required fonts
- Test text layout and rendering

### Phase 5: Async Integration
- Replace smol with browser-compatible executor
- Test async operations (timers, animations)

### Phase 6: Polish
- Full input handling (IME, clipboard)
- Accessibility (ARIA)
- Performance optimization

---

## 7. Estimated Scope

| Component | Effort | Complexity |
|-----------|--------|------------|
| Web Platform Module | High | Medium |
| Text System Config | Medium | Medium |
| Async Runtime Bridge | Medium | High |
| Blade Integration | Low | Low (already exists) |
| Event Translation | Medium | Medium |
| Full Polish | High | High |

---

## 8. Alternative Approaches

### Option A: Full GPUI Port (Recommended)
Fork GPUI, add WASM platform target. Most complete but highest effort.

### Option B: GPUI-lite for Web
Extract core rendering and layout, skip platform features. Faster but limited.

### Option C: GPUI-inspired Web Framework
Build new framework using GPUI patterns + Blade WebGPU. Most flexible, loses GPUI compatibility.

---

## 9. Key Resources

### GPUI
- [GPUI README](https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md)
- [GPUI Cargo.toml](https://github.com/zed-industries/zed/blob/main/crates/gpui/Cargo.toml)
- [GPUI WASM Discussion](https://github.com/zed-industries/zed/discussions/8203)

### Dependencies
- [cosmic-text](https://github.com/pop-os/cosmic-text) - Pure Rust text handling
- [wgpu](https://wgpu.rs/) - Cross-platform GPU abstraction
- [wasm-bindgen-futures](https://docs.rs/wasm-bindgen-futures) - WASM async

### Zed Resources
- [Linux Port Blog](https://zed.dev/blog/zed-decoded-linux-when) - Platform abstraction details
- [DeepWiki GPUI](https://deepwiki.com/zed-industries/zed/2.2-gpui-framework) - Architecture docs

---

## 10. Conclusion

GPUI WASM is feasible with **moderate-high effort**. The biggest advantage is that **Blade already has a working WebGPU backend**, which solves the hardest problem (GPU rendering in browsers).

Key blockers are:
1. **Platform abstraction** - needs new Web platform implementation
2. **Text rendering** - cosmic-text needs WASM configuration
3. **Async runtime** - smol needs browser bridge

The path forward is to fork GPUI and systematically add WASM support, leveraging existing blade-graphics WebGPU infrastructure.
