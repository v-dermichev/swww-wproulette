use rand::seq::IndexedRandom;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::Config;
use crate::state::State;

const EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "bmp"];

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTENSIONS.contains(&e.to_lowercase().as_str()))
}

pub fn collect_wallpapers(dir: &Path, trash_dir: &Path) -> Vec<PathBuf> {
    WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| {
            let path = e.path();
            // Skip .trash, .git, hidden dirs
            if path.is_dir() {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                return !name.starts_with('.') || path == dir;
            }
            true
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && is_image(e.path()))
        .filter(|e| !e.path().starts_with(trash_dir))
        .map(|e| e.path().to_path_buf())
        .collect()
}

pub fn pick_random(config: &Config, state: &State, starred_only: bool) -> Option<PathBuf> {
    let wallpapers = if starred_only {
        let starred = state.starred();
        starred.into_iter().filter(|p| p.exists()).collect::<Vec<_>>()
    } else {
        collect_wallpapers(&config.wallpaper_dir, state.trash_dir())
    };

    if wallpapers.is_empty() {
        return None;
    }

    let mut rng = rand::rng();
    wallpapers.choose(&mut rng).cloned()
}

pub fn apply(path: &Path, config: &Config) -> Result<(), String> {
    let status = std::process::Command::new("swww")
        .arg("img")
        .arg(path)
        .arg("--transition-type")
        .arg(&config.transition_type)
        .arg("--transition-duration")
        .arg(config.transition_duration.to_string())
        .status()
        .map_err(|e| format!("Failed to run swww: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err("swww failed".into())
    }
}
