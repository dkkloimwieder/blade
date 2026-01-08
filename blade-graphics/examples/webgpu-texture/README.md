# WebGPU Texture Example

> Texture creation, upload, and sampling in blade-graphics

This example demonstrates the complete texture pipeline: creating a texture, uploading data from CPU, and sampling it in a fragment shader.

## What This Example Demonstrates

| Pattern | Description |
|---------|-------------|
| **Texture creation** | `create_texture()` with format and usage flags |
| **Texture view** | Shader-visible handle to texture data |
| **Sampler creation** | Filter modes and address modes |
| **Staging buffer upload** | CPU→GPU texture data transfer |
| **Manual ShaderData impl** | How bindings work under the hood |
| **textureSample()** | WGSL texture sampling in fragment shader |

## Running

### Browser (WebGPU)

```bash
RUSTFLAGS="--cfg blade_wgpu" cargo run-wasm -p blade-graphics --example webgpu-texture
```

Open http://localhost:8000 in a WebGPU-capable browser.

**Chrome on Linux** requires flags:
```bash
google-chrome \
  --user-data-dir=/path/to/repo/.chrome-profile \
  --enable-unsafe-webgpu \
  --enable-features=Vulkan,VulkanFromANGLE \
  --use-angle=vulkan \
  --enable-dawn-features=allow_unsafe_apis \
  http://localhost:8000
```

### Native (WebGPU backend via wgpu)

```bash
RUSTFLAGS="--cfg blade_wgpu" cargo run -p blade-graphics --example webgpu-texture
```

## Expected Output

An **orange and blue checkerboard pattern** filling the window. The edges appear slightly blurred due to linear filtering (see Filter Modes below).

## Code Structure

```
webgpu-texture/
├── main.rs      # Texture setup, upload, and render loop
└── shader.wgsl  # Vertex shader (fullscreen quad) + fragment shader (texture sampling)
```

## Key Patterns Explained

### 1. Texture Creation

```rust
let texture = context.create_texture(gpu::TextureDesc {
    name: "checkerboard",
    format: gpu::TextureFormat::Rgba8Unorm,  // 4 bytes per pixel
    size: extent,
    dimension: gpu::TextureDimension::D2,
    array_layer_count: 1,
    mip_level_count: 1,
    // RESOURCE: can be sampled in shaders
    // COPY: can be copy destination (for upload)
    usage: gpu::TextureUsage::RESOURCE | gpu::TextureUsage::COPY,
    sample_count: 1,
    external: None,
});
```

### 2. Texture View (Shader Access)

```rust
let texture_view = context.create_texture_view(
    texture,
    gpu::TextureViewDesc {
        name: "checkerboard_view",
        format: gpu::TextureFormat::Rgba8Unorm,
        dimension: gpu::ViewDimension::D2,
        subresources: &Default::default(),  // All mips/layers
    },
);
```

### 3. Sampler with Filter Modes

```rust
let sampler = context.create_sampler(gpu::SamplerDesc {
    name: "linear_sampler",
    mag_filter: gpu::FilterMode::Linear,  // Magnification (zoom in)
    min_filter: gpu::FilterMode::Linear,  // Minification (zoom out)
    address_modes: [
        gpu::AddressMode::Repeat,  // U axis
        gpu::AddressMode::Repeat,  // V axis
        gpu::AddressMode::Repeat,  // W axis (3D textures)
    ],
    ..Default::default()
});
```

#### Filter Modes

| Mode | Effect | Use Case |
|------|--------|----------|
| `Linear` | Interpolates between texels | Smooth scaling, photographs |
| `Nearest` | Picks closest texel | Pixel art, crisp edges |

The example uses `Linear` filtering, which causes slight blur at checkerboard edges. For pixel-perfect rendering:

```rust
mag_filter: gpu::FilterMode::Nearest,
min_filter: gpu::FilterMode::Nearest,
```

#### Address Modes

| Mode | Effect |
|------|--------|
| `Repeat` | Tiles the texture (UV wraps around) |
| `ClampToEdge` | Stretches edge pixels |
| `MirrorRepeat` | Tiles with alternating mirrors |

### 4. Staging Buffer Upload

```rust
// Create CPU-visible buffer
let upload_buffer = context.create_buffer(gpu::BufferDesc {
    name: "texture_staging",
    size: texture_data.len() as u64,
    memory: gpu::Memory::Upload,
});

// Copy pixel data to staging buffer
unsafe {
    ptr::copy_nonoverlapping(
        texture_data.as_ptr(),
        upload_buffer.data(),
        texture_data.len(),
    );
}
context.sync_buffer(upload_buffer);  // Mark as dirty

// Transfer to GPU texture
command_encoder.init_texture(texture);  // Initialize layout
if let mut transfer = command_encoder.transfer("upload_texture") {
    transfer.copy_buffer_to_texture(
        upload_buffer.into(),
        bytes_per_row,      // Row stride in bytes
        texture.into(),
        extent,
    );
}
```

### 5. Manual ShaderData Implementation

```rust
struct TextureParams {
    sprite_texture: gpu::TextureView,
    sprite_sampler: gpu::Sampler,
}

impl gpu::ShaderData for TextureParams {
    fn layout() -> gpu::ShaderDataLayout {
        gpu::ShaderDataLayout {
            bindings: vec![
                ("sprite_texture", gpu::ShaderBinding::Texture),
                ("sprite_sampler", gpu::ShaderBinding::Sampler),
            ],
        }
    }

    fn fill(&self, mut ctx: gpu::PipelineContext) {
        use gpu::ShaderBindable as _;
        self.sprite_texture.bind_to(&mut ctx, 0);
        self.sprite_sampler.bind_to(&mut ctx, 1);
    }
}
```

Binding names (`sprite_texture`, `sprite_sampler`) must match WGSL variable names.

### 6. WGSL Texture Sampling

```wgsl
// Texture and sampler bindings (group 0)
var sprite_texture: texture_2d<f32>;
var sprite_sampler: sampler;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(sprite_texture, sprite_sampler, input.uv);
}
```

## Resource Lifecycle

1. **Create** texture, view, sampler
2. **Upload** data via staging buffer + transfer pass
3. **Destroy** staging buffer (no longer needed)
4. **Use** texture view + sampler in render passes
5. **Destroy** view, texture, sampler at shutdown (reverse order)

## Next Steps

After understanding this example, explore:
- **mini** - Compute shaders (mipmap generation with texture storage)
- **bunnymark** - Vertex buffers, instancing, texture + compute
- **game-of-life** - Compute ping-pong with storage textures
