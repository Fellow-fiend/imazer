#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod core;
mod platform;
mod ui;

use std::path::PathBuf;

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
        Box::new(|_cc| Ok(Box::new(app::AppState::new(files)))),
    )
}
