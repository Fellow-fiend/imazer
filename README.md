# Imazer

Imazer is a simple, minimal, and fast cross-platform desktop image resizer.

## Features

- Clean GUI (drag-and-drop + file picker)
- Batch resize with multi-threading (Rayon)
- Resize by pixels or percentage
- Optional aspect-ratio lock
- Output directory picker
- Supported input formats:
  - PNG
  - JPG/JPEG
  - WEBP
  - GIF
  - TIFF
  - SVG (rasterized first, saved as PNG)
- Preserves original names with `_resized` suffix
- Remembers last used settings
- Windows context menu integration script

## Tech stack

- Rust
- egui/eframe for GUI
- image crate for raster formats
- resvg/usvg for SVG rasterization

No Node.js / Python runtime needed.

## Project layout

```text
imazer/
├── Cargo.toml
├── Makefile
├── src/
│   └── main.rs
└── scripts/
    └── windows/
        ├── install_context_menu.reg
        └── uninstall_context_menu.reg
```

## Build requirements

Install Rust toolchain (`rustup` + `cargo`).

### Windows (MSVC or MinGW)

```bash
make release
```

Or directly:

```bash
cargo build --release
```

Binary path:

```text
target/release/imazer.exe
```

> For Windows 7 compatibility, build using a Rust toolchain that still targets Windows 7 in your environment policy and test on a real Win7 machine.

### Linux (gcc/clang toolchain available)

```bash
make release
```

Binary path:

```text
target/release/imazer
```

### macOS (clang)

```bash
make release
```

Binary path:

```text
target/release/imazer
```

## Run

```bash
make run
```

## CLI usage (also used by Windows context menu)

You can pass image paths directly:

```bash
imazer file1.png file2.jpg
```

The app opens with these files preloaded.

## Windows context menu integration

1. Copy `imazer.exe` to `%ProgramFiles%\Imazer\imazer.exe`.
2. Double-click `scripts/windows/install_context_menu.reg` and accept registry changes.
3. Right-click image files and select **Resize images**.

To remove:

1. Double-click `scripts/windows/uninstall_context_menu.reg`.

## Notes on performance

- Resizing runs in parallel using worker threads.
- Uses Lanczos3 filtering for quality.
- Avoids loading all images into memory at once during the batch loop.
