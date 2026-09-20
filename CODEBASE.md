# CODEBASE.md: otopod Semantic Digest

> **Notice**: This file is an AI-optimized semantic index. Do not write narrative prose. Keep token density high.

## 1. System Topology & Data Flow
```text
CLI Entry (src/main.rs)
  │
  ├──> Config (src/config.rs) ──> ~/.config/otopod/config.toml
  ├──> Scanner (src/scanner.rs) ──> WalkDir ~/Videos & CWD (depth 8) ──> Deduplicated Raw Video
  ├──> Condenser (src/condenser.rs) ──> Locate .ja.srt / extract embedded sub ──> Single-pass ffmpeg
  └──> UI (src/ui.rs) ──> Inquire Prompts, Spinners & Render Config
```

## 2. Global Constraints & Architecture Patterns
- **Primary Language & Edition**: Rust 2021 edition (1.80+)
- **Architectural Paradigm**: Modular CLI (`scanner`, `condenser`, `config`, `ui`)
- **Hard Constraints**: <400 lines/file (<300 soft), <60 lines/fn, zero production `unwrap()`/`expect()`, 0 warnings (`cargo clippy -- -D warnings`).
- **Target Distribution**: Linux `x86_64` standalone binary via GitHub Releases and local binary install (`~/.local/bin/otopod`).

## 3. Module & Interface Skeleton

### `src/main.rs` (Role: cli/orchestrator, Lines: 141)
- **Responsibility**: Orchestrates 3-step audio condense flow (video selection, subtitle discovery, single-pass ffmpeg audio extraction).
- **Imports**: `crate::condenser`, `crate::config`, `crate::scanner`, `crate::ui`, `inquire::*`, `anyhow::Result`, `std::path::PathBuf`
- **Public Functions & Signatures**:
  ```rust
  fn main() -> Result<()>
  ```
- **Consumers**: Process entrypoint (`bin "otopod"`).
- **Side Effects / I/O**: Terminal I/O, subprocess spawns, disk output write to MPD audio directory.

### `src/scanner.rs` (Role: infra/io, Lines: 353)
- **Responsibility**: Recursively indexes anime video files in `~/Videos` and current working directory up to depth 8 with canonical deduplication, relative path display formatting, and CWD proximity prioritization.
- **Imports**: `walkdir::WalkDir`, `inquire::{Select, Text}`, `anyhow::Result`, `std::cmp::Ordering`, `std::collections::HashSet`, `std::path::{Path, PathBuf}`
- **Constants**:
  ```rust
  pub const VIDEO_EXTENSIONS: &[&str] = &["mkv", "mp4", "avi", "webm", "m4v", "mov", "ts", "flv", "wmv"];
  pub const SCAN_MAX_DEPTH: usize = 8;
  ```
- **Public Functions & Signatures**:
  ```rust
  pub fn is_video_extension(ext: &str) -> bool
  pub fn is_video_file(path: &Path) -> bool
  pub fn scan_video_files(base_dir: &Path, max_depth: usize) -> Vec<PathBuf>
  pub fn format_video_display(path: &Path, videos_dir: &Path, cwd: &Path, cwd_is_home: bool) -> String
  pub fn discover_video_files(videos_dir: &Path, cwd: &Path, home_dir: Option<&Path>) -> Vec<PathBuf>
  pub fn sort_video_files(files: &mut [PathBuf], cwd: &Path, cwd_is_home: bool)
  pub fn select_video_file() -> Result<PathBuf>
  pub fn natural_path_cmp(left: &Path, right: &Path) -> Ordering
  pub fn natural_cmp(left: &str, right: &str) -> Ordering
  ```
- **Consumers**: `src/main.rs`
- **Side Effects / I/O**: Filesystem directory walking (`dirs::video_dir()` and `std::env::current_dir()`), interactive prompt.

### `src/condenser.rs` (Role: domain/infra, Lines: 288)
- **Responsibility**: Subtitle timestamp parsing (SRT / ASS), interval merging with padding, embedded subtitle extraction via ffmpeg, and single-pass `aselect` audio condensing to opus.
- **Imports**: `regex::Regex`, `std::process::Command`, `anyhow::{Context, Result}`, `std::path::{Path, PathBuf}`
- **Types & Enums**:
  ```rust
  pub struct SubtitleInterval { pub start_secs: f64, pub end_secs: f64 }
  ```
- **Public Functions & Signatures**:
  ```rust
  pub fn parse_subtitle(sub_path: &Path) -> Result<Vec<SubtitleInterval>>
  pub fn merge_intervals(intervals: &[SubtitleInterval], padding: f64) -> Vec<SubtitleInterval>
  pub fn condense_audio(video_path: &Path, intervals: &[SubtitleInterval], output_path: &Path) -> Result<f64>
  pub fn find_subtitle(video_path: &Path) -> Option<PathBuf>
  pub fn extract_embedded_subtitle(video_path: &Path) -> Option<PathBuf>
  pub fn get_audio_duration(video_path: &Path) -> Option<f64>
  pub fn format_duration(secs: f64) -> String
  ```
- **Consumers**: `src/main.rs`
- **Side Effects / I/O**: Reads subtitle files, invokes `ffmpeg` and `ffprobe`, writes condensed `.opus` files.

### `src/config.rs` (Role: infra/config, Lines: 81)
- **Responsibility**: Loads user configuration from `~/.config/otopod/config.toml` with default fallback to `~/Music/immersionpod/current/`.
- **Imports**: `serde::{Deserialize, Serialize}`, `anyhow::Result`, `std::path::PathBuf`
- **Types & Enums**:
  ```rust
  pub struct Config { pub output_dir: Option<String> }
  ```
- **Public Functions & Signatures**:
  ```rust
  impl Config {
      pub fn load() -> Result<Self>
      pub fn resolved_output_dir(&self) -> PathBuf
  }
  ```
- **Consumers**: `src/main.rs`
- **Side Effects / I/O**: Reads/writes `~/.config/otopod/config.toml`.

### `src/ui.rs` (Role: tui/presentation, Lines: 95)
- **Responsibility**: Terminal styles, banners, step indicators, render configs, and progress spinners.
- **Imports**: `console::style`, `indicatif::{ProgressBar, ProgressStyle}`, `inquire::ui::*`
- **Public Functions & Signatures**:
  ```rust
  pub fn print_banner()
  pub fn custom_render_config() -> RenderConfig<'static>
  pub fn create_spinner(msg: &'static str) -> ProgressBar
  pub fn create_spinner_owned(msg: String) -> ProgressBar
  pub fn print_step(step: usize, total: usize, title: &str)
  pub fn print_success(msg: &str)
  pub fn print_info(msg: &str)
  pub fn print_warning(msg: &str)
  pub fn print_error(msg: &str)
  ```
- **Consumers**: `src/main.rs`
- **Side Effects / I/O**: ANSI stdout printing.

## 4. Execution Lifecycle Trace
1. **Startup**: Entrypoint loads config and sets custom Inquire render configuration.
2. **Step 1 (Scan Video)**: `scanner::select_video_file()` checks CLI argument, then scans `~/Videos` and cwd up to depth 8 with canonical deduplication.
3. **Step 2 (Locate Subtitle)**: `condenser::find_subtitle()` checks for `<stem>.ja.srt`, `<stem>.ja.ass`, or falls back to extracting embedded Japanese subtitle track via ffmpeg.
4. **Step 3 (Condense Audio)**: Parses subtitle intervals, merges dialogue with 0.3s padding, and executes single-pass `ffmpeg -af aselect=...` directly into output `.opus`.
5. **Output**: Reports retained audio percentage and displays MPD update command.

## 5. Verification Commands
```bash
# Build
cargo build --release

# Test
cargo test --all-targets

# Lint & Format
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## 6. Recent Iteration Changes
- **2026-09-20 (v0.1.7)**:
  - Added contextual proximity sorting (`sort_video_files`): video files under current working directory (`cwd`) are placed first, followed by natural episode naming order.
- **2026-09-20 (v0.1.6)**:
  - Added `src/scanner.rs` with deep recursive search (`max_depth: 8`), expanded video extensions, natural episode sorting, and relative display path formatting.
  - Added current directory (`cwd`) traversal with canonical deduplication against `~/Videos`.
  - Removed production `unwrap()` calls on file paths.
  - Added unit test suite covering video extensions, natural sorting, relative path displays, and deduplication.
  - Added AI index in `CODEBASE.md`.
