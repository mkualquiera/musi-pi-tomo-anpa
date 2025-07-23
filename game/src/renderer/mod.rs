pub mod gizmo;
pub mod text;

use glam::Mat4;
use glyphon::{Color as GlyphonColor, Resolution};
use image::GenericImageView;
use std::{cell::RefCell, mem, rc::Rc, sync::Arc};
use wgpu::{
    wgc::device, Buffer, Color, CommandBuffer, Device, Queue, Surface, SurfaceConfiguration,
    TexelCopyBufferLayout, Texture, TextureDescriptor, TextureView,
};
use winit::window::Window;

use crate::{
    game::Game,
    geometry::Transform,
    renderer::{
        gizmo::{
            GizmoBindableTexture, GizmoRenderPipeline, GizmoSprite, GizmoSpriteSheet, SpriteSpec,
        },
        text::{FeaturedTextBuffer, TextRenderPipeline},
    },
};

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
    pub const YELLOW: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };

    pub fn additive_darken(&self, factor: f32) -> Self {
        Self {
            r: (self.r - factor).max(0.0),
            g: (self.g - factor).max(0.0),
            b: (self.b - factor).max(0.0),
            a: self.a,
        }
    }
}

pub struct RenderingSystem {
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    target_aspect_ratio: f32,

    ortographic_transform: Transform,

    gizmo_pipeline: GizmoRenderPipeline,

    alignment_hint: u32,

    white_gizmo_texture: GizmoBindableTexture,

    pub text_pipeline: Rc<RefCell<TextRenderPipeline>>,
    original_size: (u32, u32),
}

pub struct Drawer<'a> {
    //pass: RenderPass<'a>,
    pub renderer: &'a RenderingSystem,
    view: &'a TextureView,
    command_buffers: Vec<CommandBuffer>,
    pub ortho: &'a Transform,
}

impl RenderingSystem {
    pub async fn new(window: Arc<Window>, width: u32, height: u32, alignment_hint: u32) -> Self {
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

        let gizmo_pipeline = GizmoRenderPipeline::new(&device, &config);

        let ortographic_transform = Transform::from_matrix(Mat4::orthographic_rh(
            0.0,
            width as f32,
            height as f32,
            0.0,
            -100.0,
            100.0,
        ));

        let white_gizmo_texture = gizmo_pipeline.make_texture_bindable(
            &device,
            Self::create_texture(&device, &queue, 1, 1, Some(&[255, 255, 255, 255])),
        );

        let text_pipeline = TextRenderPipeline::new(&device, &queue, surface_format);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            ortographic_transform,
            target_aspect_ratio,
            gizmo_pipeline,
            alignment_hint,
            white_gizmo_texture,
            text_pipeline: Rc::new(RefCell::new(text_pipeline)),
            original_size: (width, height),
        }
    }
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            // First, calculate what size we'd want to maintain aspect ratio
            let new_aspect_ratio = new_size.width as f32 / new_size.height as f32;
            let (mut width, mut height) = if new_aspect_ratio > self.target_aspect_ratio {
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

            // Now scale down if either dimension exceeds 2047
            if width > 2047 || height > 2047 {
                let scale_factor = (2047.0 / width as f32).min(2047.0 / height as f32);
                width = (width as f32 * scale_factor) as u32;
                height = (height as f32 * scale_factor) as u32;
            }

            // Apply alignment
            let (width, height) = (
                width / self.alignment_hint * self.alignment_hint,
                height / self.alignment_hint * self.alignment_hint,
            );

            self.size = winit::dpi::PhysicalSize::new(width, height);
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn canonical_resize(&mut self) {
        self.resize(self.size);
    }

    pub fn render(&mut self, game: &Game) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut drawer = Drawer::new(self, &view);

        game.render(&mut drawer);

        drawer.flush();

        output.present();

        self.text_pipeline.borrow_mut().atlas.trim();

        Ok(())
    }

    pub fn create_texture(
        device: &Device,
        queue: &Queue,
        width: u32,
        height: u32,
        data: Option<&[u8]>,
    ) -> Texture {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        if let Some(data) = data {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                data,
                TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * width),
                    rows_per_image: None,
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        }
        texture
    }

    pub fn create_gizmo_texture(
        device: &Device,
        queue: &Queue,
        gizmo_pipeline: &GizmoRenderPipeline,
        width: u32,
        height: u32,
        data: &[u8],
    ) -> GizmoBindableTexture {
        let texture = Self::create_texture(device, queue, width, height, Some(data));
        gizmo_pipeline.make_texture_bindable(device, texture)
    }

    pub fn gizmo_texture_from_encoded_image(&mut self, image_data: &[u8]) -> GizmoBindableTexture {
        let image = image::load_from_memory(image_data).unwrap();
        let (width, height) = image.dimensions();
        let rgba = image.to_rgba8();
        Self::create_gizmo_texture(
            &self.device,
            &self.queue,
            &self.gizmo_pipeline,
            width,
            height,
            rgba.as_raw().as_slice(),
        )
    }

    pub fn gizmo_sprite_sheet_from_encoded_image(
        &mut self,
        image_data: &[u8],
        region_start: [f32; 2],
        region_end: [f32; 2],
        num_tiles: [u32; 2],
    ) -> GizmoSpriteSheet {
        let texture = self.gizmo_texture_from_encoded_image(image_data);
        GizmoSpriteSheet::new(Rc::new(texture), region_start, region_end, num_tiles)
    }

    pub fn create_text_buffer(
        &mut self,
        font_size: f32,
        line_size: f32,
        width: f32,
        height: f32,
        text: &str,
        attrs: glyphon::Attrs<'static>,
        align: glyphon::cosmic_text::Align,
    ) -> FeaturedTextBuffer {
        self.text_pipeline
            .borrow_mut()
            .create_buffer(font_size, line_size, width, height, text, attrs, align)
    }

    pub fn load_font(&mut self, bytes: &[u8]) {
        self.text_pipeline.borrow_mut().load_font(bytes);
    }
}

impl<'a> Drawer<'a> {
    pub fn new(renderer: &'a RenderingSystem, view: &'a TextureView) -> Self {
        Self {
            renderer,
            view,
            command_buffers: Vec::new(),
            ortho: &renderer.ortographic_transform,
        }
    }

    fn apply_gizmo_transform(&mut self, transform: &Transform) {
        // we need to flush or else it will be out of order
        self.flush();
        self.renderer
            .gizmo_pipeline
            .write_transform(&self.renderer.queue, transform);
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

    pub fn apply_gizmo_color(&mut self, color: EngineColor) {
        self.flush();
        self.renderer
            .gizmo_pipeline
            .write_color(&self.renderer.queue, color);
    }

    pub fn draw_geometry_slow(
        &mut self,
        vertex_buffer: &Buffer,
        index_buffer: &Buffer,
        num_indices: u32,
        transform: Option<&Transform>,
        color: Option<&EngineColor>,
        texture: GizmoSprite,
    ) {
        if let Some(t) = transform {
            self.apply_gizmo_transform(t);
        } else {
            self.apply_gizmo_transform(self.ortho);
        }
        if let Some(c) = color {
            self.apply_gizmo_color(*c);
        } else {
            self.apply_gizmo_color(EngineColor {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            });
        }

        let GizmoSprite {
            texture,
            sprite_spec,
        } = texture;

        self.renderer
            .gizmo_pipeline
            .write_sprite_spec(&self.renderer.queue, sprite_spec);

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

            self.renderer.gizmo_pipeline.setup_pass(&mut render_pass);
            self.renderer
                .gizmo_pipeline
                .bind_texture(&mut render_pass, texture);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..num_indices, 0, 0..1);
        }
        //self.renderer
        //    .queue
        //    .submit(std::iter::once(encoder.finish()));
        self.command_buffers.push(encoder.finish());
    }

    pub fn draw_square_slow(
        &mut self,
        transform: Option<&Transform>,
        color: Option<&EngineColor>,
        texture: GizmoSprite,
    ) {
        //self.draw_geometry_slow(vertices, indices, count, transform, color);
        self.renderer.gizmo_pipeline.with_quad_geometry(
            |vertex_buffer, index_buffer, num_indices| {
                self.draw_geometry_slow(
                    vertex_buffer,
                    index_buffer,
                    num_indices,
                    transform,
                    color,
                    texture,
                );
            },
        );
    }

    pub fn white_sprite(&self) -> GizmoSprite<'a> {
        GizmoSprite {
            texture: &self.renderer.white_gizmo_texture,
            sprite_spec: SpriteSpec {
                use_texture: 1,
                region_start: [0.0, 0.0],
                region_end: [1.0, 1.0],
                num_tiles: [1, 1],
                selected_tile: [0, 0],
            },
        }
    }

    pub fn draw_text_slow(
        &mut self,
        text_buffer: &FeaturedTextBuffer,
        x: f32,
        y: f32,
        scale: f32,
        color: GlyphonColor,
    ) {
        self.renderer
            .text_pipeline
            .borrow_mut()
            .prepare_for_text_draw(
                &self.renderer.device,
                &self.renderer.queue,
                text_buffer,
                Resolution {
                    width: self.renderer.original_size.0,
                    height: self.renderer.original_size.1,
                },
                color,
                x,
                y,
                scale,
            )
            .expect("Failed to prepare text draw");

        let mut encoder =
            self.renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Text Encoder"),
                });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Text Pass"),
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

            self.renderer
                .text_pipeline
                .borrow_mut()
                .render(&mut render_pass)
                .expect("Failed to render text");
        }

        self.command_buffers.push(encoder.finish());
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
