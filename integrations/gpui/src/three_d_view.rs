//! 🧊 3D Renderer Component
//!
//! Strategy: `wgpu` renders a 3D scene to an off-screen texture → read pixels back → RGBA → GPUI.
//! Uses WGSL shaders for a colored rotating cube with perspective projection.

use glam::{Mat4, Vec3};
use gpui::*;
use gpui_component::StyledExt;
use image::RgbaImage;
use std::sync::{Arc, Mutex};
use wgpu::util::DeviceExt;

use crate::bitmap_utils;

/// Vertex for our 3D mesh
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

pub struct ThreeDView {
    rendered_frame: Arc<Mutex<Option<RgbaImage>>>,
    rotation_angle: f32,
    width: u32,
    height: u32,
}

impl ThreeDView {
    pub fn new(_window: &Window, cx: &mut Context<Self>) -> Self {
        let width = 800u32;
        let height = 600u32;
        let frame = Arc::new(Mutex::new(None));
        let frame_clone = frame.clone();

        // Initial render
        cx.spawn(async move |this: WeakEntity<ThreeDView>, cx| {
            let rendered = render_3d_scene(width, height, 0.0).await;
            if let Ok(img) = rendered {
                *frame_clone.lock().unwrap() = Some(img);
                cx.update(|cx| {
                    this.update(cx, |_: &mut ThreeDView, cx: &mut Context<ThreeDView>| cx.notify())
                }).ok();
            }
        })
        .detach();

        Self {
            rendered_frame: frame,
            rotation_angle: 0.0,
            width,
            height,
        }
    }

    fn rotate(&mut self, delta: f32, cx: &mut Context<Self>) {
        self.rotation_angle += delta;
        let angle = self.rotation_angle;
        let w = self.width;
        let h = self.height;
        let frame = self.rendered_frame.clone();

        cx.spawn(async move |this: WeakEntity<ThreeDView>, cx| {
            let rendered = render_3d_scene(w, h, angle).await;
            if let Ok(img) = rendered {
                *frame.lock().unwrap() = Some(img);
                cx.update(|cx| {
                    this.update(cx, |_: &mut ThreeDView, cx: &mut Context<ThreeDView>| cx.notify())
                }).ok();
            }
        })
        .detach();
    }
}

impl Render for ThreeDView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let frame = self.rendered_frame.lock().unwrap();
        let rotate_left  = cx.listener(|this, _: &MouseDownEvent, _, cx| this.rotate(-15.0_f32.to_radians(), cx));
        let rotate_right = cx.listener(|this, _: &MouseDownEvent, _, cx| this.rotate( 15.0_f32.to_radians(), cx));

        div()
            .v_flex()
            .gap_3()
            .items_center()
            .p_4()
            .child(
                div()
                    .text_color(rgb(0xcdd6f4))
                    .text_xl()
                    .child("🧊 3D Renderer (wgpu)"),
            )
            .child(
                div()
                    .w(px(self.width as f32))
                    .h(px(self.height as f32))
                    .bg(rgb(0x11111b))
                    .rounded(px(8.))
                    .overflow_hidden()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(if let Some(ref rgba_img) = *frame {
                        let source = bitmap_utils::rgba_to_gpui_image(rgba_img);
                        div().child(
                            img(source)
                                .w(px(self.width as f32))
                                .h(px(self.height as f32))
                                .object_fit(ObjectFit::Contain),
                        )
                    } else {
                        div()
                            .text_color(rgb(0x6c7086))
                            .child("Rendering 3D scene...")
                    }),
            )
            .child(
                // Rotation controls
                div()
                    .h_flex()
                    .gap_2()
                    .child(
                        div()
                            .px_4()
                            .py_2()
                            .bg(rgb(0xf38ba8))
                            .text_color(rgb(0x1e1e2e))
                            .rounded(px(6.))
                            .cursor_pointer()
                            .child("⟲ Rotate Left")
                            .on_mouse_down(MouseButton::Left, rotate_left),
                    )
                    .child(
                        div()
                            .px_4()
                            .py_2()
                            .bg(rgb(0xa6e3a1))
                            .text_color(rgb(0x1e1e2e))
                            .rounded(px(6.))
                            .cursor_pointer()
                            .child("⟳ Rotate Right")
                            .on_mouse_down(MouseButton::Left, rotate_right),
                    )
                    .child(
                        div()
                            .text_color(rgb(0xa6adc8))
                            .child(format!("Angle: {:.1}\u{00b0}", self.rotation_angle.to_degrees())),
                    ),
            )
    }
}

/// WGSL shader for a simple colored triangle/cube
const SHADER_SRC: &str = r#"
struct Uniforms {
    mvp: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.position = uniforms.mvp * vec4<f32>(position, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_main(@location(0) color: vec3<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>(color, 1.0);
}
"#;

/// Render a 3D cube to an RGBA image using wgpu (off-screen)
async fn render_3d_scene(
    width: u32,
    height: u32,
    rotation: f32,
) -> anyhow::Result<RgbaImage> {
    // 1. Initialize wgpu
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .ok_or_else(|| anyhow::anyhow!("No GPU adapter found"))?;

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .await?;

    // 2. Create off-screen render target
    let texture_desc = wgpu::TextureDescriptor {
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        label: Some("offscreen_texture"),
        view_formats: &[],
    };
    let texture = device.create_texture(&texture_desc);
    let texture_view = texture.create_view(&Default::default());

    // 3. Cube vertices (colored) — front + back faces
    let vertices: &[Vertex] = &[
        // Front face
        Vertex { position: [-0.5, -0.5,  0.5], color: [1.0, 0.0, 0.0] },
        Vertex { position: [ 0.5, -0.5,  0.5], color: [0.0, 1.0, 0.0] },
        Vertex { position: [ 0.5,  0.5,  0.5], color: [0.0, 0.0, 1.0] },
        Vertex { position: [-0.5, -0.5,  0.5], color: [1.0, 0.0, 0.0] },
        Vertex { position: [ 0.5,  0.5,  0.5], color: [0.0, 0.0, 1.0] },
        Vertex { position: [-0.5,  0.5,  0.5], color: [1.0, 1.0, 0.0] },
        // Back face
        Vertex { position: [-0.5, -0.5, -0.5], color: [1.0, 0.0, 1.0] },
        Vertex { position: [-0.5,  0.5, -0.5], color: [0.0, 1.0, 1.0] },
        Vertex { position: [ 0.5,  0.5, -0.5], color: [1.0, 1.0, 1.0] },
        Vertex { position: [-0.5, -0.5, -0.5], color: [1.0, 0.0, 1.0] },
        Vertex { position: [ 0.5,  0.5, -0.5], color: [1.0, 1.0, 1.0] },
        Vertex { position: [ 0.5, -0.5, -0.5], color: [0.5, 0.5, 0.5] },
        // Right face
        Vertex { position: [ 0.5, -0.5,  0.5], color: [0.0, 1.0, 0.0] },
        Vertex { position: [ 0.5, -0.5, -0.5], color: [0.5, 0.5, 0.5] },
        Vertex { position: [ 0.5,  0.5, -0.5], color: [1.0, 1.0, 1.0] },
        Vertex { position: [ 0.5, -0.5,  0.5], color: [0.0, 1.0, 0.0] },
        Vertex { position: [ 0.5,  0.5, -0.5], color: [1.0, 1.0, 1.0] },
        Vertex { position: [ 0.5,  0.5,  0.5], color: [0.0, 0.0, 1.0] },
        // Left face
        Vertex { position: [-0.5, -0.5, -0.5], color: [1.0, 0.0, 1.0] },
        Vertex { position: [-0.5, -0.5,  0.5], color: [1.0, 0.0, 0.0] },
        Vertex { position: [-0.5,  0.5,  0.5], color: [1.0, 1.0, 0.0] },
        Vertex { position: [-0.5, -0.5, -0.5], color: [1.0, 0.0, 1.0] },
        Vertex { position: [-0.5,  0.5,  0.5], color: [1.0, 1.0, 0.0] },
        Vertex { position: [-0.5,  0.5, -0.5], color: [0.0, 1.0, 1.0] },
        // Top face
        Vertex { position: [-0.5,  0.5,  0.5], color: [1.0, 1.0, 0.0] },
        Vertex { position: [ 0.5,  0.5,  0.5], color: [0.0, 0.0, 1.0] },
        Vertex { position: [ 0.5,  0.5, -0.5], color: [1.0, 1.0, 1.0] },
        Vertex { position: [-0.5,  0.5,  0.5], color: [1.0, 1.0, 0.0] },
        Vertex { position: [ 0.5,  0.5, -0.5], color: [1.0, 1.0, 1.0] },
        Vertex { position: [-0.5,  0.5, -0.5], color: [0.0, 1.0, 1.0] },
        // Bottom face
        Vertex { position: [-0.5, -0.5, -0.5], color: [1.0, 0.0, 1.0] },
        Vertex { position: [ 0.5, -0.5, -0.5], color: [0.5, 0.5, 0.5] },
        Vertex { position: [ 0.5, -0.5,  0.5], color: [0.0, 1.0, 0.0] },
        Vertex { position: [-0.5, -0.5, -0.5], color: [1.0, 0.0, 1.0] },
        Vertex { position: [ 0.5, -0.5,  0.5], color: [0.0, 1.0, 0.0] },
        Vertex { position: [-0.5, -0.5,  0.5], color: [1.0, 0.0, 0.0] },
    ];

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    // 4. MVP Matrix
    let aspect = width as f32 / height as f32;
    let proj = Mat4::perspective_rh(45.0_f32.to_radians(), aspect, 0.1, 100.0);
    let view = Mat4::look_at_rh(Vec3::new(0.0, 1.0, 3.0), Vec3::ZERO, Vec3::Y);
    let model = Mat4::from_rotation_y(rotation) * Mat4::from_rotation_x(rotation * 0.7);
    let mvp = proj * view * model;

    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Uniform Buffer"),
        contents: bytemuck::cast_slice(&mvp.to_cols_array()),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    // 5. Shader + Pipeline
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Shader"),
        source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
        label: Some("bind_group_layout"),
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
        label: Some("bind_group"),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x3,
                    },
                    wgpu::VertexAttribute {
                        offset: 12,
                        shader_location: 1,
                        format: wgpu::VertexFormat::Float32x3,
                    },
                ],
            }],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });

    // 6. Render
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.07, g: 0.07, b: 0.11, a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        render_pass.set_pipeline(&pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.draw(0..vertices.len() as u32, 0..1);
    }

    // 7. Copy texture → CPU buffer
    let bytes_per_row = (4 * width + 255) & !255; // align to 256
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Buffer"),
        size: (bytes_per_row * height) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &output_buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );

    queue.submit(Some(encoder.finish()));

    // 8. Read back pixels
    let buffer_slice = output_buffer.slice(..);
    let (tx, rx) = flume::bounded(1);
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        tx.send(result).unwrap();
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv_async().await??;

    let data = buffer_slice.get_mapped_range();

    // Remove row padding
    let mut rgba_data = Vec::with_capacity((width * height * 4) as usize);
    for row in 0..height {
        let start = (row * bytes_per_row) as usize;
        let end = start + (width * 4) as usize;
        rgba_data.extend_from_slice(&data[start..end]);
    }
    drop(data);
    output_buffer.unmap();

    RgbaImage::from_raw(width, height, rgba_data)
        .ok_or_else(|| anyhow::anyhow!("Failed to create image"))
}
