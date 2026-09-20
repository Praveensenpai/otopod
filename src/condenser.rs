use anyhow::{Context, Result};
use regex::Regex;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A single subtitle dialogue interval
#[derive(Debug, Clone)]
pub struct SubtitleInterval {
    pub start_secs: f64,
    pub end_secs: f64,
}

/// Parse an SRT timestamp like "00:01:23,456" into seconds
fn parse_srt_time(s: &str) -> Option<f64> {
    // Handle both comma and period as decimal separator
    let s = s.replace(',', ".");
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    let h: f64 = parts[0].parse().ok()?;
    let m: f64 = parts[1].parse().ok()?;
    let sec: f64 = parts[2].parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + sec)
}

/// Parse an ASS timestamp like "0:01:23.45" or "00:01:23.45" into seconds
fn parse_ass_time(s: &str) -> Option<f64> {
    let s = s.replace(',', ".");
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    let h: f64 = parts[0].parse().ok()?;
    let m: f64 = parts[1].parse().ok()?;
    let sec: f64 = parts[2].parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + sec)
}

/// Parse all subtitle intervals from an SRT or ASS file
pub fn parse_subtitle(sub_path: &Path) -> Result<Vec<SubtitleInterval>> {
    let content = std::fs::read_to_string(sub_path)
        .with_context(|| format!("Cannot read subtitle file: {}", sub_path.display()))?;

    let mut intervals = Vec::new();

    // Check for SRT pattern: 00:01:23,456 --> 00:01:25,789
    let re_srt =
        Regex::new(r"(\d{2}:\d{2}:\d{2}[,\.]\d{3})\s*-->\s*(\d{2}:\d{2}:\d{2}[,\.]\d{3})")?;
    for cap in re_srt.captures_iter(&content) {
        let start = parse_srt_time(&cap[1]);
        let end = parse_srt_time(&cap[2]);
        if let (Some(s), Some(e)) = (start, end) {
            if e > s {
                intervals.push(SubtitleInterval {
                    start_secs: s,
                    end_secs: e,
                });
            }
        }
    }

    if intervals.is_empty() {
        // Fallback/Try ASS pattern: Dialogue: 0,0:01:23.45,0:01:26.78,...
        let re_ass = Regex::new(
            r"(?i)Dialogue:\s*\d+,\s*(\d{1,2}:\d{2}:\d{2}[\.\,]\d{2,3}),\s*(\d{1,2}:\d{2}:\d{2}[\.\,]\d{2,3})",
        )?;
        for cap in re_ass.captures_iter(&content) {
            let start = parse_ass_time(&cap[1]);
            let end = parse_ass_time(&cap[2]);
            if let (Some(s), Some(e)) = (start, end) {
                if e > s {
                    intervals.push(SubtitleInterval {
                        start_secs: s,
                        end_secs: e,
                    });
                }
            }
        }
    }

    Ok(intervals)
}

/// Merge overlapping or adjacent intervals (with optional padding in seconds)
pub fn merge_intervals(intervals: &[SubtitleInterval], padding: f64) -> Vec<SubtitleInterval> {
    if intervals.is_empty() {
        return Vec::new();
    }

    let mut sorted = intervals.to_vec();
    sorted.sort_by(|a, b| {
        a.start_secs
            .partial_cmp(&b.start_secs)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut merged: Vec<SubtitleInterval> = Vec::new();
    let mut current = SubtitleInterval {
        start_secs: (sorted[0].start_secs - padding).max(0.0),
        end_secs: sorted[0].end_secs + padding,
    };

    for iv in &sorted[1..] {
        let padded_start = (iv.start_secs - padding).max(0.0);
        let padded_end = iv.end_secs + padding;

        if padded_start <= current.end_secs {
            // Overlapping — extend current
            current.end_secs = current.end_secs.max(padded_end);
        } else {
            merged.push(current.clone());
            current = SubtitleInterval {
                start_secs: padded_start,
                end_secs: padded_end,
            };
        }
    }
    merged.push(current);
    merged
}

fn build_expr_tree(items: &[String]) -> String {
    if items.is_empty() {
        return String::new();
    }
    if items.len() == 1 {
        return items[0].clone();
    }
    let mid = items.len() / 2;
    let left = build_expr_tree(&items[..mid]);
    let right = build_expr_tree(&items[mid..]);
    format!("({}+{})", left, right)
}

/// Build the ffmpeg `aselect` filter expression for all intervals
fn build_select_filter(intervals: &[SubtitleInterval]) -> String {
    let parts: Vec<String> = intervals
        .iter()
        .map(|iv| format!("between(t,{:.3},{:.3})", iv.start_secs, iv.end_secs))
        .collect();
    let expr = build_expr_tree(&parts);
    format!("aselect='{}',asetpts=N/SR/TB", expr)
}

/// Run a single-pass ffmpeg audio condense directly to the output .ogg file.
/// Returns the total duration of condensed audio in seconds.
pub fn condense_audio(
    video_path: &Path,
    intervals: &[SubtitleInterval],
    output_path: &Path,
) -> Result<f64> {
    let filter = build_select_filter(intervals);

    // Ensure output dir exists
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Cannot create output directory: {}", parent.display()))?;
    }

    let output = Command::new("ffmpeg")
        .args([
            "-y", // overwrite existing
            "-i",
            &video_path.to_string_lossy(), // input video
            "-af",
            &filter, // audio filter
            "-vn",   // no video
            "-map_metadata",
            "-1", // strip all metadata (removes chapters from mkv)
            "-c:a",
            "libopus", // opus codec (.opus) — modern, smaller, better quality
            "-b:a",
            "64k",                          // 64kbps opus ≈ 128kbps vorbis quality
            &output_path.to_string_lossy(), // output file
        ])
        .output()
        .context("Failed to spawn ffmpeg. Is ffmpeg installed and in PATH?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "ffmpeg exited with non-zero status while condensing audio from: {}\nFFmpeg Error:\n{}",
            video_path.display(),
            stderr
        );
    }

    // Calculate total duration of condensed audio
    let total_secs: f64 = intervals.iter().map(|iv| iv.end_secs - iv.start_secs).sum();
    Ok(total_secs)
}

/// Find the external subtitle file alongside a video file
/// Searches for: <stem>.ja.srt, <stem>.ja.ass, <stem>.srt, <stem>.ass
pub fn find_subtitle(video_path: &Path) -> Option<PathBuf> {
    let parent = video_path.parent()?;
    let stem = video_path.file_stem()?.to_string_lossy();

    let candidates = [
        format!("{}.ja.srt", stem),
        format!("{}.ja.ass", stem),
        format!("{}.srt", stem),
        format!("{}.ass", stem),
    ];

    for candidate in &candidates {
        let p = parent.join(candidate);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

/// Try to extract an embedded subtitle track from the video container using ffmpeg.
/// Prioritizes Japanese subtitle tracks (`jpn` / `ja`), falling back to first subtitle track (`0:s:0`).
pub fn extract_embedded_subtitle(video_path: &Path) -> Option<PathBuf> {
    let tmp_dir = std::env::temp_dir();
    let tmp_sub = tmp_dir.join(format!(
        "otopod_embedded_{}.srt",
        video_path.file_stem().unwrap_or_default().to_string_lossy()
    ));

    // Remove old temp sub if it exists
    let _ = std::fs::remove_file(&tmp_sub);

    // Try Japanese subtitle track first: jpn or ja, then fallback to first subtitle track 0:s:0
    let maps = ["0:s:m:language:jpn", "0:s:m:language:ja", "0:s:0"];

    for map in &maps {
        let status = Command::new("ffmpeg")
            .args([
                "-y",
                "-i",
                &video_path.to_string_lossy(),
                "-map",
                map,
                "-c:s",
                "srt",
                &tmp_sub.to_string_lossy(),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        if let Ok(st) = status {
            if st.success() && tmp_sub.exists() {
                if let Ok(meta) = std::fs::metadata(&tmp_sub) {
                    if meta.len() > 50 {
                        return Some(tmp_sub);
                    }
                }
            }
        }
    }

    None
}

/// Get the audio duration of a video file using ffprobe
pub fn get_audio_duration(video_path: &Path) -> Option<f64> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            &video_path.to_string_lossy(),
        ])
        .output()
        .ok()?;

    let s = String::from_utf8_lossy(&output.stdout);
    s.trim().parse::<f64>().ok()
}

/// Format seconds as mm:ss
pub fn format_duration(secs: f64) -> String {
    let total = secs as u64;
    let m = total / 60;
    let s = total % 60;
    format!("{:02}:{:02}", m, s)
}
