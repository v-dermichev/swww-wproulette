use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub struct State {
    data_dir: PathBuf,
    trash_dir: PathBuf,
}

impl State {
    pub fn new(wallpaper_dir: &Path) -> Self {
        let data_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("wproulette");
        let trash_dir = wallpaper_dir.join(".trash");
        fs::create_dir_all(&data_dir).ok();
        fs::create_dir_all(&trash_dir).ok();
        Self { data_dir, trash_dir }
    }

    // Current wallpaper
    pub fn current(&self) -> Option<PathBuf> {
        let path = self.data_dir.join("current");
        fs::read_to_string(&path).ok().map(|s| PathBuf::from(s.trim()))
    }

    pub fn set_current(&self, path: &Path) {
        fs::write(self.data_dir.join("current"), path.to_string_lossy().as_ref()).ok();
    }

    // Starred list
    fn starred_path(&self) -> PathBuf {
        self.data_dir.join("starred")
    }

    pub fn starred(&self) -> HashSet<PathBuf> {
        let path = self.starred_path();
        if !path.exists() {
            return HashSet::new();
        }
        fs::read_to_string(&path)
            .unwrap_or_default()
            .lines()
            .filter(|l| !l.is_empty())
            .map(PathBuf::from)
            .collect()
    }

    pub fn is_starred(&self, path: &Path) -> bool {
        self.starred().contains(path)
    }

    pub fn toggle_star(&self, path: &Path) -> bool {
        let mut starred = self.starred();
        let was_starred = starred.contains(path);
        if was_starred {
            starred.remove(path);
        } else {
            starred.insert(path.to_path_buf());
        }
        let content: String = starred.iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(self.starred_path(), content).ok();
        !was_starred
    }

    // Trash with original path preservation
    fn trash_manifest_path(&self) -> PathBuf {
        self.data_dir.join("trash_manifest")
    }

    fn file_hash(path: &Path) -> String {
        let data = fs::read(path).unwrap_or_default();
        let hash = Sha256::digest(&data);
        hex::encode(&hash[..8])
    }

    pub fn trash(&self, path: &Path) -> Result<(), String> {
        if !path.exists() {
            return Err("File not found".into());
        }

        // Preserve subdir structure relative to wallpaper_dir
        let relative = path.strip_prefix(
            path.ancestors()
                .find(|a| self.trash_dir.parent().is_some_and(|p| p == *a))
                .unwrap_or(path.parent().unwrap_or(path))
        ).unwrap_or(path.file_name().map(Path::new).unwrap_or(path));

        // Add hash suffix to filename for uniqueness
        let hash = Self::file_hash(path);
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
        let trash_name = format!("{}.{}{}", stem, hash, ext);

        // Preserve subdirectory
        let trash_subdir = self.trash_dir.join(
            relative.parent().unwrap_or(Path::new(""))
        );
        fs::create_dir_all(&trash_subdir).map_err(|e| e.to_string())?;

        let trash_path = trash_subdir.join(&trash_name);

        // Record original path in manifest
        let manifest = self.trash_manifest_path();
        let entry = format!("{}|{}\n", trash_path.to_string_lossy(), path.to_string_lossy());
        let mut content = fs::read_to_string(&manifest).unwrap_or_default();
        content.push_str(&entry);
        fs::write(&manifest, content).map_err(|e| e.to_string())?;

        // Move file
        fs::rename(path, &trash_path).map_err(|e| e.to_string())?;

        // Remove from starred if present
        let mut starred = self.starred();
        if starred.remove(path) {
            let content: String = starred.iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join("\n");
            fs::write(self.starred_path(), content).ok();
        }

        Ok(())
    }

    pub fn restore_last(&self) -> Result<PathBuf, String> {
        let manifest = self.trash_manifest_path();
        let content = fs::read_to_string(&manifest).map_err(|_| "No trash history".to_string())?;
        let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();

        let last = lines.last().ok_or("Trash is empty")?;
        let parts: Vec<&str> = last.splitn(2, '|').collect();
        if parts.len() != 2 {
            return Err("Corrupt manifest entry".into());
        }

        let trash_path = PathBuf::from(parts[0]);
        let original_path = PathBuf::from(parts[1]);

        if !trash_path.exists() {
            return Err("Trashed file no longer exists".into());
        }

        // Ensure parent dir exists
        if let Some(parent) = original_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        fs::rename(&trash_path, &original_path).map_err(|e| e.to_string())?;

        // Remove last line from manifest
        let remaining: String = lines[..lines.len() - 1].join("\n");
        fs::write(&manifest, if remaining.is_empty() { remaining } else { remaining + "\n" })
            .map_err(|e| e.to_string())?;

        Ok(original_path)
    }

    pub fn trashed_entries(&self, n: usize) -> Vec<(PathBuf, PathBuf)> {
        let manifest = self.trash_manifest_path();
        let content = fs::read_to_string(&manifest).unwrap_or_default();
        let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
        lines.iter().rev().take(n).filter_map(|line| {
            let parts: Vec<&str> = line.splitn(2, '|').collect();
            if parts.len() == 2 {
                Some((PathBuf::from(parts[0]), PathBuf::from(parts[1])))
            } else {
                None
            }
        }).collect()
    }

    pub fn trash_dir(&self) -> &Path {
        &self.trash_dir
    }
}
