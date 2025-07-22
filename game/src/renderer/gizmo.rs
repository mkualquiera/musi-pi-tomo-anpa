use std::{mem, rc::Rc};

use wgpu::{
    BindGroup, BindGroupLayout, BindGroupLayoutEntry, Buffer, Device, Queue, RenderPipeline,
    SurfaceConfiguration, Texture,
};

use crate::{geometry::Transform, renderer::EngineColor};

pub struct GizmoBindableTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: BindGroup,
}

#[derive(Clone, Copy)]
pub struct GizmoSprite<'a> {
    pub texture: &'a GizmoBindableTexture,
    pub sprite_spec: SpriteSpec,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub uv: [f32; 2],
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
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

//#[repr(C)]
//#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[derive(Clone, Copy)]
pub struct SpriteSpec {
    pub use_texture: u32,
    pub region_start: [f32; 2],
    pub region_end: [f32; 2],
    pub num_tiles: [u32; 2],
    pub selected_tile: [u32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteSpecPadded {
    pub use_texture_and_padding: [u32; 4], // use_texture in [0], rest unused
    pub region_start_and_end: [f32; 4],    // start in [0,1], end in [2,3]
    pub tiles_info: [u32; 4],              // num_tiles in [0,1], selected in [2,3]
}

impl From<SpriteSpec> for SpriteSpecPadded {
    fn from(spec: SpriteSpec) -> Self {
        Self {
            use_texture_and_padding: [spec.use_texture, 0, 0, 0],
            region_start_and_end: [
                spec.region_start[0],
                spec.region_start[1],
                spec.region_end[0],
                spec.region_end[1],
            ],
            tiles_info: [
                spec.num_tiles[0],
                spec.num_tiles[1],
                spec.selected_tile[0],
                spec.selected_tile[1],
            ],
        }
    }
}

#[derive(Clone)]
pub struct GizmoSpriteSheet {
    texture: Rc<GizmoBindableTexture>,
    region_start: [f32; 2],
    region_end: [f32; 2],
    num_tiles: [u32; 2],
}

impl GizmoSpriteSheet {
    pub fn new(
        texture: Rc<GizmoBindableTexture>,
        region_start: [f32; 2],
        region_end: [f32; 2],
        num_tiles: [u32; 2],
    ) -> Self {
        Self {
            texture,
            region_start,
            region_end,
            num_tiles,
        }
    }

    pub fn get_sprite(&self, selected_tile: [u32; 2]) -> Option<GizmoSprite> {
        if selected_tile[0] >= self.num_tiles[0] || selected_tile[1] >= self.num_tiles[1] {
            return None; // Invalid tile selection
        }
        Some(GizmoSprite {
            texture: &self.texture,
            sprite_spec: SpriteSpec {
                use_texture: 1,
                region_start: self.region_start,
                region_end: self.region_end,
                num_tiles: self.num_tiles,
                selected_tile,
            },
        })
    }
}

pub struct GizmoRenderPipeline {
    pipeline: RenderPipeline,
    transform_buffer: Buffer,
    transform_bind_group: BindGroup,
    color_buffer: Buffer,
    color_bind_group: BindGroup,
    // For pre-baked geometry:
    square_vertex_buffer: Buffer,
    square_index_buffer: Buffer,
    texture_bind_group_layout: BindGroupLayout,
    sprite_spec_bind_group: BindGroup,
    sprite_spec_buffer: Buffer,
}

impl GizmoRenderPipeline {
    pub fn new(device: &Device, config: &SurfaceConfiguration) -> Self {
        let shader_source = include_str!("../assets/shader.wgsl");
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

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Bind Group Layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let sprite_spec_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sprite Spec Buffer"),
            //size: mem::size_of::<SpriteSpecPadded>() as u64, // Ensure alignment
            size: mem::size_of::<SpriteSpecPadded>() as u64, // Ensure alignment
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sprite_spec_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Sprite Spec Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 4,
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
                bind_group_layouts: &[
                    &transform_bind_group_layout,
                    &color_bind_group_layout,
                    &texture_bind_group_layout,
                    &sprite_spec_bind_group_layout,
                ],
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
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

        let sprite_spec_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Sprite Spec Bind Group"),
            layout: &sprite_spec_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &sprite_spec_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        let square_vertices = [
            Vertex {
                position: [0.0, 0.0, 0.0],
                color: [1.0, 1.0, 1.0],
                uv: [0.0, 0.0],
            }, // Top Left
            Vertex {
                position: [0.0, 1.0, 0.0],
                color: [1.0, 1.0, 1.0],
                uv: [0.0, 1.0],
            }, // Bottom Left
            Vertex {
                position: [1.0, 1.0, 0.0],
                color: [1.0, 1.0, 1.0],
                uv: [1.0, 1.0],
            }, // Bottom Right
            Vertex {
                position: [1.0, 0.0, 0.0],
                color: [1.0, 1.0, 1.0],
                uv: [1.0, 0.0],
            }, // Top Right
        ];

        let square_indices: &[u16] = &[0, 1, 2, 3, 0, 2];

        let square_vertex_buffer = Self::create_vertex_buffer_internal(device, &square_vertices);
        let square_index_buffer = Self::create_index_buffer_internal(device, square_indices);

        Self {
            pipeline: render_pipeline,
            transform_buffer,
            transform_bind_group,
            color_buffer,
            color_bind_group,
            square_vertex_buffer,
            square_index_buffer,
            texture_bind_group_layout,
            sprite_spec_bind_group,
            sprite_spec_buffer,
        }
    }

    pub fn create_vertex_buffer_internal(device: &Device, vertices: &[Vertex]) -> wgpu::Buffer {
        let align = wgpu::COPY_BUFFER_ALIGNMENT;
        let vertex_size = std::mem::size_of_val(vertices) as u64;
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

    pub fn create_index_buffer_internal(device: &Device, indices: &[u16]) -> wgpu::Buffer {
        let align = wgpu::COPY_BUFFER_ALIGNMENT;
        let index_size = std::mem::size_of_val(indices) as u64;
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

    pub fn write_transform(&self, queue: &Queue, transform: &Transform) {
        transform.write_buffer(&self.transform_buffer, queue);
    }

    pub fn write_color(&self, queue: &Queue, color: EngineColor) {
        queue.write_buffer(&self.color_buffer, 0, bytemuck::cast_slice(&[color]));
    }

    pub fn write_sprite_spec(&self, queue: &Queue, sprite_spec: SpriteSpec) {
        queue.write_buffer(
            &self.sprite_spec_buffer,
            0,
            //bytemuck::cast_slice(&[sprite_spec]),
            // we need to pad it
            bytemuck::cast_slice(&[SpriteSpecPadded::from(sprite_spec)]),
        );
    }

    pub fn bind_texture(&self, render_pass: &mut wgpu::RenderPass, texture: &GizmoBindableTexture) {
        render_pass.set_bind_group(2, &texture.bind_group, &[]);
    }

    pub fn setup_pass(&self, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.transform_bind_group, &[]);
        render_pass.set_bind_group(1, &self.color_bind_group, &[]);
        render_pass.set_bind_group(3, &self.sprite_spec_bind_group, &[]);
    }

    pub fn with_quad_geometry<F: FnOnce(&Buffer, &Buffer, u32)>(&self, f: F) {
        f(&self.square_vertex_buffer, &self.square_index_buffer, 6);
    }

    pub fn make_texture_bindable(&self, device: &Device, texture: Texture) -> GizmoBindableTexture {
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Gizmo Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Gizmo Texture Bind Group"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        GizmoBindableTexture {
            texture,
            view,
            sampler,
            bind_group,
        }
    }
}
