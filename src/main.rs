mod condenser;
mod ui;

use anyhow::Result;
use inquire::{Select, Text};
use std::path::PathBuf;
use walkdir::WalkDir;

fn main() -> Result<()> {
    inquire::set_global_render_config(ui::custom_render_config());
    ui::print_banner();

    // STEP 1: Select video file
    ui::print_step(1, 3, "Select Raw Anime Video File");
    let video_path = select_video_file()?;
    ui::print_success(&format!(
        "Selected: {}",
        video_path.file_name().unwrap().to_string_lossy()
    ));

    // STEP 2: Find subtitle file
    ui::print_step(2, 3, "Locate Synced Subtitle File");
    let sub_path = match condenser::find_subtitle(&video_path) {
        Some(p) => {
            ui::print_success(&format!(
                "Found subtitle: {}",
                p.file_name().unwrap().to_string_lossy()
            ));
            p
        }
        None => {
            ui::print_warning("No external subtitle found alongside video.");
            ui::print_info("Run `subsink` first to download and sync a .ja.srt file.");
            let custom = Text::new("Enter subtitle file path manually:").prompt()?;
            PathBuf::from(custom)
        }
    };

    // STEP 3: Parse, merge, condense
    ui::print_step(3, 3, "Condensing Audio (Single-Pass)");

    let parse_spinner = ui::create_spinner("Parsing subtitle timestamps...");
    let raw_intervals = condenser::parse_srt(&sub_path)?;
    parse_spinner.finish_and_clear();
    ui::print_success(&format!("Parsed {} subtitle lines", raw_intervals.len()));

    if raw_intervals.is_empty() {
        ui::print_error("No valid subtitle intervals found. Aborting.");
        return Ok(());
    }

    // Merge overlapping intervals with 0.3s padding
    let merged = condenser::merge_intervals(&raw_intervals, 0.3);
    ui::print_info(&format!("Merged into {} audio segments", merged.len()));

    // Determine output path: ~/Music/immersionpod/current/<stem>.ogg
    let stem = video_path.file_stem().unwrap().to_string_lossy();
    let output_dir = dirs::audio_dir()
        .unwrap_or_else(|| PathBuf::from("~/Music"))
        .join("immersionpod")
        .join("current");

    let output_path = output_dir.join(format!("{}.opus", stem));

    let condense_msg = format!(
        "Condensing {} segments → {} ...",
        merged.len(),
        output_path.file_name().unwrap().to_string_lossy()
    );
    let condense_spinner = ui::create_spinner_owned(condense_msg);

    let total_secs = condenser::condense_audio(&video_path, &merged, &output_path)?;
    condense_spinner.finish_and_clear();

    ui::print_success(&format!(
        "Done! Condensed audio: {}  ({})",
        condenser::format_duration(total_secs),
        output_path.display()
    ));

    // Audio duration comparison
    if let Some(full_dur) = condenser::get_audio_duration(&video_path) {
        let pct = (total_secs / full_dur) * 100.0;
        ui::print_info(&format!(
            "Retained {:.1}% of full episode audio ({} dialogue / {} total)",
            pct,
            condenser::format_duration(total_secs),
            condenser::format_duration(full_dur),
        ));
    }

    println!();
    ui::print_info("Add to MPD playlist:  mpc update && mpc add");
    println!();

    Ok(())
}

fn select_video_file() -> Result<PathBuf> {
    let videos_dir = dirs::video_dir().unwrap_or_else(|| PathBuf::from("./"));

    let mut files: Vec<PathBuf> = Vec::new();

    // Check if a CLI argument was passed (e.g. otopod anime.mkv)
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        let p = PathBuf::from(&args[0]);
        if p.exists() {
            return Ok(p);
        }
        // Try relative to cwd
        let cwd_p = std::env::current_dir()?.join(&args[0]);
        if cwd_p.exists() {
            return Ok(cwd_p);
        }
        anyhow::bail!("File not found: {}", args[0]);
    }

    for entry in WalkDir::new(&videos_dir).max_depth(3).into_iter().flatten() {
        if entry.file_type().is_file() {
            if let Some(ext) = entry.path().extension().and_then(|s| s.to_str()) {
                if matches!(ext.to_lowercase().as_str(), "mkv" | "mp4" | "avi" | "webm") {
                    files.push(entry.path().to_path_buf());
                }
            }
        }
    }

    // Fallback to current dir
    if files.is_empty() {
        for entry in WalkDir::new("./").max_depth(2).into_iter().flatten() {
            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension().and_then(|s| s.to_str()) {
                    if matches!(ext.to_lowercase().as_str(), "mkv" | "mp4" | "avi" | "webm") {
                        files.push(entry.path().to_path_buf());
                    }
                }
            }
        }
    }

    if files.is_empty() {
        let custom = Text::new("No video files found. Enter video file path:").prompt()?;
        return Ok(PathBuf::from(custom));
    }

    let display: Vec<String> = files
        .iter()
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy();
            let parent = p
                .parent()
                .and_then(|par| par.file_name())
                .map(|f| f.to_string_lossy())
                .unwrap_or_default();
            format!("{}/{}", parent, name)
        })
        .collect();

    let choice = Select::new("Select video file:", display.clone()).prompt()?;
    let idx = display.iter().position(|d| d == &choice).unwrap_or(0);
    Ok(files[idx].clone())
}
