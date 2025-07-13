use glam::{Mat4, Vec3};
use log::info;
use std::{
    mem,
    sync::{Arc, Mutex},
};
use wgpu::{
    BindGroup, Buffer, Color, CommandBuffer, CommandEncoder, Device, Queue, RenderPass,
    RenderPipeline, Surface, SurfaceConfiguration, TextureView,
};
use winit::window::Window;

use crate::{game::Game, geometry::Transform};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
/// Represents a color in RGBA format.
pub struct EngineColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl EngineColor {
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const RED: Self = Self {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const GREEN: Self = Self {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    pub const BLUE: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    pub const PURPLE: Self = Self {
        r: 0.5,
        g: 0.0,
        b: 0.5,
        a: 1.0,
    };
}

pub struct RenderingSystem {
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: RenderPipeline,
    target_aspect_ratio: f32,
    original_size: winit::dpi::PhysicalSize<u32>,

    // For transforms:
    transform_buffer: Buffer,
    transform_bind_group: BindGroup,
    ortographic_transform: Transform,

    // For pre-baked geometry:
    square_vertex_buffer: Buffer,
    square_index_buffer: Buffer,

    // For uniform color
    color_buffer: Buffer,
    color_bind_group: BindGroup,
}

pub struct Drawer<'a> {
    //pass: RenderPass<'a>,
    pub renderer: &'a RenderingSystem,
    view: &'a TextureView,
    command_buffers: Vec<CommandBuffer>,
    pub ortho: &'a Transform,
}

impl RenderingSystem {
    pub async fn new(window: Arc<Window>, width: u32, height: u32) -> Self {
        let target_aspect_ratio = width as f32 / height as f32;
        let size = winit::dpi::PhysicalSize::new(width, height);
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::default(),
            })
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        let shader_source = include_str!("shader.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let transform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Transform Buffer"),
            size: 4 * 4 * mem::size_of::<f32>() as u64, // 4x4 matrix
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let transform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Transform Bind Group Layout"),
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
            });

        let color_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Color Buffer"),
            size: mem::size_of::<EngineColor>() as u64, // 4 bytes for RGBA
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let color_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Color Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&transform_bind_group_layout, &color_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let transform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Transform Bind Group"),
            layout: &transform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &transform_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        let color_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Color Bind Group"),
            layout: &color_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &color_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        let ortographic_transform = Transform::from_matrix(Mat4::orthographic_rh(
            0.0,
            width as f32,
            height as f32,
            0.0,
            -100.0,
            100.0,
        ));

        let ortographic_size_invariant_transform =
            Transform::from_matrix(Mat4::orthographic_rh(0.0, 1.0, 1.0, 0.0, -100.0, 100.0));

        let square_vertices = [
            Vertex {
                position: [0.0, 0.0, 0.0],
                color: [1.0, 1.0, 1.0],
            }, // Top Left
            Vertex {
                position: [0.0, 1.0, 0.0],
                color: [1.0, 1.0, 1.0],
            }, // Bottom Left
            Vertex {
                position: [1.0, 1.0, 0.0],
                color: [1.0, 1.0, 1.0],
            }, // Bottom Right
            Vertex {
                position: [1.0, 0.0, 0.0],
                color: [1.0, 1.0, 1.0],
            }, // Top Right
        ];

        let square_indices: &[u16] = &[0, 1, 2, 3, 0, 2];

        let square_vertex_buffer = Self::create_vertex_buffer_internal(&device, &square_vertices);
        let square_index_buffer = Self::create_index_buffer_internal(&device, square_indices);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            transform_buffer,
            transform_bind_group,
            ortographic_transform,
            square_vertex_buffer,
            square_index_buffer,
            color_buffer,
            color_bind_group,
            target_aspect_ratio,
            original_size: size,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            let new_aspect_ratio = new_size.width as f32 / new_size.height as f32;
            let (width, height) = if new_aspect_ratio > self.target_aspect_ratio {
                (
                    new_size.width,
                    (new_size.width as f32 / self.target_aspect_ratio) as u32,
                )
            } else {
                (
                    (new_size.height as f32 * self.target_aspect_ratio) as u32,
                    new_size.height,
                )
            };
            self.size = winit::dpi::PhysicalSize::new(width, height);
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn canonical_resize(&mut self) {
        self.resize(self.size);
    }

    pub fn create_vertex_buffer_internal(device: &Device, vertices: &[Vertex]) -> wgpu::Buffer {
        let align = wgpu::COPY_BUFFER_ALIGNMENT as u64;
        let vertex_size = (vertices.len() * std::mem::size_of::<Vertex>()) as u64;
        let aligned_vertex_size = (vertex_size + align - 1) & !(align - 1);

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: aligned_vertex_size,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: true,
        });

        {
            let mut buffer_view = vertex_buffer.slice(..).get_mapped_range_mut();
            let vertex_bytes = bytemuck::cast_slice(vertices);
            buffer_view[..vertex_bytes.len()].copy_from_slice(vertex_bytes);
        }
        vertex_buffer.unmap();

        vertex_buffer
    }

    pub fn create_vertex_buffer(&self, vertices: &[Vertex]) -> wgpu::Buffer {
        Self::create_vertex_buffer_internal(&self.device, vertices)
    }

    pub fn create_index_buffer_internal(device: &Device, indices: &[u16]) -> wgpu::Buffer {
        let align = wgpu::COPY_BUFFER_ALIGNMENT as u64;
        let index_size = (indices.len() * std::mem::size_of::<u16>()) as u64;
        let aligned_index_size = (index_size + align - 1) & !(align - 1);

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Index Buffer"),
            size: aligned_index_size,
            usage: wgpu::BufferUsages::INDEX,
            mapped_at_creation: true,
        });

        {
            let mut buffer_view = index_buffer.slice(..).get_mapped_range_mut();
            let index_bytes = bytemuck::cast_slice(indices);
            buffer_view[..index_bytes.len()].copy_from_slice(index_bytes);
        }
        index_buffer.unmap();

        index_buffer
    }

    pub fn create_index_buffer(&self, indices: &[u16]) -> wgpu::Buffer {
        Self::create_index_buffer_internal(&self.device, indices)
    }

    pub fn render(&mut self, game: &Game) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        //let mut encoder = self
        //    .device
        //    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
        //        label: Some("Render Encoder"),
        //    });

        //{
        //    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        //        label: Some("Render Pass"),
        //        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        //            view: &view,
        //            resolve_target: None,
        //            ops: wgpu::Operations {
        //                load: wgpu::LoadOp::Clear(wgpu::Color {
        //                    r: 0.1,
        //                    g: 0.2,
        //                    b: 0.3,
        //                    a: 1.0,
        //                }),
        //                store: wgpu::StoreOp::Store,
        //            },
        //        })],
        //        depth_stencil_attachment: None,
        //        occlusion_query_set: None,
        //        timestamp_writes: None,
        //    });
        //
        //    //render_pass.set_pipeline(&self.render_pipeline);
        //    //render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        //    //render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        //    //render_pass.draw_indexed(0..num_indices, 0, 0..1);
        //
        //    //{
        //    //    let mut drawer = Drawer { pass: render_pass };
        //    //
        //    //    drawer.pass.set_pipeline(&self.render_pipeline);
        //    //
        //    //    game.render(&mut drawer);
        //    //}
        //}

        let mut drawer = Drawer::new(self, &view);

        game.render(&mut drawer);

        drawer.flush();

        //self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

impl<'a> Drawer<'a> {
    //pub fn draw_geometry(
    //    &mut self,
    //    vertex_buffer: &wgpu::Buffer,
    //    index_buffer: &wgpu::Buffer,
    //    num_indices: u32,
    //) {
    //    self.pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    //    self.pass
    //        .set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
    //    self.pass.draw_indexed(0..num_indices, 0, 0..1);
    //}

    pub fn new(renderer: &'a RenderingSystem, view: &'a TextureView) -> Self {
        Self {
            renderer,
            view,
            command_buffers: Vec::new(),
            ortho: &renderer.ortographic_transform,
        }
    }

    fn apply_transform(&mut self, transform: &Transform) {
        // we need to flush or else it will be out of order
        self.flush();
        transform.write_buffer(&self.renderer.transform_buffer, &self.renderer.queue);
    }

    pub fn clear_slow(&mut self, color: Color) {
        let mut encoder =
            self.renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Gizmo Encoder"),
                });

        {
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Gizmo Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }
        //self.renderer
        //    .queue
        //    .submit(std::iter::once(encoder.finish()));
        self.command_buffers.push(encoder.finish());
    }

    pub fn set_color(&mut self, color: EngineColor) {
        self.flush();
        self.renderer.queue.write_buffer(
            &self.renderer.color_buffer,
            0,
            bytemuck::cast_slice(&[color]),
        );
    }

    pub fn draw_geometry_slow(
        &mut self,
        vertex_buffer: &Buffer,
        index_buffer: &Buffer,
        num_indices: u32,
        transform: Option<&Transform>,
        color: Option<&EngineColor>,
    ) {
        if let Some(t) = transform {
            self.apply_transform(t);
        } else {
            self.apply_transform(self.ortho);
        }
        if let Some(c) = color {
            self.set_color(*c);
        } else {
            self.set_color(EngineColor {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            });
        }
        let mut encoder =
            self.renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Gizmo Encoder"),
                });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Gizmo Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.renderer.render_pipeline);
            render_pass.set_bind_group(0, &self.renderer.transform_bind_group, &[]);
            render_pass.set_bind_group(1, &self.renderer.color_bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..num_indices, 0, 0..1);
        }
        //self.renderer
        //    .queue
        //    .submit(std::iter::once(encoder.finish()));
        self.command_buffers.push(encoder.finish());
    }

    pub fn draw_square_slow(&mut self, transform: Option<&Transform>, color: Option<&EngineColor>) {
        self.draw_geometry_slow(
            &self.renderer.square_vertex_buffer,
            &self.renderer.square_index_buffer,
            6, // 6 indices for the square
            transform,
            color,
        );
    }

    pub fn flush(&mut self) {
        if !self.command_buffers.is_empty() {
            self.renderer
                .queue
                .submit(mem::take(&mut self.command_buffers));
            self.command_buffers.clear();
        }
    }
}
