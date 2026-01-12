# GPUI Web Example

GPUI running in the browser via WebAssembly and WebGPU.

## Prerequisites

- Rust with `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- wasm-pack: `cargo install wasm-pack`
- A browser with WebGPU support (Chrome 113+, Edge 113+)

## Building

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

## Running

1. Build using one of the methods above
2. Serve the directory with a web server:
   ```bash
   # Python
   python -m http.server 8080

   # Or Node.js
   npx serve .
   ```
3. Open http://localhost:8080 in a WebGPU-enabled browser
4. Open DevTools console to see log messages

## Development Status

This is the entry point for GPUI WASM. Current status:

- [x] WASM module loads
- [x] Canvas element acquired
- [x] Logging to console
- [ ] WebRenderer initialization (blade-1wbn)
- [ ] Event listeners (blade-1wbn)
- [ ] Animation loop (blade-1wbn)
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
