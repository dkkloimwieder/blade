//! Surface creation and management for WebGPU backend

use super::*;

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
        })
    }

    /// Create a surface from a window handle (WASM version)
    #[cfg(target_arch = "wasm32")]
    pub fn create_surface<I>(&self, _window: &I) -> Result<Surface, crate::NotSupportedError> {
        // On WASM, surface creation is typically done via canvas element
        // This is a placeholder - real implementation would use web_sys::HtmlCanvasElement
        Err(crate::NotSupportedError::PlatformNotSupported)
    }

    /// Reconfigure a surface with new dimensions and settings
    pub fn reconfigure_surface(&self, surface: &mut Surface, config: crate::SurfaceConfig) {
        let format = map_color_space_to_format(config.color_space);
        let wgpu_format = map_texture_format(format);

        surface.config.width = config.size.width;
        surface.config.height = config.size.height;
        surface.config.format = wgpu_format;
        surface.config.present_mode = match config.display_sync {
            crate::DisplaySync::Block => wgpu::PresentMode::Fifo,
            crate::DisplaySync::Recent => wgpu::PresentMode::Mailbox,
            crate::DisplaySync::Tear => wgpu::PresentMode::Immediate,
        };
        surface.format = format;

        surface.raw.configure(&self.device, &surface.config);
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
    pub fn acquire_frame(&self) -> Frame {
        let texture = self.raw.get_current_texture().expect("Failed to acquire frame");
        let view = texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let size = [
            self.config.width.min(u16::MAX as u32) as u16,
            self.config.height.min(u16::MAX as u32) as u16,
        ];

        Frame {
            texture,
            view,
            view_key: None,
            target_size: size,
            format: self.format,
        }
    }
}
