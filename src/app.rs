use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc, Arc,
};
use std::thread;

use anyhow::{Context, Result};
use eframe::egui;
use rayon::prelude::*;

use crate::core::aspect_ratio::{height_from_width, width_from_height};
use crate::core::resize::{
    image_dimensions, is_supported_image, load_preview_rgba, process_one, PersistedSettings,
    SizeMode,
};
use crate::ui;

#[derive(Clone, Debug)]
struct ImageItem {
    path: PathBuf,
    original_width: u32,
    original_height: u32,
}

#[derive(Default)]
struct JobProgress {
    finished: AtomicUsize,
    total: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LastEdited {
    Width,
    Height,
}

pub struct AppState {
    images: Vec<ImageItem>,
    selected_idx: Option<usize>,
    settings: PersistedSettings,
    worker: Option<thread::JoinHandle<Vec<String>>>,
    progress: Option<Arc<JobProgress>>,
    logs: Vec<String>,
    last_edited: LastEdited,
    preview_texture: Option<egui::TextureHandle>,
    preview_receiver: Option<mpsc::Receiver<Result<(Vec<u8>, [usize; 2])>>>,
    preview_loading_for: Option<PathBuf>,
}

impl AppState {
    pub fn new(initial_files: Vec<PathBuf>) -> Self {
        let mut app = Self {
            images: Vec::new(),
            selected_idx: None,
            settings: load_settings().unwrap_or_default(),
            worker: None,
            progress: None,
            logs: Vec::new(),
            last_edited: LastEdited::Width,
            preview_texture: None,
            preview_receiver: None,
            preview_loading_for: None,
        };

        app.add_paths(initial_files);
        app
    }

    fn add_paths(&mut self, paths: Vec<PathBuf>) {
        for path in expand_paths(paths) {
            if !is_supported_image(&path) || self.images.iter().any(|i| i.path == path) {
                continue;
            }

            match image_dimensions(&path) {
                Ok((w, h)) => self.images.push(ImageItem {
                    path,
                    original_width: w,
                    original_height: h,
                }),
                Err(err) => self
                    .logs
                    .push(format!("Skipped {}: {err:#}", path.display())),
            }
        }

        if self.selected_idx.is_none() && !self.images.is_empty() {
            self.selected_idx = Some(0);
        }
    }

    fn remove_selected(&mut self) {
        if let Some(idx) = self.selected_idx {
            self.images.remove(idx);
            self.preview_texture = None;
            self.preview_loading_for = None;
            self.preview_receiver = None;
            self.selected_idx = if self.images.is_empty() {
                None
            } else if idx >= self.images.len() {
                Some(self.images.len() - 1)
            } else {
                Some(idx)
            };
        }
    }

    fn reference_dimensions(&self) -> Option<(u32, u32)> {
        self.selected_idx
            .and_then(|idx| self.images.get(idx))
            .map(|item| (item.original_width, item.original_height))
            .or_else(|| {
                self.images
                    .first()
                    .map(|item| (item.original_width, item.original_height))
            })
    }

    fn apply_aspect_ratio_after_width_change(&mut self) {
        if !self.settings.keep_aspect_ratio || self.settings.width_mode != SizeMode::Pixels {
            return;
        }
        if let Some((ow, oh)) = self.reference_dimensions() {
            self.settings.height = height_from_width(self.settings.width.max(1), ow, oh);
        }
    }

    fn apply_aspect_ratio_after_height_change(&mut self) {
        if !self.settings.keep_aspect_ratio || self.settings.height_mode != SizeMode::Pixels {
            return;
        }
        if let Some((ow, oh)) = self.reference_dimensions() {
            self.settings.width = width_from_height(self.settings.height.max(1), ow, oh);
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
            self.add_paths(files);
        }
    }

    fn choose_output_dir(&mut self) {
        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
            self.settings.output_dir = Some(dir);
            self.save_current_settings();
        }
    }

    fn launch_resize(&mut self) {
        if self.worker.is_some() || self.images.is_empty() {
            return;
        }

        let files = self
            .images
            .iter()
            .map(|i| i.path.clone())
            .collect::<Vec<_>>();
        let settings = self.settings.clone();

        let source_dirs = files
            .iter()
            .filter_map(|p| p.parent().map(Path::to_path_buf))
            .collect::<HashSet<_>>();
        let multi_input_dirs = source_dirs.len() > 1;

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
                    let output_dir =
                        resolve_output_dir(path, settings.output_dir.clone(), multi_input_dirs);
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

    fn poll_preview(&mut self, ctx: &egui::Context) {
        if let Some(receiver) = &self.preview_receiver {
            match receiver.try_recv() {
                Ok(Ok((bytes, size))) => {
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &bytes);
                    self.preview_texture = Some(ctx.load_texture(
                        "preview",
                        color_image,
                        egui::TextureOptions::LINEAR,
                    ));
                    self.preview_receiver = None;
                }
                Ok(Err(err)) => {
                    self.logs.push(format!("Preview error: {err:#}"));
                    self.preview_receiver = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.preview_receiver = None;
                }
            }
        }
    }

    fn request_preview_if_needed(&mut self) {
        let Some(idx) = self.selected_idx else {
            return;
        };
        let Some(item) = self.images.get(idx) else {
            return;
        };

        if self.preview_loading_for.as_ref() == Some(&item.path) {
            return;
        }

        let path = item.path.clone();
        self.preview_texture = None;
        self.preview_loading_for = Some(path.clone());

        let (tx, rx) = mpsc::channel();
        self.preview_receiver = Some(rx);
        thread::spawn(move || {
            let _ = tx.send(load_preview_rgba(&path));
        });
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
            self.add_paths(files);
        }

        self.poll_preview(ctx);

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

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("Imazer");
                ui.separator();
                ui.label("Drop images here or use Select Files.");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui::sized_button(ui, "Resize Images").clicked() {
                        self.launch_resize();
                    }
                });
            });
        });

        egui::SidePanel::left("settings")
            .resizable(true)
            .default_width(340.0)
            .show(ctx, |ui| {
                ui.heading("Batch Settings");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label("Width");
                    let mut width = self.settings.width as i32;
                    if ui
                        .add(egui::DragValue::new(&mut width).range(1..=100_000))
                        .changed()
                    {
                        self.settings.width = width.max(1) as u32;
                        self.last_edited = LastEdited::Width;
                        self.apply_aspect_ratio_after_width_change();
                        self.save_current_settings();
                    }
                });
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_source("width-mode")
                        .selected_text(match self.settings.width_mode {
                            SizeMode::Pixels => "Pixels",
                            SizeMode::Percent => "Percent",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.settings.width_mode,
                                SizeMode::Pixels,
                                "Pixels",
                            );
                            ui.selectable_value(
                                &mut self.settings.width_mode,
                                SizeMode::Percent,
                                "Percent",
                            );
                        });
                });

                ui.horizontal(|ui| {
                    ui.label("Height");
                    let mut height = self.settings.height as i32;
                    if ui
                        .add(egui::DragValue::new(&mut height).range(1..=100_000))
                        .changed()
                    {
                        self.settings.height = height.max(1) as u32;
                        self.last_edited = LastEdited::Height;
                        self.apply_aspect_ratio_after_height_change();
                        self.save_current_settings();
                    }
                });
                egui::ComboBox::from_id_source("height-mode")
                    .selected_text(match self.settings.height_mode {
                        SizeMode::Pixels => "Pixels",
                        SizeMode::Percent => "Percent",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.settings.height_mode,
                            SizeMode::Pixels,
                            "Pixels",
                        );
                        ui.selectable_value(
                            &mut self.settings.height_mode,
                            SizeMode::Percent,
                            "Percent",
                        );
                    });

                if ui
                    .checkbox(&mut self.settings.keep_aspect_ratio, "Lock aspect ratio")
                    .changed()
                {
                    if self.settings.keep_aspect_ratio {
                        if self.last_edited == LastEdited::Width {
                            self.apply_aspect_ratio_after_width_change();
                        } else {
                            self.apply_aspect_ratio_after_height_change();
                        }
                    }
                    self.save_current_settings();
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                ui.label("Output folder");
                let text = self
                    .settings
                    .output_dir
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| {
                        "Auto: source folder (or source/resized for mixed folders)".to_string()
                    });
                ui.label(egui::RichText::new(text).small());
                ui.horizontal(|ui| {
                    if ui::sized_button(ui, "Choose Folder").clicked() {
                        self.choose_output_dir();
                    }
                    if ui::sized_button(ui, "Use Auto").clicked() {
                        self.settings.output_dir = None;
                        self.save_current_settings();
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
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui::sized_button(ui, "Select Files").clicked() {
                    self.select_files();
                }
                if ui::sized_button(ui, "Remove Selected").clicked() {
                    self.remove_selected();
                }
                if ui::sized_button(ui, "Clear All").clicked() {
                    self.images.clear();
                    self.selected_idx = None;
                    self.preview_texture = None;
                }
            });

            ui.add_space(8.0);
            ui.label(format!("Loaded images: {}", self.images.len()));

            ui.columns(2, |cols| {
                cols[0].group(|ui| {
                    ui.set_min_height(380.0);
                    ui.label(egui::RichText::new("Image Queue").strong());
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let mut clicked_idx = None;
                        for (idx, item) in self.images.iter().enumerate() {
                            let name = item
                                .path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| item.path.display().to_string());
                            let selected = self.selected_idx == Some(idx);
                            let response = ui.selectable_label(
                                selected,
                                format!(
                                    "{} ({}x{})",
                                    name, item.original_width, item.original_height
                                ),
                            );
                            if response.clicked() {
                                clicked_idx = Some(idx);
                            }
                        }
                        if let Some(idx) = clicked_idx {
                            self.selected_idx = Some(idx);
                            self.request_preview_if_needed();
                        }
                    });
                });

                cols[1].group(|ui| {
                    ui.set_min_height(380.0);
                    ui.label(egui::RichText::new("Preview").strong());
                    self.request_preview_if_needed();
                    if let Some(tex) = &self.preview_texture {
                        let available = ui.available_size();
                        let original = tex.size_vec2();
                        let scale = (available.x / original.x)
                            .min(available.y / original.y)
                            .min(1.0);
                        ui.image((tex.id(), original * scale.max(0.1)));
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.label("Select an image to preview");
                        });
                    }
                });
            });

            ui.add_space(8.0);
            ui.separator();
            ui.label(egui::RichText::new("Log").strong());
            egui::ScrollArea::vertical()
                .max_height(120.0)
                .show(ui, |ui| {
                    for line in &self.logs {
                        ui.label(line);
                    }
                });
        });
    }
}

fn resolve_output_dir(
    path: &Path,
    override_dir: Option<PathBuf>,
    multi_input_dirs: bool,
) -> PathBuf {
    if let Some(dir) = override_dir {
        return dir;
    }

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if multi_input_dirs {
        parent.join("resized")
    } else {
        parent.to_path_buf()
    }
}

fn expand_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for path in paths {
        if path.is_file() {
            out.push(path);
        } else if path.is_dir() {
            collect_images_recursively(&path, &mut out);
        }
    }
    out
}

fn collect_images_recursively(root: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_images_recursively(&path, out);
            } else if path.is_file() && is_supported_image(&path) {
                out.push(path);
            }
        }
    }
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
