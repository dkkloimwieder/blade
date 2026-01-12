//! GPUI-style WebGPU UI Demo
//!
//! Demonstrates rendering UI components (divs) in browser via WebGPU/WASM.
//!
//! Features:
//! - Quad rendering with rounded corners and borders
//! - Simple flexbox layout (row/column)
//! - Mouse interaction (hover, click)
//!
//! Run native: RUSTFLAGS="--cfg blade_wgpu" cargo run -p blade-graphics --example gpui-web
//! Run WASM:   RUSTFLAGS="--cfg blade_wgpu" cargo run-wasm -p blade-graphics --example gpui-web

#![allow(irrefutable_let_patterns)]

use blade_graphics as gpu;
use bytemuck::{Pod, Zeroable};
use std::{mem, ptr};

// =============================================================================
// Data Structures
// =============================================================================

/// Global uniforms passed to shader
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    viewport_size: [f32; 2],
    _pad: [f32; 2],
}

/// Per-quad instance data (GPU storage buffer)
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Quad {
    /// Bounds: x, y, width, height (pixels)
    bounds: [f32; 4],
    /// Background color (packed RGBA, little-endian)
    background: u32,
    /// Border color (packed RGBA)
    border_color: u32,
    /// Padding for alignment
    _pad: [f32; 2],
    /// Border widths: top, right, bottom, left
    border_widths: [f32; 4],
    /// Corner radii: top-left, top-right, bottom-right, bottom-left
    corner_radii: [f32; 4],
}

impl Quad {
    fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            bounds: [x, y, w, h],
            background: pack_color(128, 128, 128, 255),
            border_color: 0,
            _pad: [0.0; 2],
            border_widths: [0.0; 4],
            corner_radii: [0.0; 4],
        }
    }

    fn with_background(mut self, r: u8, g: u8, b: u8, a: u8) -> Self {
        self.background = pack_color(r, g, b, a);
        self
    }

    fn with_border(mut self, width: f32, r: u8, g: u8, b: u8, a: u8) -> Self {
        self.border_widths = [width; 4];
        self.border_color = pack_color(r, g, b, a);
        self
    }

    fn with_corner_radius(mut self, radius: f32) -> Self {
        self.corner_radii = [radius; 4];
        self
    }

    fn with_corner_radii(mut self, tl: f32, tr: f32, br: f32, bl: f32) -> Self {
        self.corner_radii = [tl, tr, br, bl];
        self
    }
}

fn pack_color(r: u8, g: u8, b: u8, a: u8) -> u32 {
    (a as u32) << 24 | (b as u32) << 16 | (g as u32) << 8 | (r as u32)
}

// =============================================================================
// Element Tree & Layout System
// =============================================================================

/// RGBA color
#[derive(Clone, Copy, Debug)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Color {
    const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    fn pack(&self) -> u32 {
        pack_color(self.r, self.g, self.b, self.a)
    }

    fn brighten(&self, amount: u8) -> Self {
        Self {
            r: self.r.saturating_add(amount),
            g: self.g.saturating_add(amount),
            b: self.b.saturating_add(amount),
            a: self.a,
        }
    }
}

/// Dimension specification (fixed pixels or auto)
#[derive(Clone, Copy, Debug)]
enum Dimension {
    /// Fixed pixel size
    Px(f32),
    /// Fill available space
    Fill,
    /// Fit content
    Auto,
}

impl Default for Dimension {
    fn default() -> Self {
        Dimension::Auto
    }
}

/// Flexbox direction
#[derive(Clone, Copy, Debug, Default)]
enum FlexDirection {
    #[default]
    Row,
    Column,
}

/// Edge values (padding, margin, border widths)
#[derive(Clone, Copy, Debug, Default)]
struct Edges {
    top: f32,
    right: f32,
    bottom: f32,
    left: f32,
}

impl Edges {
    const fn all(v: f32) -> Self {
        Self { top: v, right: v, bottom: v, left: v }
    }

    const fn xy(x: f32, y: f32) -> Self {
        Self { top: y, right: x, bottom: y, left: x }
    }
}

/// Style properties for an element
#[derive(Clone, Debug)]
struct Style {
    width: Dimension,
    height: Dimension,
    background: Option<Color>,
    border_radius: f32,
    border_width: f32,
    border_color: Option<Color>,
    padding: Edges,
    margin: Edges,
    flex_direction: FlexDirection,
    gap: f32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            width: Dimension::Auto,
            height: Dimension::Auto,
            background: None,
            border_radius: 0.0,
            border_width: 0.0,
            border_color: None,
            padding: Edges::default(),
            margin: Edges::default(),
            flex_direction: FlexDirection::Row,
            gap: 0.0,
        }
    }
}

impl Style {
    fn width(mut self, w: Dimension) -> Self { self.width = w; self }
    fn height(mut self, h: Dimension) -> Self { self.height = h; self }
    fn size(self, w: Dimension, h: Dimension) -> Self { self.width(w).height(h) }
    fn background(mut self, color: Color) -> Self { self.background = Some(color); self }
    fn border_radius(mut self, r: f32) -> Self { self.border_radius = r; self }
    fn border(mut self, width: f32, color: Color) -> Self {
        self.border_width = width;
        self.border_color = Some(color);
        self
    }
    fn padding(mut self, p: Edges) -> Self { self.padding = p; self }
    fn margin(mut self, m: Edges) -> Self { self.margin = m; self }
    fn flex_direction(mut self, d: FlexDirection) -> Self { self.flex_direction = d; self }
    fn flex_row(self) -> Self { self.flex_direction(FlexDirection::Row) }
    fn flex_column(self) -> Self { self.flex_direction(FlexDirection::Column) }
    fn gap(mut self, g: f32) -> Self { self.gap = g; self }
}

/// UI Element with children
struct Element {
    id: usize,
    style: Style,
    children: Vec<Element>,
    interactive: bool,
}

impl Element {
    fn new(style: Style) -> Self {
        static NEXT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        Self {
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            style,
            children: Vec::new(),
            interactive: false,
        }
    }

    fn child(mut self, child: Element) -> Self {
        self.children.push(child);
        self
    }

    fn children(mut self, children: impl IntoIterator<Item = Element>) -> Self {
        self.children.extend(children);
        self
    }

    fn interactive(mut self) -> Self {
        self.interactive = true;
        self
    }
}

/// Computed layout result for an element
#[derive(Clone, Debug)]
struct LayoutRect {
    element_id: usize,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    style: Style,
    interactive: bool,
}

/// Layout the element tree into positioned rectangles
fn layout_element(element: &Element, x: f32, y: f32, available_width: f32, available_height: f32) -> Vec<LayoutRect> {
    let mut result = Vec::new();
    let style = &element.style;

    // Compute element dimensions
    let width = match style.width {
        Dimension::Px(w) => w,
        Dimension::Fill => available_width,
        Dimension::Auto => available_width, // Simplified: auto = fill for containers
    };
    let height = match style.height {
        Dimension::Px(h) => h,
        Dimension::Fill => available_height,
        Dimension::Auto => {
            // Auto height: sum of children + padding
            if element.children.is_empty() {
                40.0 // Default minimum height
            } else {
                // Will be computed after laying out children
                0.0
            }
        }
    };

    // Content area (inside padding)
    let content_x = x + style.padding.left;
    let content_y = y + style.padding.top;
    let content_width = width - style.padding.left - style.padding.right;
    let content_height = height - style.padding.top - style.padding.bottom;

    // Layout children
    let mut child_results = Vec::new();
    let mut current_x = content_x;
    let mut current_y = content_y;
    let mut max_cross = 0.0f32;

    let child_count = element.children.len();
    for (i, child) in element.children.iter().enumerate() {
        let child_style = &child.style;

        // Compute child available size
        let (child_avail_w, child_avail_h) = match style.flex_direction {
            FlexDirection::Row => {
                let remaining_w = (content_x + content_width) - current_x;
                (remaining_w, content_height)
            }
            FlexDirection::Column => {
                let remaining_h = (content_y + content_height) - current_y;
                (content_width, remaining_h)
            }
        };

        // Apply margin
        let child_x = current_x + child_style.margin.left;
        let child_y = current_y + child_style.margin.top;

        let child_rects = layout_element(
            child,
            child_x,
            child_y,
            child_avail_w - child_style.margin.left - child_style.margin.right,
            child_avail_h - child_style.margin.top - child_style.margin.bottom,
        );

        if let Some(first) = child_rects.first() {
            // Update position for next child
            match style.flex_direction {
                FlexDirection::Row => {
                    current_x = child_x + first.width + child_style.margin.right;
                    if i < child_count - 1 {
                        current_x += style.gap;
                    }
                    max_cross = max_cross.max(first.height + child_style.margin.top + child_style.margin.bottom);
                }
                FlexDirection::Column => {
                    current_y = child_y + first.height + child_style.margin.bottom;
                    if i < child_count - 1 {
                        current_y += style.gap;
                    }
                    max_cross = max_cross.max(first.width + child_style.margin.left + child_style.margin.right);
                }
            }
        }

        child_results.extend(child_rects);
    }

    // Compute final height if auto
    let final_height = match style.height {
        Dimension::Auto if !element.children.is_empty() => {
            match style.flex_direction {
                FlexDirection::Row => max_cross + style.padding.top + style.padding.bottom,
                FlexDirection::Column => (current_y - content_y) + style.padding.top + style.padding.bottom,
            }
        }
        Dimension::Auto => 40.0,
        _ => height,
    };

    // Add this element's rect (if it has a background)
    if style.background.is_some() || style.border_width > 0.0 {
        result.push(LayoutRect {
            element_id: element.id,
            x,
            y,
            width,
            height: final_height,
            style: style.clone(),
            interactive: element.interactive,
        });
    }

    result.extend(child_results);
    result
}

/// Convert layout rects to GPU quads
fn layout_to_quads(rects: &[LayoutRect]) -> Vec<Quad> {
    rects
        .iter()
        .map(|rect| {
            let mut quad = Quad::new(rect.x, rect.y, rect.width, rect.height);
            if let Some(bg) = rect.style.background {
                quad.background = bg.pack();
            }
            if let Some(bc) = rect.style.border_color {
                quad.border_color = bc.pack();
                quad.border_widths = [rect.style.border_width; 4];
            }
            quad.corner_radii = [rect.style.border_radius; 4];
            quad
        })
        .collect()
}

// =============================================================================
// Shader Data Bindings
// =============================================================================

struct RenderParams {
    globals: Globals,
    quads: gpu::BufferPiece,
}

impl gpu::ShaderData for RenderParams {
    fn layout() -> gpu::ShaderDataLayout {
        gpu::ShaderDataLayout {
            bindings: vec![
                ("globals", gpu::ShaderBinding::Plain { size: 16 }),
                ("quads", gpu::ShaderBinding::Buffer),
            ],
        }
    }

    fn fill(&self, mut ctx: gpu::PipelineContext) {
        use gpu::ShaderBindable as _;
        self.globals.bind_to(&mut ctx, 0);
        self.quads.bind_to(&mut ctx, 1);
    }
}

// =============================================================================
// Example Application
// =============================================================================

const MAX_QUADS: usize = 256;

struct Example {
    context: gpu::Context,
    surface: gpu::Surface,
    pipeline: gpu::RenderPipeline,
    command_encoder: gpu::CommandEncoder,
    quad_buffer: gpu::Buffer,
    layout_rects: Vec<LayoutRect>,
    prev_sync_point: Option<gpu::SyncPoint>,
    window_size: winit::dpi::PhysicalSize<u32>,
    mouse_pos: [f32; 2],
    hovered_rect: Option<usize>,
    #[cfg(not(target_arch = "wasm32"))]
    start_time: std::time::Instant,
    #[cfg(target_arch = "wasm32")]
    frame_count: u32,
}

impl Example {
    fn make_surface_config(size: winit::dpi::PhysicalSize<u32>) -> gpu::SurfaceConfig {
        gpu::SurfaceConfig {
            size: gpu::Extent {
                width: size.width,
                height: size.height,
                depth: 1,
            },
            usage: gpu::TextureUsage::TARGET,
            display_sync: gpu::DisplaySync::Block,
            ..Default::default()
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn new(window: &winit::window::Window) -> Self {
        let context = unsafe {
            gpu::Context::init(gpu::ContextDesc {
                presentation: true,
                validation: cfg!(debug_assertions),
                timing: false,
                capture: false,
                overlay: false,
                device_id: 0,
            })
            .unwrap()
        };
        Self::init_with_context(context, window)
    }

    #[cfg(target_arch = "wasm32")]
    async fn new_async(window: &winit::window::Window) -> Self {
        let context = gpu::Context::init_async(gpu::ContextDesc {
            presentation: true,
            validation: cfg!(debug_assertions),
            timing: false,
            capture: false,
            overlay: false,
            device_id: 0,
        })
        .await
        .unwrap();
        Self::init_with_context(context, window)
    }

    fn init_with_context(context: gpu::Context, window: &winit::window::Window) -> Self {
        println!("Device: {:?}", context.device_information());

        let window_size = window.inner_size();
        let surface = context
            .create_surface_configured(window, Self::make_surface_config(window_size))
            .unwrap();

        // Load shader
        #[cfg(target_arch = "wasm32")]
        let shader_source = include_str!("shader.wgsl");
        #[cfg(not(target_arch = "wasm32"))]
        let shader_source =
            std::fs::read_to_string("blade-graphics/examples/gpui-web/shader.wgsl").unwrap();

        let shader = context.create_shader(gpu::ShaderDesc {
            source: &shader_source,
        });

        // Create quad buffer (storage buffer for per-quad data)
        let quad_buffer = context.create_buffer(gpu::BufferDesc {
            name: "quads",
            size: (mem::size_of::<Quad>() * MAX_QUADS) as u64,
            memory: gpu::Memory::Shared,
        });

        // Create pipeline
        let render_layout = <RenderParams as gpu::ShaderData>::layout();
        let pipeline = context.create_render_pipeline(gpu::RenderPipelineDesc {
            name: "quad",
            data_layouts: &[&render_layout],
            vertex: shader.at("vs_quad"),
            vertex_fetches: &[],
            fragment: Some(shader.at("fs_quad")),
            primitive: gpu::PrimitiveState {
                topology: gpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            color_targets: &[gpu::ColorTargetState {
                format: surface.info().format,
                blend: Some(gpu::BlendState::ALPHA_BLENDING),
                write_mask: gpu::ColorWrites::ALL,
            }],
            multisample_state: gpu::MultisampleState::default(),
        });

        let command_encoder = context.create_command_encoder(gpu::CommandEncoderDesc {
            name: "main",
            buffer_count: 2,
        });

        // Build demo UI using Element tree
        let layout_rects = Self::build_demo_ui(window_size.width as f32, window_size.height as f32);

        Self {
            context,
            surface,
            pipeline,
            command_encoder,
            quad_buffer,
            layout_rects,
            prev_sync_point: None,
            window_size,
            mouse_pos: [0.0; 2],
            hovered_rect: None,
            #[cfg(not(target_arch = "wasm32"))]
            start_time: std::time::Instant::now(),
            #[cfg(target_arch = "wasm32")]
            frame_count: 0,
        }
    }

    /// Build the demo UI hierarchy using Element tree
    fn build_demo_ui(width: f32, height: f32) -> Vec<LayoutRect> {
        // Define colors
        let dark_bg = Color::rgb(30, 30, 35);
        let panel_bg = Color::rgb(45, 45, 52);
        let border_dark = Color::rgb(60, 60, 70);
        let border_light = Color::rgb(70, 70, 80);

        let blue = Color::rgb(70, 130, 220);
        let green = Color::rgb(70, 180, 130);
        let red = Color::rgb(220, 80, 80);
        let orange = Color::rgb(220, 180, 80);
        let teal = Color::rgb(80, 200, 140);
        let purple = Color::rgb(140, 100, 220);
        let violet = Color::rgb(160, 100, 200);
        let cyan = Color::rgb(100, 160, 200);
        let white_card = Color::rgb(250, 250, 255);

        // Build element tree
        let ui = Element::new(
            Style::default()
                .size(Dimension::Px(width - 40.0), Dimension::Auto)
                .background(dark_bg)
                .border_radius(12.0)
                .border(2.0, border_dark)
                .padding(Edges::all(20.0))
                .flex_column()
                .gap(16.0)
        )
        .child(
            // Button row
            Element::new(
                Style::default()
                    .width(Dimension::Fill)
                    .height(Dimension::Auto)
                    .flex_row()
                    .gap(16.0)
            )
            .child(
                Element::new(
                    Style::default()
                        .size(Dimension::Px(140.0), Dimension::Px(44.0))
                        .background(blue)
                        .border_radius(8.0)
                ).interactive()
            )
            .child(
                Element::new(
                    Style::default()
                        .size(Dimension::Px(140.0), Dimension::Px(44.0))
                        .background(green)
                        .border_radius(8.0)
                ).interactive()
            )
        )
        .child(
            // Nested panel with boxes
            Element::new(
                Style::default()
                    .width(Dimension::Fill)
                    .height(Dimension::Auto)
                    .background(panel_bg)
                    .border_radius(8.0)
                    .border(1.0, border_light)
                    .padding(Edges::all(20.0))
                    .flex_row()
                    .gap(16.0)
            )
            .children([
                Element::new(
                    Style::default()
                        .size(Dimension::Px(60.0), Dimension::Px(60.0))
                        .background(red)
                        .border_radius(6.0)
                ).interactive(),
                Element::new(
                    Style::default()
                        .size(Dimension::Px(60.0), Dimension::Px(60.0))
                        .background(orange)
                        .border_radius(6.0)
                ).interactive(),
                Element::new(
                    Style::default()
                        .size(Dimension::Px(60.0), Dimension::Px(60.0))
                        .background(teal)
                        .border_radius(6.0)
                ).interactive(),
                Element::new(
                    Style::default()
                        .size(Dimension::Px(60.0), Dimension::Px(60.0))
                        .background(purple)
                        .border_radius(6.0)
                ).interactive(),
            ])
        )
        .child(
            // Demo shapes row
            Element::new(
                Style::default()
                    .width(Dimension::Fill)
                    .height(Dimension::Auto)
                    .flex_row()
                    .gap(16.0)
            )
            .child(
                // Pill shape
                Element::new(
                    Style::default()
                        .size(Dimension::Px(160.0), Dimension::Px(40.0))
                        .background(violet)
                        .border_radius(20.0)
                ).interactive()
            )
            .child(
                // Asymmetric corners (manually set later)
                Element::new(
                    Style::default()
                        .size(Dimension::Px(80.0), Dimension::Px(40.0))
                        .background(cyan)
                        .border_radius(20.0) // Will need custom radii
                ).interactive()
            )
            .child(
                // White card
                Element::new(
                    Style::default()
                        .size(Dimension::Px(80.0), Dimension::Px(40.0))
                        .background(white_card)
                        .border_radius(4.0)
                        .border(1.0, Color::rgb(200, 200, 210))
                ).interactive()
            )
        );

        // Layout the tree
        layout_element(&ui, 20.0, 20.0, width - 40.0, height - 40.0)
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.window_size = size;
        self.context
            .reconfigure_surface(&mut self.surface, Self::make_surface_config(size));

        // Rebuild UI for new size
        self.layout_rects = Self::build_demo_ui(size.width as f32, size.height as f32);
    }

    fn on_mouse_move(&mut self, x: f32, y: f32) {
        self.mouse_pos = [x, y];
        self.update_hover();
    }

    fn update_hover(&mut self) {
        let [mx, my] = self.mouse_pos;
        self.hovered_rect = None;

        // Check rects in reverse order (top-most first), only interactive ones
        for (i, rect) in self.layout_rects.iter().enumerate().rev() {
            if !rect.interactive {
                continue;
            }
            if mx >= rect.x && mx < rect.x + rect.width && my >= rect.y && my < rect.y + rect.height {
                self.hovered_rect = Some(i);
                break;
            }
        }
    }

    fn on_click(&mut self) {
        if let Some(idx) = self.hovered_rect {
            let rect = &self.layout_rects[idx];
            #[cfg(target_arch = "wasm32")]
            log::info!("Clicked element {} at ({}, {})", rect.element_id, rect.x, rect.y);
            #[cfg(not(target_arch = "wasm32"))]
            println!("Clicked element {} at ({}, {})", rect.element_id, rect.x, rect.y);
        }
    }

    fn render(&mut self) {
        if let Some(ref sp) = self.prev_sync_point {
            self.context.wait_for(sp, !0);
        }

        // Convert layout rects to quads with hover effect
        let mut render_rects = self.layout_rects.clone();
        if let Some(idx) = self.hovered_rect {
            // Brighten hovered element
            if let Some(ref mut bg) = render_rects[idx].style.background {
                *bg = bg.brighten(25);
            }
        }

        let quads = layout_to_quads(&render_rects);
        let quad_count = quads.len().min(MAX_QUADS);
        unsafe {
            ptr::copy_nonoverlapping(
                quads.as_ptr(),
                self.quad_buffer.data() as *mut Quad,
                quad_count,
            );
        }
        self.context.sync_buffer(self.quad_buffer);

        let globals = Globals {
            viewport_size: [self.window_size.width as f32, self.window_size.height as f32],
            _pad: [0.0; 2],
        };

        let frame = self.surface.acquire_frame();
        self.command_encoder.start();

        if let mut pass = self.command_encoder.render(
            "quads",
            gpu::RenderTargetSet {
                colors: &[gpu::RenderTarget {
                    view: frame.texture_view(),
                    init_op: gpu::InitOp::Clear(gpu::TextureColor::OpaqueBlack),
                    finish_op: gpu::FinishOp::Store,
                }],
                depth_stencil: None,
            },
        ) {
            if let mut rc = pass.with(&self.pipeline) {
                rc.bind(
                    0,
                    &RenderParams {
                        globals,
                        quads: self.quad_buffer.into(),
                    },
                );
                // Draw 6 vertices per quad (2 triangles), quad_count instances
                rc.draw(0, 6, 0, quad_count as u32);
            }
        }

        self.command_encoder.present(frame);
        self.prev_sync_point = Some(self.context.submit(&mut self.command_encoder));

        #[cfg(target_arch = "wasm32")]
        {
            self.frame_count += 1;
        }
    }

    #[allow(dead_code)]
    fn deinit(mut self) {
        if let Some(ref sp) = self.prev_sync_point {
            self.context.wait_for(sp, !0);
        }
        self.context.destroy_buffer(self.quad_buffer);
        self.context
            .destroy_command_encoder(&mut self.command_encoder);
        self.context.destroy_surface(&mut self.surface);
    }
}

// =============================================================================
// Native main()
// =============================================================================
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    env_logger::init();

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let window_attributes = winit::window::Window::default_attributes()
        .with_title("GPUI-Web Demo")
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600));
    #[allow(deprecated)]
    let window = event_loop.create_window(window_attributes).unwrap();

    let mut example = Example::new(&window);

    #[allow(deprecated)]
    event_loop
        .run(|event, target| {
            target.set_control_flow(winit::event_loop::ControlFlow::Poll);
            match event {
                winit::event::Event::AboutToWait => {
                    window.request_redraw();
                }
                winit::event::Event::WindowEvent { event, .. } => match event {
                    winit::event::WindowEvent::Resized(size) => {
                        example.resize(size);
                    }
                    winit::event::WindowEvent::CursorMoved { position, .. } => {
                        example.on_mouse_move(position.x as f32, position.y as f32);
                    }
                    winit::event::WindowEvent::MouseInput {
                        state: winit::event::ElementState::Pressed,
                        button: winit::event::MouseButton::Left,
                        ..
                    } => {
                        example.on_click();
                    }
                    winit::event::WindowEvent::KeyboardInput {
                        event:
                            winit::event::KeyEvent {
                                physical_key: winit::keyboard::PhysicalKey::Code(key_code),
                                state: winit::event::ElementState::Pressed,
                                ..
                            },
                        ..
                    } => {
                        if key_code == winit::keyboard::KeyCode::Escape {
                            target.exit();
                        }
                    }
                    winit::event::WindowEvent::CloseRequested => {
                        target.exit();
                    }
                    winit::event::WindowEvent::RedrawRequested => {
                        example.render();
                    }
                    _ => {}
                },
                _ => {}
            }
        })
        .unwrap();

    example.deinit();
}

// =============================================================================
// WASM main()
// =============================================================================
#[cfg(target_arch = "wasm32")]
fn main() {
    use std::cell::RefCell;
    use std::rc::Rc;
    use winit::platform::web::WindowExtWebSys as _;

    console_error_panic_hook::set_once();
    console_log::init().expect("could not initialize logger");

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let window_attributes = winit::window::Window::default_attributes()
        .with_title("GPUI-Web Demo")
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600));
    #[allow(deprecated)]
    let window = Rc::new(event_loop.create_window(window_attributes).unwrap());

    let canvas = window.canvas().unwrap();
    canvas.set_id(gpu::CANVAS_ID);
    web_sys::window()
        .and_then(|win| win.document())
        .and_then(|doc| doc.body())
        .and_then(|body| body.append_child(&web_sys::Element::from(canvas)).ok())
        .expect("couldn't append canvas to document body");

    let example: Rc<RefCell<Option<Example>>> = Rc::new(RefCell::new(None));
    let init_started: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));

    let example_clone = example.clone();
    let init_started_clone = init_started.clone();
    let window_clone = window.clone();

    #[allow(deprecated)]
    event_loop
        .run(move |event, target| {
            target.set_control_flow(winit::event_loop::ControlFlow::Wait);
            match event {
                winit::event::Event::AboutToWait => {
                    if !*init_started_clone.borrow() {
                        *init_started_clone.borrow_mut() = true;
                        let example_init = example_clone.clone();
                        let window_init = window_clone.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            let ex = Example::new_async(&window_init).await;
                            *example_init.borrow_mut() = Some(ex);
                            log::info!("GPUI-Web demo initialized!");
                        });
                    }
                    window.request_redraw();
                }
                winit::event::Event::WindowEvent { event, .. } => match event {
                    winit::event::WindowEvent::Resized(size) => {
                        if let Some(ref mut ex) = *example.borrow_mut() {
                            ex.resize(size);
                        }
                    }
                    winit::event::WindowEvent::CursorMoved { position, .. } => {
                        if let Some(ref mut ex) = *example.borrow_mut() {
                            ex.on_mouse_move(position.x as f32, position.y as f32);
                        }
                    }
                    winit::event::WindowEvent::MouseInput {
                        state: winit::event::ElementState::Pressed,
                        button: winit::event::MouseButton::Left,
                        ..
                    } => {
                        if let Some(ref mut ex) = *example.borrow_mut() {
                            ex.on_click();
                        }
                    }
                    winit::event::WindowEvent::CloseRequested => {
                        target.exit();
                    }
                    winit::event::WindowEvent::RedrawRequested => {
                        if let Some(ref mut ex) = *example.borrow_mut() {
                            ex.render();
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        })
        .unwrap();
}
