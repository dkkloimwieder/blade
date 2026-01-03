fn main() {
    cargo_run_wasm::run_wasm_with_css(
        "body { margin: 0px; } \
         canvas { width: 100vw; height: 100vh; display: block; }",
    );
}
