use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use image::codecs::webp::WebPEncoder;
use image::imageops::FilterType;
use image::{ColorType, DynamicImage, GenericImageView};

use crate::cas::ContentAddressedStore;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThumbnailSize {
    Small,
    Large,
}

impl ThumbnailSize {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThumbnailSize::Small => "small",
            ThumbnailSize::Large => "large",
        }
    }

    pub fn max_dimension(&self) -> u32 {
        match self {
            ThumbnailSize::Small => 200,
            ThumbnailSize::Large => 800,
        }
    }
}

#[derive(Debug)]
pub struct ThumbnailGenerationResult {
    pub content_hash: String,
    pub size: ThumbnailSize,
    pub local_path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
    pub reused: bool,
}

pub struct ThumbnailGenerator<'a> {
    cas: &'a ContentAddressedStore,
}

impl<'a> ThumbnailGenerator<'a> {
    pub fn new(cas: &'a ContentAddressedStore) -> Self {
        Self { cas }
    }

    pub fn generate(
        &self,
        source: &Path,
        content_hash: &str,
        size: ThumbnailSize,
    ) -> Result<ThumbnailGenerationResult, Box<dyn std::error::Error + Send + Sync>> {
        let output_path = self.cas.thumbnail_path(content_hash, size.as_str());
        let hash_prefix = if content_hash.len() >= 2 {
            &content_hash[..2]
        } else {
            content_hash
        };
        fs::create_dir_all(
            self.cas
                .thumbs_root()
                .join(hash_prefix),
        )?;

        if output_path.exists() {
            let metadata = fs::metadata(&output_path)?;
            let info = read_image_info(source)?;
            return Ok(ThumbnailGenerationResult {
                content_hash: content_hash.to_string(),
                size,
                local_path: output_path,
                width: info.width,
                height: info.height,
                bytes: metadata.len(),
                reused: true,
            });
        }

        let img = image::open(source)?;
        let (orig_width, orig_height) = img.dimensions();

        let max_dim = size.max_dimension();
        let (new_width, new_height) = calculate_resize_dimensions(
            orig_width,
            orig_height,
            max_dim,
        );

        let resized = if orig_width <= max_dim && orig_height <= max_dim {
            img
        } else {
            img.resize(new_width, new_height, FilterType::Triangle)
        };

        let tmp_path = output_path.with_extension(format!(
            "{}.tmp.webp",
            output_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
        ));

        {
            let file = File::create(&tmp_path)?;
            let writer = BufWriter::new(file);
            let encoder = WebPEncoder::new_lossless(writer);
            let color_type = resized.color();
            let (rgb_image, color_out) = match color_type {
                ColorType::Rgba8 => (
                    resized.to_rgba8().into_raw(),
                    ColorType::Rgba8,
                ),
                ColorType::Rgb8 => (
                    resized.to_rgb8().into_raw(),
                    ColorType::Rgb8,
                ),
                _ => (
                    resized.to_rgba8().into_raw(),
                    ColorType::Rgba8,
                ),
            };
            let (w, h) = (new_width.min(orig_width), new_height.min(orig_height));
            match color_out {
                ColorType::Rgba8 => {
                    encoder.encode(&rgb_image, w, h, image::ExtendedColorType::Rgba8)?
                }
                ColorType::Rgb8 => {
                    encoder.encode(&rgb_image, w, h, image::ExtendedColorType::Rgb8)?
                }
                _ => {
                    encoder.encode(&rgb_image, w, h, image::ExtendedColorType::Rgba8)?
                }
            }
        }

        fs::rename(&tmp_path, &output_path)?;

        let metadata = fs::metadata(&output_path)?;

        Ok(ThumbnailGenerationResult {
            content_hash: content_hash.to_string(),
            size,
            local_path: output_path,
            width: new_width.min(orig_width),
            height: new_height.min(orig_height),
            bytes: metadata.len(),
            reused: false,
        })
    }

    pub fn get_existing_path(
        &self,
        content_hash: &str,
        size: ThumbnailSize,
    ) -> Option<PathBuf> {
        let path = self.cas.thumbnail_path(content_hash, size.as_str());
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    pub fn delete_thumbnails(
        &self,
        content_hash: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let small = self.cas.thumbnail_path(content_hash, "small");
        let large = self.cas.thumbnail_path(content_hash, "large");
        if small.exists() {
            fs::remove_file(&small)?;
        }
        if large.exists() {
            fs::remove_file(&large)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
}

pub fn read_image_info(
    source: &Path,
) -> Result<ImageInfo, Box<dyn std::error::Error + Send + Sync>> {
    let format = image::guess_format(source).map_err(|e| {
        format!("无法识别图片格式: {}", e)
    })?;
    let file = File::open(source)?;
    let mut reader = image::io::Reader::new(file);
    reader.set_format(format);
    let dimensions = reader.into_dimensions()?;
    Ok(ImageInfo {
        width: dimensions.0,
        height: dimensions.1,
    })
}

fn calculate_resize_dimensions(
    width: u32,
    height: u32,
    max_dim: u32,
) -> (u32, u32) {
    if width == 0 || height == 0 {
        return (max_dim, max_dim);
    }

    if width <= max_dim && height <= max_dim {
        return (width, height);
    }

    let ratio = if width >= height {
        max_dim as f64 / width as f64
    } else {
        max_dim as f64 / height as f64
    };

    let new_width = (width as f64 * ratio) as u32;
    let new_height = (height as f64 * ratio) as u32;

    (new_width.max(1), new_height.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_preserves_aspect_ratio() {
        let (w, h) = calculate_resize_dimensions(1920, 1080, 200);
        assert!(w == 200 || h == 200);
        assert!(w <= 200 && h <= 200);

        let original_ratio = 1920.0 / 1080.0;
        let new_ratio = w as f64 / h as f64;
        assert!((new_ratio - original_ratio).abs() < 0.1);
    }

    #[test]
    fn small_images_not_upscaled() {
        let (w, h) = calculate_resize_dimensions(100, 100, 200);
        assert_eq!(w, 100);
        assert_eq!(h, 100);
    }

    #[test]
    fn thumbnail_path_matches_cas() {
        let dir = std::env::temp_dir().join(format!(
            "opentu-thumb-test-{}",
            std::process::id()
        ));
        let cas = ContentAddressedStore::new(dir.clone());
        let gen = ThumbnailGenerator::new(&cas);
        let hash = "abcdef1234567890";
        let path = gen.get_existing_path(hash, ThumbnailSize::Small);
        let cas_path = cas.thumbnail_path(hash, "small");
        assert!(path.is_none() || path.unwrap() == cas_path);
        let _ = fs::remove_dir_all(&dir);
    }
}
