# GPUI Web Example

GPUI running in the browser via WebAssembly and WebGPU.

## Prerequisites

- Rust with `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- wasm-pack: `cargo install wasm-pack`
- A browser with WebGPU support (Chrome 113+, Edge 113+)

## Quick Start

```bash
# Install wasm-pack if you haven't
cargo install wasm-pack

# Build and run
cd examples/gpui-web
wasm-pack build --target web
python3 -m http.server 8080
# Open http://localhost:8080 in Chrome
```

**Why a web server?** WASM runs in a browser, and browsers block loading WASM from `file://` URLs for security. A local HTTP server is required.

## Building Options

### Option 1: wasm-pack (recommended)

```bash
cd examples/gpui-web
wasm-pack build --target web
```

This creates a `pkg/` directory with:
- `gpui_web.js` - JavaScript bindings
- `gpui_web_bg.wasm` - WebAssembly binary

### Option 2: cargo + wasm-bindgen CLI

```bash
# Install wasm-bindgen CLI
cargo install wasm-bindgen-cli

# Build
cargo build --target wasm32-unknown-unknown -p gpui-web --release

# Generate JS bindings
wasm-bindgen target/wasm32-unknown-unknown/release/gpui_web.wasm \
    --out-dir examples/gpui-web/pkg \
    --target web
```

## Serving

After building, serve the directory with any HTTP server:

```bash
# Python (built-in)
python3 -m http.server 8080

# Node.js
npx serve .

# Or any other static file server
```

Then open http://localhost:8080 in a WebGPU-enabled browser (Chrome 113+).
Open DevTools console (F12) to see log messages.

## What You'll See

When working correctly:
1. **Black screen** - The canvas clears to black (WebGPU working!)
2. **Click to cycle colors** - Click on the canvas to cycle through black, white, and transparent
3. **Console logging** - Check browser DevTools console for:
   - Mouse position logs (every 50 pixels)
   - Key press logs (click canvas first to focus)
   - Click position and color index

## Development Status

This is the entry point for GPUI WASM. Current status:

- [x] WASM module loads
- [x] Canvas element acquired
- [x] Logging to console
- [x] WebRenderer initialization
- [x] Event listeners (mouse, keyboard)
- [x] Animation loop
- [ ] Scene rendering (blade-91hg)
- [ ] GPUI component demo (blade-8lxc)

## Troubleshooting

### "WebGPU is not supported"

Make sure you're using a browser with WebGPU:
- Chrome 113+ (check `chrome://flags/#enable-unsafe-webgpu`)
- Edge 113+
- Firefox Nightly (limited support)

### Canvas not found

The HTML must have a canvas with id `gpui-canvas`:
```html
<canvas id="gpui-canvas"></canvas>
```
