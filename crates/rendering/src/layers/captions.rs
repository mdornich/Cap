use bytemuck::{Pod, Zeroable};
use cap_project::XY;
use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping, Style, SwashCache,
    TextArea, TextAtlas, TextBounds, TextRenderer, Viewport, Weight,
};
use log::{debug, info, warn};
use wgpu::{util::DeviceExt, Device, Queue};

use crate::{parse_color_component, DecodedSegmentFrames, ProjectUniforms, RenderVideoConstants};

/// Represents a caption segment with timing and text
#[derive(Debug, Clone)]
pub struct CaptionSegment {
    pub id: String,
    pub start: f32,
    pub end: f32,
    pub text: String,
}

/// Settings for caption rendering
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct CaptionSettings {
    pub enabled: u32, // 0 = disabled, 1 = enabled
    pub font_size: f32,
    pub color: [f32; 4],
    pub background_color: [f32; 4],
    pub position: u32, // 0 = top, 1 = middle, 2 = bottom
    pub outline: u32,  // 0 = disabled, 1 = enabled
    pub outline_color: [f32; 4],
    pub font: u32,     // 0 = SansSerif, 1 = Serif, 2 = Monospace
    pub bold: u32,     // 0 = disabled, 1 = enabled
    pub italic: u32,   // 0 = disabled, 1 = enabled
    pub _padding: [f32; 2], // for alignment (increased for new fields)
}

impl Default for CaptionSettings {
    fn default() -> Self {
        Self {
            enabled: 1,
            font_size: 24.0,
            color: [1.0, 1.0, 1.0, 1.0],            // white
            background_color: [0.0, 0.0, 0.0, 0.8], // 80% black
            position: 2,                            // bottom
            outline: 1,                             // enabled
            outline_color: [0.0, 0.0, 0.0, 1.0],    // black
            font: 0,                                // SansSerif
            bold: 1,                                // enabled
            italic: 0,                              // disabled
            _padding: [0.0, 0.0],
        }
    }
}

/// Vertex data for background quad
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct QuadVertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl QuadVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x2,  // position
        1 => Float32x4,  // color
    ];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<QuadVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Caption layer that renders text using GPU
pub struct CaptionsLayer {
    settings_buffer: wgpu::Buffer,
    font_system: FontSystem,
    swash_cache: SwashCache,
    text_atlas: TextAtlas,
    text_renderer: TextRenderer,
    text_buffer: Buffer,
    current_text: Option<String>,
    current_segment_time: f32,
    current_bold: u32,
    current_italic: u32,
    current_font: u32,
    viewport: Viewport,
    // Background rendering resources
    background_pipeline: wgpu::RenderPipeline,
    background_vertex_buffer: wgpu::Buffer,
    background_index_buffer: wgpu::Buffer,
    current_background_bounds: Option<TextBounds>,
    current_background_color: [f32; 4],
}

impl CaptionsLayer {
    pub fn new(device: &Device, queue: &Queue) -> Self {
        // Create default settings buffer
        let settings = CaptionSettings::default();
        let settings_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Caption Settings Buffer"),
            contents: bytemuck::cast_slice(&[settings]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Initialize glyphon text rendering components
        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(device);
        let viewport = Viewport::new(device, &cache);
        let mut text_atlas =
            TextAtlas::new(device, queue, &cache, wgpu::TextureFormat::Rgba8UnormSrgb);
        let text_renderer = TextRenderer::new(
            &mut text_atlas,
            device,
            wgpu::MultisampleState::default(),
            None,
        );

        // Create an empty buffer with default metrics
        let metrics = Metrics::new(24.0, 24.0 * 1.2); // Default font size and line height
        let text_buffer = Buffer::new_empty(metrics);

        // Create background rendering resources
        let shader_source = r#"
            struct VertexInput {
                @location(0) position: vec2<f32>,
                @location(1) color: vec4<f32>,
            };

            struct VertexOutput {
                @builtin(position) position: vec4<f32>,
                @location(0) color: vec4<f32>,
            };

            @vertex
            fn vs_main(input: VertexInput) -> VertexOutput {
                var output: VertexOutput;
                // Convert from pixel coordinates to NDC
                // Assuming viewport of 1920x1080 (will be adjusted in prepare)
                output.position = vec4<f32>(
                    input.position.x,
                    input.position.y,
                    0.0,
                    1.0
                );
                output.color = input.color;
                return output;
            }

            @fragment
            fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
                return input.color;
            }
        "#;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Caption Background Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Caption Background Pipeline Layout"),
            bind_group_layouts: &[],  // No bind groups needed - color comes from vertex data
            push_constant_ranges: &[],
        });

        let background_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Caption Background Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[QuadVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

        // Create vertex buffer for a quad (will update vertices in prepare)
        let vertices = [
            QuadVertex { position: [0.0, 0.0], color: [0.0, 0.0, 0.0, 0.8] },
            QuadVertex { position: [1.0, 0.0], color: [0.0, 0.0, 0.0, 0.8] },
            QuadVertex { position: [1.0, 1.0], color: [0.0, 0.0, 0.0, 0.8] },
            QuadVertex { position: [0.0, 1.0], color: [0.0, 0.0, 0.0, 0.8] },
        ];
        
        let background_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Caption Background Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let indices: [u16; 6] = [0, 1, 2, 2, 3, 0];
        let background_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Caption Background Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            settings_buffer,
            font_system,
            swash_cache,
            text_atlas,
            text_renderer,
            text_buffer,
            current_text: None,
            current_segment_time: 0.0,
            current_bold: 1, // default from CaptionSettings::default()
            current_italic: 0,
            current_font: 0,
            viewport,
            background_pipeline,
            background_vertex_buffer,
            background_index_buffer,
            current_background_bounds: None,
            current_background_color: [0.0, 0.0, 0.0, 0.0],
        }
    }

    /// Update the settings for caption rendering
    pub fn update_settings(&mut self, queue: &Queue, settings: CaptionSettings) {
        queue.write_buffer(&self.settings_buffer, 0, bytemuck::cast_slice(&[settings]));
    }

    /// Update the current caption text and timing
    pub fn update_caption(&mut self, text: Option<String>, time: f32) {
        debug!("Updating caption - Text: {:?}, Time: {}", text, time);
        // Only update the stored text, don't modify the buffer here
        // The buffer will be updated in prepare() with proper styling
        self.current_text = text;
        self.current_segment_time = time;
    }

    pub fn prepare(
        &mut self,
        uniforms: &ProjectUniforms,
        segment_frames: &DecodedSegmentFrames,
        output_size: XY<u32>,
        constants: &RenderVideoConstants,
    ) {
        // Render captions if there are any caption segments to display
        if let Some(caption_data) = &uniforms.project.captions {
            if caption_data.settings.enabled {
                // Find the current caption for this time
                let current_time = segment_frames.segment_time;

                if let Some(current_caption) =
                    find_caption_at_time_project(current_time, &caption_data.segments)
                {
                    // Get caption text and time for use in rendering
                    let caption_text = current_caption.text.clone();

                    // Create settings for the caption
                    let settings = CaptionSettings {
                        enabled: 1,
                        font_size: caption_data.settings.size as f32,
                        color: [
                            parse_color_component(&caption_data.settings.color, 0),
                            parse_color_component(&caption_data.settings.color, 1),
                            parse_color_component(&caption_data.settings.color, 2),
                            1.0,
                        ],
                        background_color: [
                            parse_color_component(&caption_data.settings.background_color, 0),
                            parse_color_component(&caption_data.settings.background_color, 1),
                            parse_color_component(&caption_data.settings.background_color, 2),
                            caption_data.settings.background_opacity as f32 / 100.0,
                        ],
                        position: match caption_data.settings.position.as_str() {
                            "top" => 0,
                            "middle" => 1,
                            _ => 2, // default to bottom
                        },
                        outline: if caption_data.settings.outline { 1 } else { 0 },
                        outline_color: [
                            parse_color_component(&caption_data.settings.outline_color, 0),
                            parse_color_component(&caption_data.settings.outline_color, 1),
                            parse_color_component(&caption_data.settings.outline_color, 2),
                            1.0,
                        ],
                        font: match caption_data.settings.font.as_str() {
                            "System Serif" => 1,
                            "System Monospace" => 2,
                            _ => 0, // Default to SansSerif for "System Sans-Serif" and any other value
                        },
                        bold: if caption_data.settings.bold { 1 } else { 0 },
                        italic: if caption_data.settings.italic { 1 } else { 0 },
                        _padding: [0.0, 0.0],
                    };

                    // Update the current caption text
                    let text_changed = self.current_text.as_ref() != Some(&caption_text);
                    self.update_caption(Some(caption_text.clone()), current_time);

                    if settings.enabled == 0 {
                        return;
                    }

                    if self.current_text.is_none() {
                        return;
                    }

                    // Only recreate buffer if text changed or styles changed
                    if let Some(text) = &self.current_text {
                        let (width, height) = (output_size.x, output_size.y);

                        // Access device and queue from the pipeline's constants
                        let device = &constants.device;
                        let queue = &constants.queue;

                        // Find caption position based on settings
                        let y_position = match settings.position {
                            0 => height as f32 * 0.1,  // top
                            1 => height as f32 * 0.5,  // middle
                            _ => height as f32 * 0.85, // bottom (default)
                        };

                        // Set up caption appearance
                        let color = Color::rgb(
                            (settings.color[0] * 255.0) as u8,
                            (settings.color[1] * 255.0) as u8,
                            (settings.color[2] * 255.0) as u8,
                        );

                        // Get outline color if needed
                        let outline_color = Color::rgb(
                            (settings.outline_color[0] * 255.0) as u8,
                            (settings.outline_color[1] * 255.0) as u8,
                            (settings.outline_color[2] * 255.0) as u8,
                        );

                        // Calculate text bounds
                        let font_size = settings.font_size * (height as f32 / 1080.0); // Scale font size based on resolution
                        let metrics = Metrics::new(font_size, font_size * 1.2); // 1.2 line height

                        // Check if styles have changed
                        let styles_changed = self.current_bold != settings.bold ||
                                           self.current_italic != settings.italic ||
                                           self.current_font != settings.font;

                        // Set width for text wrapping
                        let text_width = width as f32 * 0.9;

                        // Always recreate buffer to ensure clean state
                        // This prevents any corruption from style changes
                        info!("Creating fresh text buffer - font_size: {}, width: {}", font_size, text_width);
                        self.text_buffer = Buffer::new(&mut self.font_system, metrics);
                        self.text_buffer.set_size(&mut self.font_system, Some(text_width), None);
                        self.text_buffer.set_wrap(&mut self.font_system, glyphon::Wrap::Word);

                        // Position text in the center horizontally
                        // The bounds dictate the rendering area
                        let bounds = TextBounds {
                            left: ((width as f32 - text_width) / 2.0) as i32, // Center the text horizontally
                            top: y_position as i32,
                            right: ((width as f32 + text_width) / 2.0) as i32, // Center + width
                            bottom: (y_position + font_size * 4.0) as i32, // Increased height for better visibility
                        };

                        // Apply text styling directly when setting the text
                        // Create text attributes with or without outline
                        let font_family = match settings.font {
                            0 => Family::SansSerif,
                            1 => Family::Serif,
                            2 => Family::Monospace,
                            _ => Family::SansSerif, // Default to SansSerif for any other value
                        };
                        
                        // Build text attributes with style settings
                        let mut attrs = Attrs::new().family(font_family).color(color);
                        
                        // Apply bold style if enabled
                        if settings.bold == 1 {
                            attrs = attrs.weight(Weight::BOLD);
                        }
                        
                        // Apply italic style if enabled
                        if settings.italic == 1 {
                            attrs = attrs.style(Style::Italic);
                        }

                        // Apply text to buffer with the styled attributes
                        // Always set text since we're recreating the buffer
                        info!("Setting text with attributes - bold: {}, italic: {}, font: {}", settings.bold, settings.italic, settings.font);
                        self.text_buffer.set_text(
                            &mut self.font_system,
                            text,
                            &attrs,
                            Shaping::Advanced,
                        );
                        // Update current style state
                        self.current_bold = settings.bold;
                        self.current_italic = settings.italic;
                        self.current_font = settings.font;

                        // Update the viewport with explicit resolution
                        self.viewport.update(queue, Resolution { width, height });

                        // Store background info for rendering
                        if settings.background_color[3] > 0.01 {
                            self.current_background_bounds = Some(bounds);
                            self.current_background_color = settings.background_color;

                            // Calculate actual text bounds for background
                            // We need to measure the actual text to get proper background size
                            let line_count = text.lines().count() as f32;
                            let text_height = font_size * line_count * 1.5; // Add some padding
                            
                            // Add padding around text
                            let padding = font_size * 0.5;
                            let bg_left = bounds.left as f32 - padding;
                            let bg_right = bounds.right as f32 + padding;
                            let bg_top = y_position - padding * 0.5;
                            let bg_bottom = y_position + text_height + padding * 0.5;

                            // Update vertex buffer with proper NDC coordinates
                            let ndc_left = (bg_left / width as f32) * 2.0 - 1.0;
                            let ndc_right = (bg_right / width as f32) * 2.0 - 1.0;
                            let ndc_top = 1.0 - (bg_top / height as f32) * 2.0;
                            let ndc_bottom = 1.0 - (bg_bottom / height as f32) * 2.0;

                            let vertices = [
                                QuadVertex { 
                                    position: [ndc_left, ndc_top], 
                                    color: settings.background_color 
                                },
                                QuadVertex { 
                                    position: [ndc_right, ndc_top], 
                                    color: settings.background_color 
                                },
                                QuadVertex { 
                                    position: [ndc_right, ndc_bottom], 
                                    color: settings.background_color 
                                },
                                QuadVertex { 
                                    position: [ndc_left, ndc_bottom], 
                                    color: settings.background_color 
                                },
                            ];
                            
                            queue.write_buffer(&self.background_vertex_buffer, 0, bytemuck::cast_slice(&vertices));
                        } else {
                            self.current_background_bounds = None;
                        }

                        // Prepare text areas for rendering
                        let mut text_areas = Vec::new();

                        // Add outline if enabled (by rendering the text multiple times with slight offsets in different positions)
                        if settings.outline == 1 {
                            info!("Rendering with outline");
                            // Outline is created by drawing the text multiple times with small offsets in different directions
                            let outline_offsets = [
                                (-1.0, -1.0),
                                (0.0, -1.0),
                                (1.0, -1.0),
                                (-1.0, 0.0),
                                (1.0, 0.0),
                                (-1.0, 1.0),
                                (0.0, 1.0),
                                (1.0, 1.0),
                            ];

                            for (offset_x, offset_y) in outline_offsets.iter() {
                                text_areas.push(TextArea {
                                    buffer: &self.text_buffer,
                                    left: bounds.left as f32 + offset_x, // Match bounds with small offset for outline
                                    top: y_position + offset_y,
                                    scale: 1.0,
                                    bounds,
                                    default_color: outline_color,
                                    custom_glyphs: &[],
                                });
                            }
                        }

                        // Add main text (rendered last, on top of everything)
                        text_areas.push(TextArea {
                            buffer: &self.text_buffer,
                            left: bounds.left as f32, // Match the bounds left for positioning
                            top: y_position,
                            scale: 1.0,
                            bounds,
                            default_color: color,
                            custom_glyphs: &[],
                        });

                        // Prepare text rendering
                        let text_areas_count = text_areas.len();
                        info!("Preparing text renderer with {} text areas", text_areas_count);
                        match self.text_renderer.prepare(
                            device,
                            queue,
                            &mut self.font_system,
                            &mut self.text_atlas,
                            &self.viewport,
                            text_areas,
                            &mut self.swash_cache,
                        ) {
                            Ok(_) => {
                                info!("Text renderer prepared successfully");
                            }
                            Err(e) => {
                                warn!("Error preparing text: {:?}", e);
                                // Log more details about the error
                                warn!("Text areas count: {}", text_areas_count);
                                warn!("Buffer metrics: font_size={}", font_size);
                            }
                        }
                    }
                } else {
                }
            } else {
            }
        } else {
        }
    }

    /// Render the current caption to the frame
    pub fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        // First render the background if present
        if self.current_background_bounds.is_some() && self.current_background_color[3] > 0.01 {
            pass.set_pipeline(&self.background_pipeline);
            pass.set_vertex_buffer(0, self.background_vertex_buffer.slice(..));
            pass.set_index_buffer(self.background_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..6, 0, 0..1);
        }

        // Then render the text on top
        match self
            .text_renderer
            .render(&self.text_atlas, &self.viewport, pass)
        {
            Ok(_) => {}
            Err(e) => warn!("Error rendering text: {:?}", e),
        }
    }
}

/// Function to find the current caption segment based on playback time
pub fn find_caption_at_time(time: f32, segments: &[CaptionSegment]) -> Option<&CaptionSegment> {
    segments
        .iter()
        .find(|segment| time >= segment.start && time < segment.end)
}

// Adding a new version that accepts cap_project::CaptionSegment
/// Function to find the current caption segment from cap_project::CaptionSegment based on playback time
pub fn find_caption_at_time_project(
    time: f32,
    segments: &[cap_project::CaptionSegment],
) -> Option<CaptionSegment> {
    segments
        .iter()
        .find(|segment| time >= segment.start && time < segment.end)
        .map(|segment| CaptionSegment {
            id: segment.id.clone(),
            start: segment.start,
            end: segment.end,
            text: segment.text.clone(),
        })
}

/// Convert from cap_project::CaptionSegment to our internal CaptionSegment
pub fn convert_project_caption(segment: &cap_project::CaptionSegment) -> CaptionSegment {
    CaptionSegment {
        id: segment.id.clone(),
        start: segment.start,
        end: segment.end,
        text: segment.text.clone(),
    }
}