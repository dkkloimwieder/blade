//! Platform-specific initialization for WebGPU backend
//!
//! WASM uses async initialization, native uses pollster.

use super::*;

//=============================================================================
// Platform Error
//=============================================================================

#[derive(Debug)]
pub struct PlatformError(pub String);

impl std::fmt::Display for PlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PlatformError {}

//=============================================================================
// WASM Platform (async)
//=============================================================================

#[cfg(target_arch = "wasm32")]
pub async fn create_context(
    _desc: &crate::ContextDesc,
) -> Result<Context, PlatformError> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,
        ..Default::default()
    });

    // wgpu v28: request_adapter returns Result<Adapter, RequestAdapterError>
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .map_err(|e| PlatformError(format!("Adapter request failed: {}", e)))?;

    // wgpu v28: DeviceDescriptor requires experimental_features and trace fields
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Blade WebGPU Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                experimental_features: wgpu::ExperimentalFeatures::default(),
                trace: wgpu::Trace::Off,
            },
        )
        .await
        .map_err(|e| PlatformError(format!("Device request failed: {}", e)))?;

    // Set device lost callback for graceful error handling
    device.set_device_lost_callback(|reason, message| {
        log::error!("WebGPU device lost: {:?} - {}", reason, message);
    });

    let adapter_info = adapter.get_info();
    let wgpu_limits: wgpu::Limits = device.limits();

    Ok(Context {
        instance,
        adapter,
        device,
        queue,
        hub: std::sync::Arc::new(RwLock::new(Hub::new())),
        device_information: crate::DeviceInformation {
            device_name: adapter_info.name,
            driver_name: adapter_info.driver,
            driver_info: adapter_info.driver_info,
            is_software_emulated: adapter_info.device_type == wgpu::DeviceType::Cpu,
        },
        limits: Limits {
            uniform_buffer_alignment: wgpu_limits.min_uniform_buffer_offset_alignment,
            max_bind_groups: wgpu_limits.max_bind_groups,
        },
    })
}

//=============================================================================
// Native Platform (sync with pollster)
//=============================================================================

#[cfg(not(target_arch = "wasm32"))]
pub fn create_context(
    _desc: &crate::ContextDesc,
) -> Result<Context, PlatformError> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });

    // wgpu v28: request_adapter returns Result<Adapter, RequestAdapterError>
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .map_err(|e| PlatformError(format!("Adapter request failed: {}", e)))?;

    // wgpu v28: DeviceDescriptor requires experimental_features and trace fields
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("Blade WebGPU Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
            experimental_features: wgpu::ExperimentalFeatures::default(),
            trace: wgpu::Trace::Off,
        },
    ))
    .map_err(|e| PlatformError(format!("Device request failed: {}", e)))?;

    // Set device lost callback for graceful error handling
    device.set_device_lost_callback(|reason, message| {
        log::error!("WebGPU device lost: {:?} - {}", reason, message);
    });

    let adapter_info = adapter.get_info();
    let wgpu_limits: wgpu::Limits = device.limits();

    Ok(Context {
        instance,
        adapter,
        device,
        queue,
        hub: std::sync::Arc::new(RwLock::new(Hub::new())),
        device_information: crate::DeviceInformation {
            device_name: adapter_info.name,
            driver_name: adapter_info.driver,
            driver_info: adapter_info.driver_info,
            is_software_emulated: adapter_info.device_type == wgpu::DeviceType::Cpu,
        },
        limits: Limits {
            uniform_buffer_alignment: wgpu_limits.min_uniform_buffer_offset_alignment,
            max_bind_groups: wgpu_limits.max_bind_groups,
        },
    })
}
