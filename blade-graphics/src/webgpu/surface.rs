//! Surface creation and management for WebGPU backend

use super::*;

#[cfg(target_arch = "wasm32")]
use web_sys;

/// Maps Blade color space to wgpu texture format
fn map_color_space_to_format(color_space: crate::ColorSpace) -> crate::TextureFormat {
    match color_space {
        crate::ColorSpace::Linear => crate::TextureFormat::Bgra8UnormSrgb,
        crate::ColorSpace::Srgb => crate::TextureFormat::Bgra8Unorm,
    }
}

/// Maps Blade TextureFormat to wgpu::TextureFormat
fn map_texture_format(format: crate::TextureFormat) -> wgpu::TextureFormat {
    // Common formats - extend as needed
    match format {
        crate::TextureFormat::Bgra8Unorm => wgpu::TextureFormat::Bgra8Unorm,
        crate::TextureFormat::Bgra8UnormSrgb => wgpu::TextureFormat::Bgra8UnormSrgb,
        crate::TextureFormat::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
        crate::TextureFormat::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
        _ => wgpu::TextureFormat::Bgra8UnormSrgb, // fallback
    }
}

impl Context {
    /// Create a surface from a window handle
    #[cfg(not(target_arch = "wasm32"))]
    pub fn create_surface<
        I: raw_window_handle::HasWindowHandle + raw_window_handle::HasDisplayHandle,
    >(
        &self,
        window: &I,
    ) -> Result<Surface, crate::NotSupportedError> {
        // Create wgpu surface from window
        // SAFETY: We trust the window outlives the surface.
        // The user is responsible for dropping surface before window.
        let surface = unsafe { self.instance.create_surface_unsafe(
            wgpu::SurfaceTargetUnsafe::from_window(window)
                .map_err(|_| crate::NotSupportedError::PlatformNotSupported)?
        ) }
        .map_err(|e| {
            log::error!("Failed to create surface: {}", e);
            crate::NotSupportedError::PlatformNotSupported
        })?;

        // Get surface capabilities
        let caps = surface.get_capabilities(&self.adapter);
        let format = caps
            .formats
            .first()
            .copied()
            .unwrap_or(wgpu::TextureFormat::Bgra8UnormSrgb);

        // Create initial config (will be reconfigured later)
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: 1,
            height: 1,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps
                .alpha_modes
                .first()
                .copied()
                .unwrap_or(wgpu::CompositeAlphaMode::Auto),
            view_formats: vec![],
        };

        let blade_format = match format {
            wgpu::TextureFormat::Bgra8Unorm => crate::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Bgra8UnormSrgb => crate::TextureFormat::Bgra8UnormSrgb,
            wgpu::TextureFormat::Rgba8Unorm => crate::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Rgba8UnormSrgb => crate::TextureFormat::Rgba8UnormSrgb,
            _ => crate::TextureFormat::Bgra8UnormSrgb,
        };

        Ok(Surface {
            raw: surface,
            config,
            format: blade_format,
            hub: self.hub.clone(),
        })
    }

    /// Create a surface from a window handle (WASM version)
    ///
    /// On WASM, this auto-discovers the canvas element with ID matching `CANVAS_ID` ("blade").
    /// For explicit canvas access, use `create_surface_from_canvas` instead.
    #[cfg(target_arch = "wasm32")]
    pub fn create_surface<I>(&self, _window: &I) -> Result<Surface, crate::NotSupportedError> {
        use wasm_bindgen::JsCast;

        // Auto-discover canvas with the expected ID
        let canvas = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id(crate::CANVAS_ID))
            .ok_or_else(|| {
                log::error!("Canvas element with id '{}' not found", crate::CANVAS_ID);
                crate::NotSupportedError::PlatformNotSupported
            })?
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .map_err(|_| {
                log::error!("Element '{}' is not a canvas", crate::CANVAS_ID);
                crate::NotSupportedError::PlatformNotSupported
            })?;

        self.create_surface_from_canvas(canvas)
    }

    /// Create a surface from an HTML canvas element (WASM only)
    #[cfg(target_arch = "wasm32")]
    pub fn create_surface_from_canvas(
        &self,
        canvas: web_sys::HtmlCanvasElement,
    ) -> Result<Surface, crate::NotSupportedError> {
        let surface = self
            .instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
            .map_err(|e| {
                log::error!("Failed to create surface from canvas: {}", e);
                crate::NotSupportedError::PlatformNotSupported
            })?;

        // Get surface capabilities
        let caps = surface.get_capabilities(&self.adapter);
        let format = caps
            .formats
            .first()
            .copied()
            .unwrap_or(wgpu::TextureFormat::Bgra8UnormSrgb);

        // Create initial config (will be reconfigured later)
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: 1,
            height: 1,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps
                .alpha_modes
                .first()
                .copied()
                .unwrap_or(wgpu::CompositeAlphaMode::Auto),
            view_formats: vec![],
        };

        let blade_format = match format {
            wgpu::TextureFormat::Bgra8Unorm => crate::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Bgra8UnormSrgb => crate::TextureFormat::Bgra8UnormSrgb,
            wgpu::TextureFormat::Rgba8Unorm => crate::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Rgba8UnormSrgb => crate::TextureFormat::Rgba8UnormSrgb,
            _ => crate::TextureFormat::Bgra8UnormSrgb,
        };

        Ok(Surface {
            raw: surface,
            config,
            format: blade_format,
            hub: self.hub.clone(),
        })
    }

    /// Reconfigure a surface with new dimensions and settings
    ///
    /// Note: The texture format is determined at surface creation based on
    /// adapter capabilities. We don't change the format here to ensure
    /// compatibility across browsers (Firefox only supports rgba8unorm).
    pub fn reconfigure_surface(&self, surface: &mut Surface, config: crate::SurfaceConfig) {
        surface.config.width = config.size.width;
        surface.config.height = config.size.height;
        // Keep the format that was detected as supported during surface creation
        // Don't override: Firefox WebGPU doesn't support bgra8unorm-srgb
        surface.config.present_mode = match config.display_sync {
            crate::DisplaySync::Block => wgpu::PresentMode::Fifo,
            crate::DisplaySync::Recent => wgpu::PresentMode::Mailbox,
            crate::DisplaySync::Tear => wgpu::PresentMode::Immediate,
        };

        surface.raw.configure(&self.device, &surface.config);
    }

    /// Destroy a surface.
    ///
    /// In WebGPU/wgpu, surfaces are automatically cleaned up when dropped,
    /// so this is a no-op.
    pub fn destroy_surface(&self, _surface: &mut Surface) {
        // wgpu::Surface is dropped automatically
    }
}

impl Surface {
    /// Get surface info for rendering
    pub fn info(&self) -> crate::SurfaceInfo {
        crate::SurfaceInfo {
            format: self.format,
            alpha: crate::AlphaMode::Ignored,
        }
    }

    /// Acquire the next frame to render to
    ///
    /// The frame's texture view is stored in the hub so it can be used
    /// in render passes. It will be removed when the frame is presented.
    pub fn acquire_frame(&self) -> Frame {
        let texture = self.raw.get_current_texture().expect("Failed to acquire frame");
        let view = texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let size = [
            self.config.width.min(u16::MAX as u32) as u16,
            self.config.height.min(u16::MAX as u32) as u16,
        ];

        // Store view in hub so render pass can look it up by key
        let view_key = self.hub.write().unwrap().texture_views.insert(view);

        Frame {
            texture,
            view_key: Some(view_key),
            target_size: size,
            format: self.format,
        }
    }
}
