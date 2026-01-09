//! WASM runner with DWARF debug symbol preservation
//!
//! Usage: cargo run-wasm-debug --example webgpu-triangle
//!
//! This tool builds WASM with debug symbols preserved for source-level
//! debugging in Chrome DevTools (requires DWARF extension).

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

struct Args {
    example: Option<String>,
    bin: Option<String>,
    package: Option<String>,
    release: bool,
    profile: Option<String>,
    port: u32,
}

impl Args {
    fn parse() -> Result<Self> {
        let mut args = std::env::args().skip(1).peekable();
        let mut result = Args {
            example: None,
            bin: None,
            package: None,
            release: false,
            profile: None,
            port: 8000,
        };

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--example" => {
                    result.example = args.next();
                }
                "--bin" => {
                    result.bin = args.next();
                }
                "-p" | "--package" => {
                    result.package = args.next();
                }
                "--release" => {
                    result.release = true;
                }
                "--profile" => {
                    result.profile = args.next();
                }
                "--port" => {
                    if let Some(p) = args.next() {
                        result.port = p.parse().context("invalid port")?;
                    }
                }
                arg if arg.starts_with("--example=") => {
                    result.example = Some(arg.strip_prefix("--example=").unwrap().to_string());
                }
                arg if arg.starts_with("--bin=") => {
                    result.bin = Some(arg.strip_prefix("--bin=").unwrap().to_string());
                }
                arg if arg.starts_with("-p=") => {
                    result.package = Some(arg.strip_prefix("-p=").unwrap().to_string());
                }
                arg if arg.starts_with("--package=") => {
                    result.package = Some(arg.strip_prefix("--package=").unwrap().to_string());
                }
                arg if arg.starts_with("--profile=") => {
                    result.profile = Some(arg.strip_prefix("--profile=").unwrap().to_string());
                }
                arg if arg.starts_with("--port=") => {
                    result.port = arg
                        .strip_prefix("--port=")
                        .unwrap()
                        .parse()
                        .context("invalid port")?;
                }
                _ => {}
            }
        }

        if result.example.is_none() && result.bin.is_none() && result.package.is_none() {
            bail!("Usage: cargo run-wasm-debug --example <name> [--package <pkg>] [--release] [--port <port>]");
        }

        Ok(result)
    }

    fn binary_name(&self) -> String {
        self.example
            .clone()
            .or_else(|| self.bin.clone())
            .or_else(|| self.package.clone())
            .unwrap()
    }

    fn profile_dir(&self) -> &str {
        if self.release {
            "release"
        } else if let Some(ref profile) = self.profile {
            if profile == "dev" {
                "debug"
            } else {
                profile
            }
        } else {
            "debug"
        }
    }

    fn cargo_args(&self) -> Vec<String> {
        let mut args = vec![
            "build".to_string(),
            "--target".to_string(),
            "wasm32-unknown-unknown".to_string(),
        ];

        if let Some(ref example) = self.example {
            args.push("--example".to_string());
            args.push(example.clone());
        }
        if let Some(ref bin) = self.bin {
            args.push("--bin".to_string());
            args.push(bin.clone());
        }
        if let Some(ref package) = self.package {
            args.push("-p".to_string());
            args.push(package.clone());
        }
        if self.release {
            args.push("--release".to_string());
        } else if let Some(ref profile) = self.profile {
            args.push("--profile".to_string());
            args.push(profile.clone());
        }

        args
    }
}

fn find_wasm_file(args: &Args) -> Result<PathBuf> {
    let target_dir = PathBuf::from("target/wasm32-unknown-unknown").join(args.profile_dir());

    // Binary name keeps hyphens in the output filename
    let binary_name = args.binary_name();

    let wasm_path = if args.example.is_some() {
        target_dir.join("examples").join(format!("{}.wasm", binary_name))
    } else {
        target_dir.join(format!("{}.wasm", binary_name))
    };

    if !wasm_path.exists() {
        bail!("WASM file not found at: {}", wasm_path.display());
    }

    Ok(wasm_path)
}

fn run_wasm_bindgen(wasm_path: &Path, out_dir: &Path) -> Result<()> {
    let mut bindgen = wasm_bindgen_cli_support::Bindgen::new();

    bindgen
        .keep_debug(true) // Preserve DWARF debug symbols
        .web(true)
        .map_err(|e| anyhow::anyhow!("bindgen web mode error: {}", e))?
        .typescript(false)
        .input_path(wasm_path)
        .generate(out_dir)
        .map_err(|e| anyhow::anyhow!("bindgen generate error: {}", e))?;

    Ok(())
}

fn generate_html(out_dir: &Path, name: &str) -> Result<()> {
    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>{name} (Debug)</title>
    <style>
        body {{ margin: 0; }}
        canvas {{ width: 100vw; height: 100vh; display: block; }}
    </style>
</head>
<body>
    <script type="module">
        import init from './{name}.js';
        window.addEventListener('load', () => init('./{name}_bg.wasm'));
    </script>
</body>
</html>
"#,
        name = name
    );

    std::fs::write(out_dir.join("index.html"), html)?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse()?;

    // 1. Build WASM
    println!("Building WASM with debug info...");
    let cargo_args = args.cargo_args();
    let status = Command::new("cargo")
        .args(&cargo_args)
        .status()
        .context("failed to run cargo build")?;

    if !status.success() {
        bail!("cargo build failed");
    }

    // 2. Find WASM file
    let wasm_path = find_wasm_file(&args)?;
    println!("Found WASM: {}", wasm_path.display());

    // 3. Create output directory
    let out_dir = PathBuf::from("target/wasm-debug-out");
    if out_dir.exists() {
        std::fs::remove_dir_all(&out_dir)?;
    }
    std::fs::create_dir_all(&out_dir)?;

    // 4. Run wasm-bindgen with DWARF preservation
    println!("Running wasm-bindgen with --keep-debug...");
    run_wasm_bindgen(&wasm_path, &out_dir)?;

    // 5. Generate HTML (wasm-bindgen keeps hyphens in output filenames)
    generate_html(&out_dir, &args.binary_name())?;

    // 6. Serve
    println!("\n============================================");
    println!("DWARF Debug Build Ready!");
    println!("============================================");
    println!("Server: http://127.0.0.1:{}", args.port);
    println!("Output: {}", out_dir.display());
    println!();
    println!("For source-level debugging:");
    println!("  1. Install Chrome DWARF extension");
    println!("  2. Open DevTools > Sources");
    println!("  3. Look for file:// sources");
    println!("============================================\n");

    // Use Python's http.server as it's simpler and more reliable
    let status = Command::new("python3")
        .args(["-m", "http.server", &args.port.to_string(), "-d", out_dir.to_str().unwrap()])
        .status()
        .context("failed to start http server (requires python3)")?;

    if !status.success() {
        bail!("http server exited with error");
    }

    Ok(())
}
