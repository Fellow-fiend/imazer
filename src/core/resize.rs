use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use image::imageops::FilterType;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SizeMode {
    Pixels,
    Percent,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PersistedSettings {
    pub width: u32,
    pub height: u32,
    pub keep_aspect_ratio: bool,
    pub width_mode: SizeMode,
    pub height_mode: SizeMode,
    pub output_dir: Option<PathBuf>,
}

impl Default for PersistedSettings {
    fn default() -> Self {
        Self {
            width: 1024,
            height: 1024,
            keep_aspect_ratio: true,
            width_mode: SizeMode::Pixels,
            height_mode: SizeMode::Pixels,
            output_dir: None,
        }
    }
}

pub fn is_supported_image(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "svg" | "tif" | "tiff"
    )
}

pub fn process_one(
    path: &Path,
    output_dir: &Path,
    settings: &PersistedSettings,
) -> Result<PathBuf> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    let dyn_img = if ext == "svg" {
        load_svg(path)?
    } else {
        image::open(path).with_context(|| format!("open image {}", path.display()))?
    };

    let (orig_w, orig_h) = (dyn_img.width(), dyn_img.height());
    let (target_w, target_h) = compute_target_size(orig_w, orig_h, settings);
    let resized = dyn_img.resize_exact(target_w, target_h, FilterType::Lanczos3);

    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("image");
    let output_name = if ext == "svg" {
        format!("{stem}_resized.png")
    } else {
        format!("{stem}_resized.{ext}")
    };

    std::fs::create_dir_all(output_dir)
        .with_context(|| format!("create output dir {}", output_dir.display()))?;
    let out_path = output_dir.join(output_name);
    resized
        .save(&out_path)
        .with_context(|| format!("save output {}", out_path.display()))?;

    Ok(out_path)
}

pub fn compute_target_size(orig_w: u32, orig_h: u32, settings: &PersistedSettings) -> (u32, u32) {
    let mut w = match settings.width_mode {
        SizeMode::Pixels => settings.width.max(1),
        SizeMode::Percent => ((orig_w as f32) * (settings.width as f32 / 100.0)).round() as u32,
    }
    .max(1);

    let mut h = match settings.height_mode {
        SizeMode::Pixels => settings.height.max(1),
        SizeMode::Percent => ((orig_h as f32) * (settings.height as f32 / 100.0)).round() as u32,
    }
    .max(1);

    if settings.keep_aspect_ratio {
        let ratio = orig_w as f32 / orig_h.max(1) as f32;
        if settings.width_mode != settings.height_mode {
            if settings.width_mode == SizeMode::Pixels {
                h = ((w as f32) / ratio).round() as u32;
            } else {
                w = ((h as f32) * ratio).round() as u32;
            }
        } else {
            h = ((w as f32) / ratio).round() as u32;
        }
    }

    (w.max(1), h.max(1))
}

pub fn image_dimensions(path: &Path) -> Result<(u32, u32)> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    if ext == "svg" {
        let data = std::fs::read(path).with_context(|| format!("read svg {}", path.display()))?;
        let opts = resvg::usvg::Options::default();
        let tree = resvg::usvg::Tree::from_data(&data, &opts)
            .with_context(|| format!("parse svg {}", path.display()))?;
        let size = tree.size().to_int_size();
        return Ok((size.width(), size.height()));
    }

    image::image_dimensions(path)
        .with_context(|| format!("read image dimensions {}", path.display()))
}

pub fn load_preview_rgba(path: &Path) -> Result<(Vec<u8>, [usize; 2])> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    let img = if ext == "svg" {
        load_svg(path)?
    } else {
        image::open(path).with_context(|| format!("open image {}", path.display()))?
    };
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Ok((rgba.into_vec(), size))
}

fn load_svg(path: &Path) -> Result<image::DynamicImage> {
    let data = std::fs::read(path).with_context(|| format!("read svg {}", path.display()))?;

    let opts = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(&data, &opts)
        .with_context(|| format!("parse svg {}", path.display()))?;

    let size = tree.size().to_int_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())
        .context("allocate svg pixmap")?;

    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );

    let rgba = image::RgbaImage::from_raw(size.width(), size.height(), pixmap.take())
        .context("convert svg pixmap")?;

    Ok(image::DynamicImage::ImageRgba8(rgba))
}
