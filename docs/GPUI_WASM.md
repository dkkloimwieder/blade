# GPUI WASM Integration

This document describes the GPUI web platform implementation, enabling GPUI applications to run in browsers via WebAssembly and WebGPU.

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Getting Started](#getting-started)
4. [Rendering Pipeline](#rendering-pipeline)
5. [Event Handling](#event-handling)
6. [Text Rendering](#text-rendering)
7. [Issues and Resolutions](#issues-and-resolutions)
8. [Known Limitations](#known-limitations)
9. [Future Work](#future-work)

---

## Overview

GPUI WASM allows GPUI applications to run in web browsers by compiling to WebAssembly and using WebGPU for hardware-accelerated rendering. The implementation provides a web-specific platform layer that translates between browser APIs and GPUI's platform abstractions.

### Key Components

| Component | Purpose | Location |
|-----------|---------|----------|
| WebPlatform | Platform trait implementation | `platform/web/platform.rs` |
| WebWindow | Window abstraction over canvas | `platform/web/window.rs` |
| WebRenderer | WebGPU-based scene renderer | `platform/web/renderer.rs` |
| WebTextSystem | Canvas 2D text rasterization | `platform/web/text_system.rs` |
| WebGpuAtlas | GPU texture atlas for glyphs | `platform/web/web_atlas.rs` |
| WebDispatcher | Task scheduling for WASM | `platform/web/dispatcher.rs` |

### Design Decisions

1. **Single-threaded model**: Uses `Rc<RefCell<>>` instead of `Arc<Mutex<>>` since WASM is single-threaded
2. **Canvas 2D for text**: Text is rasterized via Canvas 2D API, then uploaded to GPU atlas
3. **Async GPU initialization**: WebGPU requires async context creation
4. **Deferred command buffer**: All GPU commands batched with offset-based buffer management

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Browser                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐   │
│  │ HTML Canvas  │  │ DOM Events   │  │ requestAnimationFrame│   │
│  └──────┬───────┘  └──────┬───────┘  └──────────┬───────────┘   │
└─────────┼─────────────────┼─────────────────────┼───────────────┘
          │                 │                     │
          ▼                 ▼                     ▼
┌─────────────────────────────────────────────────────────────────┐
│                    WASM (Rust compiled)                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐   │
│  │ WebRenderer  │  │ EventListeners│ │ Animation Loop       │   │
│  │  (WebGPU)    │  │ (wasm-bindgen)│ │ (request_frame)      │   │
│  └──────┬───────┘  └──────┬───────┘  └──────────┬───────────┘   │
│         │                 │                     │               │
│         ▼                 ▼                     ▼               │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                     GPUI Core                            │   │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────────┐ │   │
│  │  │   App   │  │ Window  │  │  Scene  │  │ Text Layout │ │   │
│  │  └─────────┘  └─────────┘  └─────────┘  └─────────────┘ │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow

1. **Input**: Browser events → EventListeners → WebWindow → GPUI input callbacks
2. **Update**: GPUI processes input → Updates model → Marks views dirty
3. **Render**: GPUI builds Scene → WebRenderer draws batches → WebGPU presents

---

## Getting Started

### Prerequisites

- Rust with `wasm32-unknown-unknown` target
- `wasm-bindgen` and `wasm-bindgen-futures`
- Browser with WebGPU support (Chrome 113+, Firefox 121+)

### Example Application

```rust
use gpui::{
    div, prelude::*, px, rgb, App, Application, Bounds, Context,
    IntoElement, ParentElement, Render, Styled, Window, WindowBounds, WindowOptions,
};

struct Counter { count: u32 }

impl Render for Counter {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0x1a1a2e))
            .child(
                div()
                    .w(px(200.))
                    .h(px(100.))
                    .bg(rgb(0x4a4a8e))
                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                        this.count += 1;
                        cx.notify();
                    }))
                    .child(format!("Count: {}", self.count))
            )
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Info).ok();

    // Canvas must exist in DOM
    setup_canvas();

    // Async GPU initialization required
    wasm_bindgen_futures::spawn_local(async {
        let renderer = gpui::WebRenderer::new();
        let canvas = gpui::get_canvas_element("gpui-canvas").unwrap();

        renderer.initialize_async(canvas, config).await.unwrap();
        gpui::set_pending_renderer(renderer);

        Application::new().run(|cx: &mut App| {
            let bounds = Bounds::centered(None, size(px(800.), px(600.)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_window, cx| cx.new(|_| Counter { count: 0 }),
            ).ok();
        });
    });
}
```

### HTML Setup

```html
<!DOCTYPE html>
<html>
<head>
    <style>
        body { margin: 0; overflow: hidden; }
        #gpui-canvas { width: 100%; height: 100vh; display: block; }
    </style>
</head>
<body>
    <canvas id="gpui-canvas"></canvas>
    <script type="module">
        import init from './pkg/your_app.js';
        init();
    </script>
</body>
</html>
```

---

## Rendering Pipeline

### WebRenderer (`platform/web/renderer.rs`)

The WebRenderer handles all GPU rendering using blade-graphics' WebGPU backend.

#### Initialization Flow

```
1. gpu::Context::init_async()     → Create WebGPU context
2. gpu::Surface::create()         → Attach to canvas
3. Create command encoder         → 2 buffers for double-buffering
4. Create GlobalParams buffer     → Viewport size, alpha mode
5. Create render pipelines        → Quad, mono sprite, poly sprite
6. Create instance buffers        → 4096 items each, 256-byte aligned
7. Create atlas                   → WebGpuAtlas for glyph textures
```

#### Render Pipelines

| Pipeline | Purpose | Shader | Instance Data |
|----------|---------|--------|---------------|
| `quad_pipeline` | Solid rectangles | `vs_quad` → `fs_quad` | Bounds, colors, corners |
| `shadow_pipeline` | Blurred shadows | `vs_shadow` → `fs_shadow` | Bounds, blur radius, corners, color |
| `mono_sprite_pipeline` | Grayscale glyphs | `vs_mono_sprite` → `fs_mono_sprite` | Atlas tile, tint color |
| `poly_sprite_pipeline` | Color sprites | `vs_poly_sprite` → `fs_poly_sprite` | Atlas tile, opacity |

#### Draw Flow

```rust
pub fn draw(&self, scene: &Scene) {
    // 1. Wait for previous frame
    if let Some(ref sp) = state.last_sync_point {
        state.gpu.wait_for(sp, 1000);
    }

    // 2. Flush pending atlas uploads
    state.atlas.flush_uploads();

    // 3. Acquire frame from surface
    let frame = state.surface.acquire_frame();

    // 4. Begin encoding
    state.command_encoder.start();

    // 5. Create render pass
    let mut pass = state.command_encoder.render("main", targets);

    // 6. Draw each batch with offset tracking
    for batch in scene.batches() {
        match batch {
            PrimitiveBatch::Quads(quads) => {
                offset = draw_quads_internal(&mut pass, quads, offset, ...);
            }
            PrimitiveBatch::MonochromeSprites { texture_id, sprites } => {
                offset = draw_mono_sprites_internal(&mut pass, sprites, offset, ...);
            }
            // ...
        }
    }

    // 7. Present and submit
    state.command_encoder.present(frame);
    state.last_sync_point = Some(state.gpu.submit(&mut state.command_encoder));
}
```

#### Buffer Management

WebGPU requires storage buffer offsets to be aligned to 256 bytes:

```rust
const STORAGE_BUFFER_ALIGNMENT: u64 = 256;

// After uploading batch data, align offset for next batch
let next_offset = buffer_offset + data_size;
let aligned = (next_offset + 255) & !255;
```

Instance data is uploaded via direct pointer copy:

```rust
unsafe {
    let dst = (buffer.data() as *mut u8).add(offset as usize) as *mut Quad;
    ptr::copy_nonoverlapping(quads.as_ptr(), dst, count);
}
gpu.sync_buffer_range(buffer, offset, data_size);
```

### Shaders (`platform/web/shaders.wgsl`)

#### Global Parameters

```wgsl
struct GlobalParams {
    viewport_size: vec2<f32>,
    premultiplied_alpha: u32,
    pad: u32,
}
```

#### Quad Shader

The quad fragment shader uses signed distance fields for:
- Solid color fill
- Border rendering with anti-aliasing
- Rounded corners

```wgsl
@fragment
fn fs_quad(input: QuadVarying) -> @location(0) vec4<f32> {
    // SDF-based corner and border rendering
    let corner_radius = quad.corner_radii[corner_id];
    let dist = distance_to_rounded_rect(local_pos, half_size, corner_radius);

    // Anti-aliased edge
    let coverage = 1.0 - smoothstep(-0.5, 0.5, dist);

    return blend_color(color, coverage);
}
```

#### Sprite Shaders

**Critical**: `textureSample` must be called before any divergent control flow:

```wgsl
@fragment
fn fs_mono_sprite(input: MonoSpriteVarying) -> @location(0) vec4<f32> {
    // Sample FIRST (must be in uniform control flow)
    let sample = textureSample(t_sprite, s_sprite, input.tile_position).r;

    // Then check clipping (divergent)
    if (any(input.clip_distances < vec4<f32>(0.0))) {
        return vec4<f32>(0.0);
    }

    return blend_color(input.color, sample * input.color.a);
}
```

### Atlas System (`platform/web/web_atlas.rs`)

The atlas manages GPU textures for glyph and sprite storage.

#### Texture Types

| Type | Format | Purpose |
|------|--------|---------|
| Monochrome | R8Unorm | Grayscale glyphs, SVG |
| Polychrome | BGRA8Unorm | Emoji, color images |

#### Allocation

Uses `etagere::BucketedAtlasAllocator` for efficient rectangle packing:

```rust
pub fn get_or_insert_with<F>(&self, key: &AtlasKey, build: F) -> AtlasTile
where
    F: FnOnce() -> (Size<DevicePixels>, Vec<u8>)
{
    // Check cache
    if let Some(tile) = self.tiles_by_key.get(key) {
        return tile.clone();
    }

    // Build tile data
    let (size, data) = build();

    // Allocate space in texture
    let tile = self.allocate(size)?;

    // Queue upload (deferred until flush_uploads)
    self.uploads.push(PendingUpload { tile, data });

    tile
}
```

#### Deferred Uploads

All texture uploads are batched and executed once per frame:

```rust
pub fn flush_uploads(&self) {
    for upload in self.uploads.drain(..) {
        self.gpu.write_texture(
            gpu::TexturePiece { texture: upload.texture, ... },
            &upload.data,
            gpu::TextureDataLayout { bytes_per_row, rows_per_image },
            extent,
        );
    }
}
```

---

## Event Handling

### Event Listeners (`platform/web/event_listeners.rs`)

Event listeners are attached to the canvas element using wasm-bindgen closures.

#### Mouse Events

| Event | Handler | GPUI Event |
|-------|---------|------------|
| mousedown | `handle_mouse_down` | `PlatformInput::MouseDown` |
| mouseup | `handle_mouse_up` | `PlatformInput::MouseUp` |
| mousemove | `handle_mouse_move_event` | `PlatformInput::MouseMove` |
| mouseenter | `handle_mouse_enter` | Sets `is_hovered` flag |
| mouseleave | `handle_mouse_leave` | `PlatformInput::MouseExited` |
| wheel | `handle_wheel` | `PlatformInput::ScrollWheel` |

#### Keyboard Events

```rust
// Canvas must be focusable for keyboard input
canvas.set_tab_index(0);

// Selective preventDefault - allow browser shortcuts
canvas.add_event_listener("keydown", move |event: KeyboardEvent| {
    let key = event.key();

    // Allow F5, F11, F12 through to browser
    if !matches!(key.as_str(), "F5" | "F11" | "F12") {
        event.prevent_default();
    }

    window.handle_key_down(&event);
});
```

### Click Detection (`platform/web/window.rs`)

Double-click detection uses `performance.now()` for timing:

```rust
const DOUBLE_CLICK_MS: f64 = 500.0;

fn handle_mouse_down(&self, event: &MouseEvent, timestamp: f64) {
    let mut state = self.0.lock();

    // Check if this is a repeat click
    let is_same_button = state.last_mouse_down_button == event.button();
    let time_delta = timestamp - state.last_mouse_down_time;

    if is_same_button && time_delta < DOUBLE_CLICK_MS {
        state.click_count += 1;
    } else {
        state.click_count = 1;
    }

    state.last_mouse_down_time = timestamp;
    state.last_mouse_down_button = event.button();
}
```

### Animation Loop

The animation loop uses a self-referential closure pattern:

```rust
pub fn start_animation_loop(window: Rc<WebWindow>) -> Result<(), JsValue> {
    let callback: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let callback_clone = callback.clone();

    let closure = Closure::new(move || {
        window.request_frame();

        // Schedule next frame
        if let Some(ref cb) = *callback_clone.borrow() {
            web_sys::window()
                .unwrap()
                .request_animation_frame(cb.as_ref().unchecked_ref())
                .ok();
        }
    });

    // Store closure and leak to keep alive
    *callback.borrow_mut() = Some(closure);
    std::mem::forget(callback);

    Ok(())
}
```

---

## Text Rendering

### WebTextSystem (`platform/web/text_system.rs`)

Text is rasterized using the Canvas 2D API and uploaded to the GPU atlas.

#### Font Handling

Fonts are loaded via CSS `@font-face`, not through GPUI:

```rust
// System font fallback stack
const SYSTEM_FONT: &str = "system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif";

fn font_id(&mut self, font: &Font) -> FontId {
    let family = if font.family == ".SystemUIFont" {
        SYSTEM_FONT.to_string()
    } else {
        font.family.clone()
    };

    // Create CSS font template
    let css_template = format!("{} {} {{size}}px {}", style, weight, family);

    FontId(self.fonts.len())
}
```

#### Text Measurement

Uses a reference size (1000px) for consistent metrics:

```rust
fn font_metrics(&self, font_id: FontId, font_size: Pixels, scale_factor: f32) -> FontMetrics {
    const REF_SIZE: f32 = 1000.0;

    self.context.set_font(&font_info.css_font(REF_SIZE));
    let metrics = self.context.measure_text("M");

    let actual_size = font_size.0 * scale_factor;
    let scale = actual_size / REF_SIZE;

    FontMetrics {
        ascent: metrics.font_bounding_box_ascent() * scale,
        descent: metrics.font_bounding_box_descent() * scale,
        // ...
    }
}
```

#### Glyph Rasterization

```rust
fn rasterize_glyph(&self, params: &RenderGlyphParams) -> Option<(Bounds<i32>, Vec<u8>)> {
    let scaled_size = params.font_size.0 * params.scale_factor;

    // Resize canvas to fit glyph
    let width = bounds.size.width + 2;  // 2px padding
    let height = bounds.size.height + 2;
    self.canvas.set_width(width as u32);
    self.canvas.set_height(height as u32);

    // Clear and draw
    self.context.clear_rect(0.0, 0.0, width, height);
    self.context.set_fill_style_str("white");
    self.context.fill_text(&char.to_string(), x, y);

    // Extract alpha channel
    let image_data = self.context.get_image_data(0.0, 0.0, width, height)?;
    let rgba = image_data.data();

    // Convert RGBA to grayscale (use alpha channel)
    let grayscale: Vec<u8> = rgba.chunks(4).map(|p| p[3]).collect();

    Some((bounds, grayscale))
}
```

#### Emoji Detection

```rust
fn is_emoji(c: char) -> bool {
    let code = c as u32;
    code >= 0x1F000 ||                    // Emoticons and symbols
    (code >= 0x2600 && code <= 0x27BF) || // Misc symbols
    (code >= 0xFE00 && code <= 0xFE0F)    // Variation selectors
}
```

---

## Issues and Resolutions

### Critical Fixes

| Issue ID | Problem | Resolution |
|----------|---------|------------|
| blade-jf3u | Click interaction not working | `std::time::Instant` panics on WASM. Switched to `web_time::Instant` in gpui-util-wasm |
| blade-mr3i | writeBuffer called every frame | Added `scene_needs_render` flag to Window. Only call `platform_window.draw()` when scene changed |
| blade-7bd | Send+Sync compilation errors | WASM is single-threaded. Added `unsafe impl Send + Sync` for RefCell-based types |
| blade-86h | Error scope async race condition | Skip error scopes on WASM entirely. Rely on browser DevTools for validation errors |
| blade-0ft | Texture upload validation error | bytesPerRow must be 256-byte aligned for multi-row texture copies. Use None for single-row |

### Shader Fixes

| Problem | Resolution |
|---------|------------|
| textureSample in divergent control flow | Move `textureSample()` call before any conditional branching |
| Bind group entry count mismatch | Ensure bind group layout entries exactly match shader `@binding` declarations |

### Performance Issues

| Issue ID | Problem | Resolution |
|----------|---------|------------|
| blade-txe | Chrome 2x slower than Firefox | Browser limitation. Chrome's WebGPU implementation is slower - not actionable |
| blade-aht | LRU cache O(n) per-frame overhead | Fixed cache update algorithm |

### Timing and Measurement

| Problem | Resolution |
|---------|------------|
| `std::time::Instant::now()` panics | Use `web_time::Instant` which uses `performance.now()` internally |
| `measure()` in profiler panics | Replaced with `web_time` crate in gpui-util-wasm |

---

## Known Limitations

### Not Implemented

| Feature | Status | Notes |
|---------|--------|-------|
| Path rendering | TODO | Vector fill/stroke not implemented |
| Underline rendering | TODO | Text underlines not drawn |
| Surface rendering | TODO | Native image surfaces not supported |
| IME input | No-op | `update_ime_position()` does nothing |
| File dialogs | Not possible | Browser security restrictions |
| Clipboard | TODO | Needs `navigator.clipboard` API |
| Custom cursors | TODO | Needs CSS cursor property |

### Platform Constraints

| Constraint | Reason |
|------------|--------|
| Single window only | Platform stores one active window |
| Canvas must pre-exist | GPU surface created from existing element |
| Fonts via CSS only | No font file loading API |
| No subpixel rendering | Canvas 2D returns grayscale only |
| No window positioning | Browser controls window placement |

### Browser Differences

| Aspect | Chrome | Firefox | Notes |
|--------|--------|---------|-------|
| WebGPU performance | ~27 FPS | ~60 FPS | Chrome is ~2x slower (bunnymark) |
| Font metrics | Varies | Varies | `measureText()` results differ |
| Emoji rendering | System | System | Depends on OS emoji font |
| Canvas readback | Slower | Faster | `getImageData()` performance |

---

## Future Work

### Rendering
- [x] Implement shadow rendering with blur
- [ ] Add path fill/stroke support
- [ ] Implement underline rendering
- [ ] Support surface primitives

### Platform Integration
- [ ] Clipboard API (`navigator.clipboard`)
- [ ] Cursor style changes (CSS)
- [ ] IME support for text input
- [ ] Fullscreen API

### Performance
- [ ] Reduce Canvas 2D readback overhead
- [ ] Implement glyph caching across frames
- [ ] Profile and optimize hot paths

### Features
- [ ] Multi-window support (multiple canvases)
- [ ] Font loading API (when browsers support)
- [ ] High-DPI improvements
- [ ] Touch event support

---

## File Reference

| File | Purpose |
|------|---------|
| `platform/web/mod.rs` | Module exports |
| `platform/web/platform.rs` | WebPlatform trait implementation |
| `platform/web/window.rs` | WebWindow and state management |
| `platform/web/renderer.rs` | WebRenderer GPU rendering |
| `platform/web/shaders.wgsl` | WGSL shader source |
| `platform/web/text_system.rs` | Canvas 2D text rasterization |
| `platform/web/web_atlas.rs` | GPU texture atlas |
| `platform/web/event_listeners.rs` | Browser event handling |
| `platform/web/events.rs` | Event type conversion |
| `platform/web/dispatcher.rs` | Task scheduling |
