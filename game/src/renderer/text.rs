use std::rc::Rc;

use glyphon::{
    Attrs, Buffer, Cache, Color, FontSystem, Metrics, Resolution, SwashCache, TextArea, TextAtlas,
    TextBounds, TextRenderer, Viewport,
};
use wgpu::{Device, MultisampleState, TextureFormat};

pub struct TextRenderPipeline {
    font_system: FontSystem,
    swash_cache: SwashCache,
    viewport: Viewport,
    pub atlas: TextAtlas,
    text_renderer: TextRenderer,
    cache: Cache,
}

pub struct FeaturedTextBuffer {
    buffer: Buffer,
    text: String,
    attrs: Attrs<'static>,
    width: f32,
    height: f32,
}

impl FeaturedTextBuffer {
    pub fn set_text(&mut self, pipeline: &mut TextRenderPipeline, text: &str) {
        self.text = text.to_string();
        self.buffer.set_text(
            &mut pipeline.font_system,
            text,
            &self.attrs,
            glyphon::Shaping::Advanced,
        );
        self.buffer
            .shape_until_scroll(&mut pipeline.font_system, false);
    }
}

const SCALING_FACTOR: f32 = 8.0;

impl TextRenderPipeline {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        swapchain_format: TextureFormat,
    ) -> Self {
        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(device);
        let viewport = Viewport::new(device, &cache);
        let mut atlas = TextAtlas::new(device, queue, &cache, swapchain_format);
        let text_renderer = TextRenderer::new(
            &mut atlas,
            device,
            MultisampleState {
                count: 1,
                mask: 0,
                alpha_to_coverage_enabled: false,
            },
            None,
        );

        Self {
            font_system,
            swash_cache,
            viewport,
            atlas,
            text_renderer,
            cache,
        }
    }

    pub fn load_font(&mut self, bytes: &[u8]) {
        self.font_system.db_mut().load_font_data(bytes.to_vec());
    }

    pub fn create_buffer(
        &mut self,
        font_size: f32,
        line_size: f32,
        width: f32,
        height: f32,
        text: &str,
        attrs: Attrs<'static>,
    ) -> FeaturedTextBuffer {
        let width = width * SCALING_FACTOR;
        let height = height * SCALING_FACTOR;
        let font_size = font_size * SCALING_FACTOR;
        let line_size = line_size * SCALING_FACTOR;
        let mut buffer = Buffer::new(&mut self.font_system, Metrics::new(font_size, line_size));
        buffer.set_size(&mut self.font_system, Some(width), Some(height));
        buffer.set_text(
            &mut self.font_system,
            text,
            &attrs,
            glyphon::Shaping::Advanced,
        );
        buffer.shape_until_scroll(&mut self.font_system, false);
        FeaturedTextBuffer {
            buffer,
            text: text.to_string(),
            attrs,
            width,
            height,
        }
    }

    pub fn prepare_for_text_draw(
        &mut self,
        device: &Device,
        queue: &wgpu::Queue,
        text_buffer: &FeaturedTextBuffer,
        resolution: Resolution,
        color: Color,
        x: f32,
        y: f32,
        scale: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let resolution = Resolution {
            width: (resolution.width as f32 * SCALING_FACTOR) as u32,
            height: (resolution.height as f32 * SCALING_FACTOR) as u32,
        };
        self.viewport.update(queue, resolution);

        self.text_renderer.prepare(
            device,
            queue,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            [TextArea {
                buffer: &text_buffer.buffer,
                left: x * SCALING_FACTOR,
                top: y * SCALING_FACTOR,
                bounds: TextBounds {
                    left: 0,
                    top: 0,
                    right: (resolution.width as f32 * SCALING_FACTOR) as i32,
                    bottom: (resolution.height as f32 * SCALING_FACTOR) as i32,
                },
                scale,
                default_color: color,
                custom_glyphs: &[],
            }],
            &mut self.swash_cache,
        )?;

        Ok(())
    }

    pub fn render(
        &mut self,
        pass: &mut wgpu::RenderPass,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.text_renderer
            .render(&self.atlas, &self.viewport, pass)?;
        Ok(())
    }
}
