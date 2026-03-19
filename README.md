# Imazer

Imazer is a fast cross-platform desktop image resizer built in Rust.

## Features

- Modern `egui` desktop UI with toolbar, settings panel, file queue, and image preview.
- Drag & drop files or folders (folders are scanned recursively).
- Batch resize with parallel execution (`rayon`).
- Resize by pixel or percentage.
- Correct aspect-ratio lock based on original dimensions.
- Smart output behavior:
  - Single source directory: outputs to source folder by default.
  - Multiple source directories: outputs to `resized/` per source directory by default.
  - Optional UI override for custom output folder.
- Settings persistence (`serde` + JSON config file).
- Windows packaging assets (context menu `.reg`, Inno Setup installer, signing script).

## Project layout

```text
imazer/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── app.rs
│   ├── ui/
│   │   └── mod.rs
│   ├── core/
│   │   ├── aspect_ratio.rs
│   │   ├── mod.rs
│   │   └── resize.rs
│   └── platform/
│       ├── mod.rs
│       └── windows.rs
├── installer/
│   ├── resize_context_menu.reg
│   └── setup.iss
└── scripts/
    ├── build_release.bat
    ├── sign.bat
    └── windows/
```

## Build

```bash
cargo build --release
```

Output binary:

- Windows: `target/release/imazer.exe`
- Linux/macOS: `target/release/imazer`

## Installer (Windows)

1. Build release binary.
2. Build installer with Inno Setup:

```bash
iscc installer/setup.iss
```

Generated installer is placed in `installer/dist/`.

## Context menu registration (Windows)

Use the installer (preferred), or import:

- `installer/resize_context_menu.reg`

## Code signing (Windows)

```bat
set SIGN_PWD=your-pfx-password
scripts\sign.bat target\release\imazer.exe path\to\certificate.pfx
```

## CLI usage

The app accepts file/folder paths as arguments:

```bash
imazer file1.png folder_with_images
```
