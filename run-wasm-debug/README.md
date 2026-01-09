# run-wasm-debug

WASM runner with DWARF debug symbol preservation for source-level Rust debugging in Chrome DevTools.

## Usage

```bash
RUSTFLAGS="--cfg blade_wgpu" cargo run-wasm-debug --example webgpu-triangle -p blade-graphics
```

Then open http://127.0.0.1:8000 in Chrome.

## Options

| Flag | Description |
|------|-------------|
| `--example <name>` | Build and run an example |
| `--bin <name>` | Build and run a binary |
| `-p, --package <pkg>` | Specify package |
| `--release` | Use release profile |
| `--profile <name>` | Use custom profile |
| `--port <port>` | Server port (default: 8000) |

## How It Works

1. Builds WASM with `cargo build --target wasm32-unknown-unknown`
2. Runs wasm-bindgen with `keep_debug(true)` to preserve DWARF debug symbols
3. Generates full-screen HTML with proper initialization
4. Serves output via Python's http.server

## Output

Files are generated in `target/wasm-debug-out/`:

```
index.html           # Generated HTML page
<name>.js            # wasm-bindgen JavaScript bindings
<name>_bg.wasm       # WASM binary with DWARF debug info
```

## Requirements

- **Chrome** with [C/C++ DevTools Support (DWARF)](https://chromewebstore.google.com/detail/pdcpmagijalfljmkmjngeonclgbbannb) extension
- **Python 3** for the HTTP server

## Debugging Workflow

1. Run the command above
2. Open http://127.0.0.1:8000 in Chrome
3. Open DevTools (F12) > Sources tab
4. Look for `file://` sources in the left panel
5. Set breakpoints in your Rust source files
6. Refresh the page to hit breakpoints

## Comparison with Other Tools

| Tool | DWARF Debug | Use Case |
|------|-------------|----------|
| `cargo run-wasm` | No | Quick testing |
| `wasm-server-runner` | No | Development with auto-reload |
| `cargo run-wasm-debug` | Yes | Source-level debugging |

## Notes

- Debug builds produce large WASM files (~70MB) due to DWARF info
- DWARF preservation depends on walrus crate maturity
- WebGPU examples require `RUSTFLAGS="--cfg blade_wgpu"`
