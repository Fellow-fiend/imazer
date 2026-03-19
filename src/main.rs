use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::thread;

use anyhow::{Context, Result};
use eframe::egui;
use image::imageops::FilterType;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

fn main() -> eframe::Result<()> {
    let mut files = std::env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    files.retain(|p| p.exists());

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Imazer - Image Resizer",
        native_options,
        Box::new(|_cc| Ok(Box::new(AppState::new(files)))),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum SizeMode {
    Pixels,
    Percent,
}

#[derive(Clone, Debug)]
struct ImageItem {
    path: PathBuf,
}

#[derive(Default)]
struct JobProgress {
    finished: AtomicUsize,
    total: usize,
}

#[derive(Clone, Serialize, Deserialize)]
struct PersistedSettings {
    width: u32,
    height: u32,
    keep_aspect_ratio: bool,
    width_mode: SizeMode,
    height_mode: SizeMode,
    output_dir: Option<PathBuf>,
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

struct AppState {
    images: Vec<ImageItem>,
    settings: PersistedSettings,
    worker: Option<thread::JoinHandle<Vec<String>>>,
    progress: Option<Arc<JobProgress>>,
    logs: Vec<String>,
}

impl AppState {
    fn new(initial_files: Vec<PathBuf>) -> Self {
        let mut app = Self {
            images: Vec::new(),
            settings: load_settings().unwrap_or_default(),
            worker: None,
            progress: None,
            logs: Vec::new(),
        };

        app.add_files(initial_files);
        app
    }

    fn add_files(&mut self, files: Vec<PathBuf>) {
        for path in files {
            if is_supported_image(&path) && !self.images.iter().any(|i| i.path == path) {
                self.images.push(ImageItem { path });
            }
        }
    }

    fn select_files(&mut self) {
        if let Some(files) = rfd::FileDialog::new()
            .add_filter(
                "images",
                &["png", "jpg", "jpeg", "webp", "gif", "svg", "tif", "tiff"],
            )
            .pick_files()
        {
            self.add_files(files);
        }
    }

    fn choose_output_dir(&mut self) {
        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
            self.settings.output_dir = Some(dir);
            let _ = save_settings(&self.settings);
        }
    }

    fn launch_resize(&mut self) {
        if self.worker.is_some() || self.images.is_empty() {
            return;
        }

        let output_dir = match self.settings.output_dir.clone() {
            Some(v) => v,
            None => {
                self.logs.push("Set an output directory first.".to_string());
                return;
            }
        };

        let files = self
            .images
            .iter()
            .map(|i| i.path.clone())
            .collect::<Vec<_>>();
        let settings = self.settings.clone();

        let progress = Arc::new(JobProgress {
            finished: AtomicUsize::new(0),
            total: files.len(),
        });
        let progress_for_thread = progress.clone();

        self.logs.clear();
        self.worker = Some(thread::spawn(move || {
            files
                .par_iter()
                .map(|path| {
                    let result = process_one(path, &output_dir, &settings)
                        .map(|saved| format!("OK: {}", saved.display()))
                        .unwrap_or_else(|err| format!("ERR: {} => {err:#}", path.display()));

                    progress_for_thread.finished.fetch_add(1, Ordering::Relaxed);
                    result
                })
                .collect::<Vec<_>>()
        }));
        self.progress = Some(progress);
    }

    fn save_current_settings(&mut self) {
        if let Err(e) = save_settings(&self.settings) {
            self.logs.push(format!("Failed to save settings: {e:#}"));
        }
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if !dropped.is_empty() {
            let files = dropped
                .into_iter()
                .filter_map(|f| f.path)
                .collect::<Vec<_>>();
            self.add_files(files);
        }

        if let Some(handle) = &self.worker {
            if handle.is_finished() {
                if let Some(done) = self.worker.take() {
                    match done.join() {
                        Ok(messages) => self.logs = messages,
                        Err(_) => self.logs.push("Worker thread panicked".to_string()),
                    }
                }
                self.progress = None;
            }
        }

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.heading("Imazer");
            ui.label("Drop images here or click Select files.");
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("Select files").clicked() {
                    self.select_files();
                }
                if ui.button("Clear list").clicked() {
                    self.images.clear();
                }
            });

            ui.separator();

            ui.label(format!("Loaded images: {}", self.images.len()));
            egui::ScrollArea::vertical()
                .max_height(160.0)
                .show(ui, |ui| {
                    for item in &self.images {
                        ui.label(item.path.display().to_string());
                    }
                });

            ui.separator();
            ui.heading("Resize options");

            ui.horizontal(|ui| {
                ui.label("Width:");
                let mut width = self.settings.width as i32;
                if ui.add(egui::DragValue::new(&mut width).speed(1)).changed() {
                    self.settings.width = width.max(1) as u32;
                    self.save_current_settings();
                }
                let old = self.settings.width_mode;
                egui::ComboBox::from_id_source("width-mode")
                    .selected_text(match self.settings.width_mode {
                        SizeMode::Pixels => "px",
                        SizeMode::Percent => "%",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.settings.width_mode, SizeMode::Pixels, "px");
                        ui.selectable_value(&mut self.settings.width_mode, SizeMode::Percent, "%");
                    });
                if self.settings.width_mode != old {
                    self.save_current_settings();
                }
            });

            ui.horizontal(|ui| {
                ui.label("Height:");
                let mut height = self.settings.height as i32;
                if ui.add(egui::DragValue::new(&mut height).speed(1)).changed() {
                    self.settings.height = height.max(1) as u32;
                    self.save_current_settings();
                }
                let old = self.settings.height_mode;
                egui::ComboBox::from_id_source("height-mode")
                    .selected_text(match self.settings.height_mode {
                        SizeMode::Pixels => "px",
                        SizeMode::Percent => "%",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.settings.height_mode, SizeMode::Pixels, "px");
                        ui.selectable_value(&mut self.settings.height_mode, SizeMode::Percent, "%");
                    });
                if self.settings.height_mode != old {
                    self.save_current_settings();
                }
            });

            if ui
                .checkbox(&mut self.settings.keep_aspect_ratio, "Keep aspect ratio")
                .changed()
            {
                self.save_current_settings();
            }

            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Output folder:");
                let text = self
                    .settings
                    .output_dir
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "(not selected)".to_string());
                ui.label(text);
                if ui.button("Select...").clicked() {
                    self.choose_output_dir();
                }
            });

            if let Some(p) = &self.progress {
                let done = p.finished.load(Ordering::Relaxed);
                let frac = if p.total > 0 {
                    done as f32 / p.total as f32
                } else {
                    0.0
                };
                ui.add(egui::ProgressBar::new(frac).text(format!("{done}/{}", p.total)));
            }

            if ui.button("Resize").clicked() {
                self.launch_resize();
            }

            ui.separator();
            ui.heading("Log");
            egui::ScrollArea::vertical()
                .max_height(180.0)
                .show(ui, |ui| {
                    for line in &self.logs {
                        ui.label(line);
                    }
                });
        });
    }
}

fn is_supported_image(path: &Path) -> bool {
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

fn process_one(path: &Path, output_dir: &Path, settings: &PersistedSettings) -> Result<PathBuf> {
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

    let out_path = output_dir.join(output_name);
    resized
        .save(&out_path)
        .with_context(|| format!("save output {}", out_path.display()))?;

    Ok(out_path)
}

fn compute_target_size(orig_w: u32, orig_h: u32, settings: &PersistedSettings) -> (u32, u32) {
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
        let ratio = orig_w as f32 / orig_h as f32;
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

fn settings_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir().context("config dir unavailable")?;
    Ok(config_dir.join("imazer").join("settings.json"))
}

fn load_settings() -> Result<PersistedSettings> {
    let path = settings_path()?;
    let data = std::fs::read(&path).with_context(|| format!("read settings {}", path.display()))?;
    let settings = serde_json::from_slice(&data).context("parse settings")?;
    Ok(settings)
}

fn save_settings(settings: &PersistedSettings) -> Result<()> {
    let path = settings_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("create settings dir")?;
    }
    let data = serde_json::to_vec_pretty(settings).context("serialize settings")?;
    std::fs::write(&path, data).with_context(|| format!("write settings {}", path.display()))?;
    Ok(())
}
