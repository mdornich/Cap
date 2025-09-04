use gif::{Encoder, Frame, Repeat};
use std::fs::File;
use std::path::Path;
use thiserror::Error;

#[derive(Clone, Copy, Debug)]
pub enum GifQuality {
    /// Fast encoding, smaller file size, lower quality
    Fast,
    /// Balanced quality and file size (default)
    Balanced,
    /// Best quality, larger file size, slower encoding
    High,
}

#[derive(Error, Debug)]
pub enum GifEncodingError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("GIF encoding error: {0}")]
    Gif(#[from] gif::EncodingError),
    #[error("Invalid frame data")]
    InvalidFrameData,
}

pub struct GifEncoderWrapper {
    encoder: Encoder<File>,
    width: u16,
    height: u16,
    frame_delay: u16,
    quality: GifQuality,
}

impl GifEncoderWrapper {
    pub fn new<P: AsRef<Path>>(
        path: P,
        width: u32,
        height: u32,
        fps: u32,
        quality: GifQuality,
    ) -> Result<Self, GifEncodingError> {
        let file = File::create(path)?;

        let global_palette = create_quality_palette(quality);
        let mut encoder = Encoder::new(file, width as u16, height as u16, &global_palette)?;
        encoder.set_repeat(Repeat::Infinite)?;

        let frame_delay = (100.0 / fps as f32) as u16;

        Ok(Self {
            encoder,
            width: width as u16,
            height: height as u16,
            frame_delay,
            quality,
        })
    }

    pub fn add_frame(
        &mut self,
        frame_data: &[u8],
        padded_bytes_per_row: usize,
    ) -> Result<(), GifEncodingError> {
        let width = self.width as usize;
        let height = self.height as usize;
        
        // Create palette for this quality level
        let palette = create_quality_palette(self.quality);
        
        // Convert to indexed data with improved dithering
        let indexed_data = self.convert_to_indexed(frame_data, padded_bytes_per_row, &palette)?;

        let mut frame = Frame::from_indexed_pixels(self.width, self.height, indexed_data, None);
        frame.delay = self.frame_delay;

        self.encoder.write_frame(&frame)?;
        Ok(())
    }

    fn convert_to_indexed(
        &self,
        frame_data: &[u8],
        padded_bytes_per_row: usize,
        palette: &[u8],
    ) -> Result<Vec<u8>, GifEncodingError> {
        let width = self.width as usize;
        let height = self.height as usize;
        let mut indexed_data = Vec::with_capacity(width * height);

        // Apply different dithering strategies based on quality
        match self.quality {
            GifQuality::Fast => {
                // Simple nearest color matching for speed
                for y in 0..height {
                    let row_start = y * padded_bytes_per_row;
                    for x in 0..width {
                        let pixel_start = row_start + x * 4;
                        if pixel_start + 2 < frame_data.len() {
                            let r = frame_data[pixel_start];
                            let g = frame_data[pixel_start + 1];
                            let b = frame_data[pixel_start + 2];
                            
                            let palette_idx = find_closest_palette_index_fast(r, g, b, palette);
                            indexed_data.push(palette_idx);
                        } else {
                            return Err(GifEncodingError::InvalidFrameData);
                        }
                    }
                }
            }
            GifQuality::Balanced | GifQuality::High => {
                // Floyd-Steinberg dithering for better quality
                let mut rgb_data = Vec::with_capacity(width * height * 3);
                for y in 0..height {
                    let row_start = y * padded_bytes_per_row;
                    for x in 0..width {
                        let pixel_start = row_start + x * 4;
                        if pixel_start + 2 < frame_data.len() {
                            rgb_data.push(frame_data[pixel_start] as i32);
                            rgb_data.push(frame_data[pixel_start + 1] as i32);
                            rgb_data.push(frame_data[pixel_start + 2] as i32);
                        } else {
                            return Err(GifEncodingError::InvalidFrameData);
                        }
                    }
                }

                for y in 0..height {
                    for x in 0..width {
                        let idx = (y * width + x) * 3;
                        let r = rgb_data[idx].clamp(0, 255) as u8;
                        let g = rgb_data[idx + 1].clamp(0, 255) as u8;
                        let b = rgb_data[idx + 2].clamp(0, 255) as u8;

                        let palette_idx = find_closest_palette_index_precise(r, g, b, palette);
                        indexed_data.push(palette_idx);

                        let (pr, pg, pb) = get_palette_color_from_data(palette_idx, palette);

                        let er = r as i32 - pr as i32;
                        let eg = g as i32 - pg as i32;
                        let eb = b as i32 - pb as i32;

                        // Floyd-Steinberg error diffusion
                        if x + 1 < width {
                            let idx_right = (y * width + x + 1) * 3;
                            rgb_data[idx_right] += (er * 7) / 16;
                            rgb_data[idx_right + 1] += (eg * 7) / 16;
                            rgb_data[idx_right + 2] += (eb * 7) / 16;
                        }
                        if y + 1 < height {
                            if x > 0 {
                                let idx_bottom_left = ((y + 1) * width + x - 1) * 3;
                                rgb_data[idx_bottom_left] += (er * 3) / 16;
                                rgb_data[idx_bottom_left + 1] += (eg * 3) / 16;
                                rgb_data[idx_bottom_left + 2] += (eb * 3) / 16;
                            }
                            let idx_bottom = ((y + 1) * width + x) * 3;
                            rgb_data[idx_bottom] += (er * 5) / 16;
                            rgb_data[idx_bottom + 1] += (eg * 5) / 16;
                            rgb_data[idx_bottom + 2] += (eb * 5) / 16;

                            if x + 1 < width {
                                let idx_bottom_right = ((y + 1) * width + x + 1) * 3;
                                rgb_data[idx_bottom_right] += er / 16;
                                rgb_data[idx_bottom_right + 1] += eg / 16;
                                rgb_data[idx_bottom_right + 2] += eb / 16;
                            }
                        }
                    }
                }
            }
        }

        Ok(indexed_data)
    }

    pub fn finish(self) -> Result<(), GifEncodingError> {
        drop(self.encoder);
        Ok(())
    }
}

fn create_quality_palette(quality: GifQuality) -> Vec<u8> {
    match quality {
        GifQuality::Fast => create_fast_palette(),
        GifQuality::Balanced => create_balanced_palette(),
        GifQuality::High => create_high_quality_palette(),
    }
}

/// Fast palette with fewer colors but faster encoding
fn create_fast_palette() -> Vec<u8> {
    let mut palette = Vec::with_capacity(256 * 3);
    
    // Use a 6x6x6 RGB cube (216 colors) + grayscale
    for r in 0..6 {
        for g in 0..6 {
            for b in 0..6 {
                palette.push((r * 51) as u8); // 51 = 255/5 rounded
                palette.push((g * 51) as u8);
                palette.push((b * 51) as u8);
            }
        }
    }
    
    // Add grayscale values (40 colors)
    for i in 0..40 {
        let gray = (i * 255 / 39) as u8;
        palette.push(gray);
        palette.push(gray);
        palette.push(gray);
    }
    
    // Fill remaining slots with transparent/black
    while palette.len() < 256 * 3 {
        palette.push(0);
        palette.push(0);
        palette.push(0);
    }
    
    palette
}

/// Balanced palette optimized for typical screen content
fn create_balanced_palette() -> Vec<u8> {
    let mut palette = Vec::with_capacity(256 * 3);

    // Enhanced RGB cube with better distribution for screen content
    for r in 0..6 {
        for g in 0..7 {
            for b in 0..6 {
                palette.push((r * 255 / 5) as u8);
                palette.push((g * 255 / 6) as u8);
                palette.push((b * 255 / 5) as u8);
            }
        }
    }

    // Add dedicated grayscale ramp for better text/UI rendering
    for i in 0..4 {
        let gray = (i * 85) as u8; // 0, 85, 170, 255
        palette.push(gray);
        palette.push(gray);
        palette.push(gray);
    }

    assert_eq!(palette.len(), 256 * 3, "Palette must be exactly 256 colors");
    palette
}

/// High-quality palette with optimized color distribution
fn create_high_quality_palette() -> Vec<u8> {
    let mut palette = Vec::with_capacity(256 * 3);
    
    // Use an 8x8x4 cube for better coverage of common colors
    for r in 0..8 {
        for g in 0..8 {
            for b in 0..4 {
                palette.push((r * 255 / 7) as u8);
                palette.push((g * 255 / 7) as u8);
                palette.push((b * 255 / 3) as u8);
            }
        }
    }
    
    // Add more blue tones (common in UIs)
    for b in 4..8 {
        for intensity in [64, 128, 192, 255] {
            palette.push(0);
            palette.push(0);
            palette.push((b * intensity / 8) as u8);
        }
    }
    
    // Add more green tones
    for g in 4..8 {
        for intensity in [64, 128, 192, 255] {
            palette.push(0);
            palette.push((g * intensity / 8) as u8);
            palette.push(0);
        }
    }
    
    // Fill remaining with fine grayscale gradient
    let remaining_slots = 256 - (palette.len() / 3);
    for i in 0..remaining_slots {
        let gray = (i * 255 / (remaining_slots - 1)) as u8;
        palette.push(gray);
        palette.push(gray);
        palette.push(gray);
    }
    
    // Ensure exactly 256 colors
    palette.truncate(256 * 3);
    palette
}

/// Fast palette index finding for speed-optimized encoding
fn find_closest_palette_index_fast(r: u8, g: u8, b: u8, palette: &[u8]) -> u8 {
    let mut best_index = 0;
    let mut best_distance = u32::MAX;
    
    for i in 0..256 {
        let pr = palette[i * 3] as i32;
        let pg = palette[i * 3 + 1] as i32;
        let pb = palette[i * 3 + 2] as i32;
        
        let dr = r as i32 - pr;
        let dg = g as i32 - pg;
        let db = b as i32 - pb;
        
        // Use weighted Euclidean distance (human eye sensitivity)
        let distance = ((dr * dr * 3 + dg * dg * 4 + db * db * 2) / 3) as u32;
        
        if distance < best_distance {
            best_distance = distance;
            best_index = i as u8;
            
            // Early exit for exact matches
            if distance == 0 {
                break;
            }
        }
    }
    
    best_index
}

/// Precise palette index finding for quality-optimized encoding
fn find_closest_palette_index_precise(r: u8, g: u8, b: u8, palette: &[u8]) -> u8 {
    let mut best_index = 0;
    let mut best_distance = f64::MAX;
    
    for i in 0..256 {
        let pr = palette[i * 3] as f64;
        let pg = palette[i * 3 + 1] as f64;
        let pb = palette[i * 3 + 2] as f64;
        
        let dr = r as f64 - pr;
        let dg = g as f64 - pg;
        let db = b as f64 - pb;
        
        // Use perceptual color difference (Delta E approximation)
        let distance = (dr * dr * 0.3 + dg * dg * 0.59 + db * db * 0.11).sqrt();
        
        if distance < best_distance {
            best_distance = distance;
            best_index = i as u8;
            
            // Early exit for very close matches
            if distance < 0.1 {
                break;
            }
        }
    }
    
    best_index
}

/// Get RGB color from palette data at given index
fn get_palette_color_from_data(index: u8, palette: &[u8]) -> (u8, u8, u8) {
    let i = index as usize * 3;
    if i + 2 < palette.len() {
        (palette[i], palette[i + 1], palette[i + 2])
    } else {
        (0, 0, 0)
    }
}
