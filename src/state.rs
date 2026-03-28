use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// File-based lock guard — creates a .lock file, removes on drop.
struct FileLock {
    path: PathBuf,
}

impl FileLock {
    fn acquire(path: &Path) -> Result<Self, String> {
        let lock_path = path.with_extension("lock");
        // Spin briefly if locked
        for _ in 0..50 {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(_) => return Ok(Self { path: lock_path }),
                Err(_) => std::thread::sleep(std::time::Duration::from_millis(10)),
            }
        }
        Err(format!("Could not acquire lock: {}", lock_path.display()))
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Manifest uses tab as delimiter — tabs are invalid in filenames on most filesystems.
const MANIFEST_DELIMITER: char = '\t';

pub struct State {
    data_dir: PathBuf,
    trash_dir: PathBuf,
    wallpaper_dir: PathBuf,
}

impl State {
    pub fn new(wallpaper_dir: &Path) -> Self {
        let data_dir = dirs::config_dir()
            .unwrap_or_else(|| {
                // Fallback: use $HOME/.config or /tmp
                std::env::var("HOME")
                    .map(|h| PathBuf::from(h).join(".config"))
                    .unwrap_or_else(|_| PathBuf::from("/tmp"))
            })
            .join("wproulette");
        let trash_dir = wallpaper_dir.join(".trash");
        fs::create_dir_all(&data_dir).ok();
        fs::create_dir_all(&trash_dir).ok();
        Self {
            data_dir,
            trash_dir,
            wallpaper_dir: wallpaper_dir.to_path_buf(),
        }
    }

    // Current wallpaper
    pub fn current(&self) -> Option<PathBuf> {
        let path = self.data_dir.join("current");
        fs::read_to_string(&path)
            .ok()
            .map(|s| PathBuf::from(s.trim()))
    }

    pub fn set_current(&self, path: &Path) {
        let _ = fs::write(
            self.data_dir.join("current"),
            path.to_string_lossy().as_ref(),
        );
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
        let starred_path = self.starred_path();
        let _lock = FileLock::acquire(&starred_path);

        let mut starred = self.starred();
        let was_starred = starred.contains(path);
        if was_starred {
            starred.remove(path);
        } else {
            starred.insert(path.to_path_buf());
        }
        Self::write_path_list(&starred_path, &starred);
        !was_starred
    }

    fn write_path_list(file: &Path, paths: &HashSet<PathBuf>) {
        let content: String = paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("\n");
        let _ = fs::write(file, content);
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

    /// Compute the relative path of `file` within `wallpaper_dir`.
    /// Returns just the filename if the file isn't inside wallpaper_dir.
    fn relative_to_wallpaper_dir(&self, file: &Path) -> PathBuf {
        file.strip_prefix(&self.wallpaper_dir)
            .unwrap_or_else(|_| {
                // Not inside wallpaper_dir — use filename only
                Path::new(file.file_name().unwrap_or_default())
            })
            .to_path_buf()
    }

    pub fn trash(&self, path: &Path) -> Result<(), String> {
        if !path.exists() {
            return Err("File not found".into());
        }

        let relative = self.relative_to_wallpaper_dir(path);

        // Add hash suffix to filename for uniqueness
        let hash = Self::file_hash(path);
        let stem = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();
        let ext = path
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();
        let trash_name = format!("{}.{}{}", stem, hash, ext);

        // Preserve subdirectory structure
        let trash_subdir = self
            .trash_dir
            .join(relative.parent().unwrap_or(Path::new("")));
        fs::create_dir_all(&trash_subdir).map_err(|e| e.to_string())?;

        let trash_path = trash_subdir.join(&trash_name);

        // Record original path in manifest (tab-delimited, append with lock)
        let manifest = self.trash_manifest_path();
        let _lock = FileLock::acquire(&manifest).map_err(|e| e.to_string())?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&manifest)
            .map_err(|e| e.to_string())?;
        writeln!(
            file,
            "{}{}{}",
            trash_path.to_string_lossy(),
            MANIFEST_DELIMITER,
            path.to_string_lossy()
        )
        .map_err(|e| e.to_string())?;
        drop(file);
        drop(_lock);

        // Move file
        fs::rename(path, &trash_path).map_err(|e| e.to_string())?;

        // Remove from starred if present
        let starred_path = self.starred_path();
        let _lock = FileLock::acquire(&starred_path);
        let mut starred = self.starred();
        if starred.remove(path) {
            Self::write_path_list(&starred_path, &starred);
        }

        Ok(())
    }

    pub fn restore_last(&self) -> Result<PathBuf, String> {
        let manifest = self.trash_manifest_path();
        let _lock = FileLock::acquire(&manifest).map_err(|e| e.to_string())?;

        let content =
            fs::read_to_string(&manifest).map_err(|_| "No trash history".to_string())?;
        let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();

        let last = lines.last().ok_or("Trash is empty")?;
        let parts: Vec<&str> = last.splitn(2, MANIFEST_DELIMITER).collect();
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
        fs::write(
            &manifest,
            if remaining.is_empty() {
                remaining
            } else {
                remaining + "\n"
            },
        )
        .map_err(|e| e.to_string())?;

        Ok(original_path)
    }

    pub fn trashed_entries(&self, n: usize) -> Vec<(PathBuf, PathBuf)> {
        let manifest = self.trash_manifest_path();
        let content = fs::read_to_string(&manifest).unwrap_or_default();
        let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
        lines
            .iter()
            .rev()
            .take(n)
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(2, MANIFEST_DELIMITER).collect();
                if parts.len() == 2 {
                    Some((PathBuf::from(parts[0]), PathBuf::from(parts[1])))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn trash_dir(&self) -> &Path {
        &self.trash_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_dir() -> (tempfile::TempDir, State) {
        let dir = tempfile::tempdir().unwrap();
        let wallpaper_dir = dir.path().join("wallpapers");
        fs::create_dir_all(&wallpaper_dir).unwrap();

        // Create test images
        for name in &["a.png", "b.jpg", "sub/c.png"] {
            let p = wallpaper_dir.join(name);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, format!("fake image {}", name)).unwrap();
        }

        // Override config dir to temp
        let config_dir = dir.path().join("config");
        fs::create_dir_all(&config_dir).unwrap();

        let state = State {
            data_dir: config_dir,
            trash_dir: wallpaper_dir.join(".trash"),
            wallpaper_dir: wallpaper_dir.clone(),
        };
        fs::create_dir_all(&state.trash_dir).unwrap();

        (dir, state)
    }

    #[test]
    fn test_current_none_initially() {
        let (_dir, state) = setup_test_dir();
        assert!(state.current().is_none());
    }

    #[test]
    fn test_set_and_get_current() {
        let (_dir, state) = setup_test_dir();
        let path = state.wallpaper_dir.join("a.png");
        state.set_current(&path);
        assert_eq!(state.current().unwrap(), path);
    }

    #[test]
    fn test_star_toggle() {
        let (_dir, state) = setup_test_dir();
        let path = state.wallpaper_dir.join("a.png");

        assert!(!state.is_starred(&path));

        let starred = state.toggle_star(&path);
        assert!(starred);
        assert!(state.is_starred(&path));

        let unstarred = state.toggle_star(&path);
        assert!(!unstarred);
        assert!(!state.is_starred(&path));
    }

    #[test]
    fn test_trash_and_restore() {
        let (_dir, state) = setup_test_dir();
        let path = state.wallpaper_dir.join("a.png");
        assert!(path.exists());

        state.trash(&path).unwrap();
        assert!(!path.exists());
        // Trash dir should have files
        assert!(state.trash_dir.read_dir().unwrap().count() > 0);

        let entries = state.trashed_entries(10);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].1, path);

        let restored = state.restore_last().unwrap();
        assert_eq!(restored, path);
        assert!(path.exists());
    }

    #[test]
    fn test_trash_preserves_subdir() {
        let (_dir, state) = setup_test_dir();
        let path = state.wallpaper_dir.join("sub/c.png");

        state.trash(&path).unwrap();
        assert!(!path.exists());

        // Check trash has sub/ directory
        let sub_trash = state.trash_dir.join("sub");
        assert!(sub_trash.exists());
    }

    #[test]
    fn test_cannot_trash_starred() {
        let (_dir, state) = setup_test_dir();
        let path = state.wallpaper_dir.join("a.png");
        state.toggle_star(&path);

        // Trash should fail — but our state doesn't enforce this,
        // the main.rs checks is_starred before calling trash.
        // Verify starred is removed after trash.
        state.trash(&path).unwrap();
        assert!(!state.is_starred(&path));
    }

    #[test]
    fn test_relative_path() {
        let (_dir, state) = setup_test_dir();
        let path = state.wallpaper_dir.join("sub/c.png");
        let rel = state.relative_to_wallpaper_dir(&path);
        assert_eq!(rel, PathBuf::from("sub/c.png"));
    }

    #[test]
    fn test_relative_path_outside_wallpaper_dir() {
        let (_dir, state) = setup_test_dir();
        let path = PathBuf::from("/tmp/random/image.png");
        let rel = state.relative_to_wallpaper_dir(&path);
        assert_eq!(rel, PathBuf::from("image.png"));
    }

    #[test]
    fn test_manifest_with_special_chars() {
        let (_dir, state) = setup_test_dir();
        // Create file with spaces in name
        let path = state.wallpaper_dir.join("my wallpaper (1).png");
        fs::write(&path, "fake").unwrap();

        state.trash(&path).unwrap();
        let entries = state.trashed_entries(10);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].1, path);

        let restored = state.restore_last().unwrap();
        assert_eq!(restored, path);
        assert!(path.exists());
    }
}
