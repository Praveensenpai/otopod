mod condenser;
mod config;
mod scanner;
mod ui;

use anyhow::Result;
use inquire::{Select, Text};
use std::path::PathBuf;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [flag] if matches!(flag.as_str(), "-h" | "--help") => {
            println!(
                "otopod {}\n\nUsage: otopod [OPTION | VIDEO_FILE]\n\nOptions:\n  -h, --help       Show this help message\n  -v, -V, --version  Show version",
                env!("CARGO_PKG_VERSION")
            );
            return Ok(());
        }
        [flag] if matches!(flag.as_str(), "-v" | "-V" | "--version") => {
            println!("otopod {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        _ => {}
    }

    inquire::set_global_render_config(ui::custom_render_config());
    ui::print_banner();

    let cfg = config::Config::load()?;
    ui::print_info(&format!(
        "Output dir: {}",
        cfg.resolved_output_dir().display()
    ));

    // STEP 1: Select video file
    ui::print_step(1, 3, "Select Raw Anime Video File");
    let video_path = scanner::select_video_file()?;
    let selected_name = video_path
        .file_name()
        .map(|f| f.to_string_lossy())
        .unwrap_or_else(|| "video".into());
    ui::print_success(&format!("Selected: {}", selected_name));

    // STEP 2: Find subtitle file
    ui::print_step(2, 3, "Locate Synced Subtitle File");
    let sub_path = match condenser::find_subtitle(&video_path) {
        Some(p) => {
            let sub_name = p
                .file_name()
                .map(|f| f.to_string_lossy())
                .unwrap_or_else(|| "subtitle".into());
            ui::print_success(&format!("Found external subtitle: {}", sub_name));
            p
        }
        None => {
            let extract_spinner =
                ui::create_spinner("Checking for embedded subtitle track in video container...");
            let embedded = condenser::extract_embedded_subtitle(&video_path);
            extract_spinner.finish_and_clear();

            match embedded {
                Some(p) => {
                    ui::print_success(
                        "Found and extracted embedded subtitle stream from container!",
                    );
                    p
                }
                None => {
                    let stem = video_path
                        .file_stem()
                        .map(|s| s.to_string_lossy())
                        .unwrap_or_else(|| "video".into());
                    let expected_sub = format!("{}.ja.srt", stem);
                    ui::print_warning("No external or embedded subtitle found alongside video.");
                    ui::print_info(&format!(
                        "Run `subsink` first to download and sync {} file.",
                        expected_sub
                    ));

                    let options = vec!["Exit to run subsink", "Enter subtitle file path manually"];
                    let choice = Select::new("What would you like to do?", options).prompt()?;

                    if choice.starts_with("Exit") {
                        println!();
                        return Ok(());
                    } else {
                        let custom = Text::new("Enter subtitle file path:").prompt()?;
                        PathBuf::from(custom)
                    }
                }
            }
        }
    };

    // STEP 3: Parse, merge, condense
    ui::print_step(3, 3, "Condensing Audio (Single-Pass)");

    let parse_spinner = ui::create_spinner("Parsing subtitle timestamps...");
    let raw_intervals = condenser::parse_subtitle(&sub_path)?;
    parse_spinner.finish_and_clear();
    ui::print_success(&format!("Parsed {} subtitle lines", raw_intervals.len()));

    if raw_intervals.is_empty() {
        ui::print_error("No valid subtitle intervals found. Aborting.");
        return Ok(());
    }

    // Merge overlapping intervals with 0.3s padding
    let merged = condenser::merge_intervals(&raw_intervals, 0.3);
    ui::print_info(&format!("Merged into {} audio segments", merged.len()));

    // Determine output path from config
    let stem = video_path
        .file_stem()
        .map(|s| s.to_string_lossy())
        .unwrap_or_else(|| "audio".into());
    let output_dir = cfg.resolved_output_dir();
    let output_path = output_dir.join(format!("{}.opus", stem));

    let out_file_display = output_path
        .file_name()
        .map(|f| f.to_string_lossy())
        .unwrap_or_else(|| "audio.opus".into());
    let condense_msg = format!(
        "Condensing {} segments → {} ...",
        merged.len(),
        out_file_display
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
