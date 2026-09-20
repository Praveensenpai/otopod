use anyhow::Result;
use inquire::{Select, Text};
use std::cmp::Ordering;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub const VIDEO_EXTENSIONS: &[&str] = &[
    "mkv", "mp4", "avi", "webm", "m4v", "mov", "ts", "flv", "wmv",
];

pub const SCAN_MAX_DEPTH: usize = 8;

pub fn is_video_extension(ext: &str) -> bool {
    VIDEO_EXTENSIONS
        .iter()
        .any(|&e| e.eq_ignore_ascii_case(ext))
}

pub fn is_video_file(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(is_video_extension)
}

pub fn scan_video_files(base_dir: &Path, max_depth: usize) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let walker = WalkDir::new(base_dir)
        .max_depth(max_depth)
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0
                || entry
                    .file_name()
                    .to_str()
                    .is_none_or(|s| !s.starts_with('.'))
        });

    for entry in walker.flatten() {
        if is_video_file(entry.path()) {
            files.push(entry.path().to_path_buf());
        }
    }

    files
}

pub fn format_video_display(
    path: &Path,
    videos_dir: &Path,
    cwd: &Path,
    cwd_is_home: bool,
) -> String {
    if !cwd_is_home {
        if let Ok(rel) = path.strip_prefix(cwd) {
            let rel_str = rel.to_string_lossy();
            if !rel_str.is_empty() {
                return rel_str.to_string();
            }
        }
    }

    if let Ok(rel) = path.strip_prefix(videos_dir) {
        let rel_str = rel.to_string_lossy();
        if !rel_str.is_empty() {
            return rel_str.to_string();
        }
    }

    let file_name = path
        .file_name()
        .map(|f| f.to_string_lossy())
        .unwrap_or_default();
    let parent = path
        .parent()
        .and_then(|par| par.file_name())
        .map(|f| f.to_string_lossy())
        .unwrap_or_default();

    if parent.is_empty() {
        file_name.to_string()
    } else {
        format!("{}/{}", parent, file_name)
    }
}

pub fn discover_video_files(
    videos_dir: &Path,
    cwd: &Path,
    home_dir: Option<&Path>,
) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut files = Vec::new();

    let canonical_videos = videos_dir.canonicalize().ok();
    let canonical_home = home_dir.and_then(|h| h.canonicalize().ok());
    let canonical_cwd = cwd.canonicalize().ok();

    // 1. Scan ~/Videos up to SCAN_MAX_DEPTH
    if videos_dir.exists() {
        for path in scan_video_files(videos_dir, SCAN_MAX_DEPTH) {
            let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
            if seen.insert(canonical) {
                files.push(path);
            }
        }
    }

    // 2. Scan cwd if it is not home dir and not videos_dir
    let is_home = canonical_home.is_some() && canonical_home == canonical_cwd;
    let is_same_as_videos = canonical_videos.is_some() && canonical_videos == canonical_cwd;

    let should_scan_cwd = !is_home && !is_same_as_videos && cwd.exists();

    if should_scan_cwd {
        for path in scan_video_files(cwd, SCAN_MAX_DEPTH) {
            let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
            if seen.insert(canonical) {
                files.push(path);
            }
        }
    }

    sort_video_files(&mut files, cwd, is_home);
    files
}

pub fn sort_video_files(files: &mut [PathBuf], cwd: &Path, cwd_is_home: bool) {
    let canonical_cwd = cwd.canonicalize().ok();

    let mut annotated: Vec<(bool, PathBuf)> = files
        .iter()
        .map(|p| {
            let in_cwd = !cwd_is_home
                && canonical_cwd
                    .as_ref()
                    .is_some_and(|c| p.canonicalize().is_ok_and(|can| can.starts_with(c)));
            (in_cwd, p.clone())
        })
        .collect();

    annotated.sort_by(|(left_in_cwd, left_path), (right_in_cwd, right_path)| {
        match (left_in_cwd, right_in_cwd) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => natural_path_cmp(left_path, right_path),
        }
    });

    for (i, (_, path)) in annotated.into_iter().enumerate() {
        files[i] = path;
    }
}

pub fn select_video_file() -> Result<PathBuf> {
    // 1. Check if CLI argument was provided
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        let p = PathBuf::from(&args[0]);
        if p.exists() {
            return Ok(p);
        }
        let cwd_p = std::env::current_dir()?.join(&args[0]);
        if cwd_p.exists() {
            return Ok(cwd_p);
        }
        anyhow::bail!("File not found: {}", args[0]);
    }

    let videos_dir = dirs::video_dir().unwrap_or_else(|| PathBuf::from("./"));
    let home_dir = dirs::home_dir();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let files = discover_video_files(&videos_dir, &cwd, home_dir.as_deref());

    if files.is_empty() {
        let custom_path_str = Text::new("No video files found. Enter video file path:").prompt()?;
        return Ok(PathBuf::from(custom_path_str));
    }

    let cwd_is_home = home_dir
        .and_then(|h| h.canonicalize().ok())
        .zip(cwd.canonicalize().ok())
        .is_some_and(|(h, c)| h == c);

    let file_displays: Vec<String> = files
        .iter()
        .map(|p| format_video_display(p, &videos_dir, &cwd, cwd_is_home))
        .collect();

    let prompt_msg = if cwd_is_home {
        "Select video file from ~/Videos:"
    } else {
        "Select anime video file:"
    };

    let choice = Select::new(prompt_msg, file_displays.clone()).prompt()?;
    let selected_index = file_displays
        .iter()
        .position(|display| display == &choice)
        .unwrap_or(0);

    Ok(files[selected_index].clone())
}

pub fn natural_path_cmp(left: &Path, right: &Path) -> Ordering {
    natural_cmp(&left.to_string_lossy(), &right.to_string_lossy())
}

pub fn natural_cmp(left: &str, right: &str) -> Ordering {
    let left_bytes = left.as_bytes();
    let right_bytes = right.as_bytes();
    let (mut left_index, mut right_index) = (0, 0);

    while left_index < left_bytes.len() && right_index < right_bytes.len() {
        let left_is_digit = left_bytes[left_index].is_ascii_digit();
        let right_is_digit = right_bytes[right_index].is_ascii_digit();

        if left_is_digit && right_is_digit {
            let left_start = left_index;
            let right_start = right_index;

            while left_index < left_bytes.len() && left_bytes[left_index].is_ascii_digit() {
                left_index += 1;
            }
            while right_index < right_bytes.len() && right_bytes[right_index].is_ascii_digit() {
                right_index += 1;
            }

            let left_number = &left_bytes[left_start..left_index];
            let right_number = &right_bytes[right_start..right_index];
            let left_significant = left_number
                .iter()
                .position(|digit| *digit != b'0')
                .unwrap_or(left_number.len().saturating_sub(1));
            let right_significant = right_number
                .iter()
                .position(|digit| *digit != b'0')
                .unwrap_or(right_number.len().saturating_sub(1));
            let left_number = &left_number[left_significant..];
            let right_number = &right_number[right_significant..];

            match left_number.len().cmp(&right_number.len()) {
                Ordering::Equal => match left_number.cmp(right_number) {
                    Ordering::Equal => {}
                    order => return order,
                },
                order => return order,
            }
        } else {
            let left_byte = left_bytes[left_index].to_ascii_lowercase();
            let right_byte = right_bytes[right_index].to_ascii_lowercase();

            match left_byte.cmp(&right_byte) {
                Ordering::Equal => {
                    left_index += 1;
                    right_index += 1;
                }
                order => return order,
            }
        }
    }

    left_bytes.len().cmp(&right_bytes.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, File};
    use tempfile::tempdir;

    #[test]
    fn test_is_video_extension() {
        assert!(is_video_extension("mkv"));
        assert!(is_video_extension("MKV"));
        assert!(is_video_extension("mp4"));
        assert!(is_video_extension("mov"));
        assert!(is_video_extension("ts"));
        assert!(!is_video_extension("koto"));
        assert!(!is_video_extension("srt"));
        assert!(!is_video_extension("txt"));
    }

    #[test]
    fn test_sorts_episode_numbers_naturally() {
        let mut episodes = vec!["episode 10.mkv", "episode 02.mkv", "episode 3.mkv"];
        episodes.sort_by(|left, right| natural_cmp(left, right));
        assert_eq!(
            episodes,
            vec!["episode 02.mkv", "episode 3.mkv", "episode 10.mkv"]
        );
    }

    #[test]
    fn test_format_video_display_relative() {
        let base = Path::new("/home/user/Videos");
        let cwd = Path::new("/home/user");
        let video = Path::new("/home/user/Videos/Anime/Non Non Biyori/Season 01/E01.mkv");
        let display = format_video_display(video, base, cwd, true);
        assert_eq!(display, "Anime/Non Non Biyori/Season 01/E01.mkv");

        let cwd_sub = Path::new("/home/user/Videos/Anime/Non Non Biyori/Season 01");
        let display_in_cwd = format_video_display(video, base, cwd_sub, false);
        assert_eq!(display_in_cwd, "E01.mkv");
    }

    #[test]
    fn test_discover_video_files_deduplicates_and_finds_cwd() {
        let root = tempdir().unwrap();
        let videos = root.path().join("Videos");
        let custom_dir = root.path().join("External");
        let deep_videos = videos.join("Anime").join("Show").join("Season 01");
        create_dir_all(&deep_videos).unwrap();
        create_dir_all(&custom_dir).unwrap();

        let ep1 = deep_videos.join("ep01.mkv");
        let ep_custom = custom_dir.join("movie.mp4");
        File::create(&ep1).unwrap();
        File::create(&ep_custom).unwrap();

        let files = discover_video_files(&videos, &custom_dir, None);
        assert_eq!(files.len(), 2);
        assert!(files.contains(&ep1));
        assert!(files.contains(&ep_custom));

        let files_inside = discover_video_files(&videos, &deep_videos, None);
        assert_eq!(files_inside.len(), 1);
        assert_eq!(files_inside[0], ep1);
    }

    #[test]
    fn test_sort_video_files_prioritizes_cwd() {
        let root = tempdir().unwrap();
        let current_anime = root.path().join("Anime").join("TargetShow");
        let other_anime = root.path().join("Anime").join("OtherShow");
        create_dir_all(&current_anime).unwrap();
        create_dir_all(&other_anime).unwrap();

        let other_ep1 = other_anime.join("ep01.mkv");
        let target_ep1 = current_anime.join("ep01.mkv");
        File::create(&other_ep1).unwrap();
        File::create(&target_ep1).unwrap();

        let mut files = vec![other_ep1.clone(), target_ep1.clone()];
        sort_video_files(&mut files, &current_anime, false);

        assert_eq!(files[0], target_ep1);
        assert_eq!(files[1], other_ep1);
    }
}
