use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_wallpaper_dir")]
    pub wallpaper_dir: PathBuf,
    #[serde(default = "default_transition_type")]
    pub transition_type: String,
    #[serde(default = "default_transition_duration")]
    pub transition_duration: f32,
    #[serde(default)]
    pub icons: Icons,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Icons {
    #[serde(default = "default_star_active")]
    pub star_active: String,
    #[serde(default = "default_star_inactive")]
    pub star_inactive: String,
    #[serde(default = "default_trash_active")]
    pub trash_active: String,
    #[serde(default = "default_trash_inactive")]
    pub trash_inactive: String,
    #[serde(default = "default_random")]
    pub random: String,
    #[serde(default = "default_starred")]
    pub starred: String,
}

fn default_wallpaper_dir() -> PathBuf {
    dirs::home_dir().unwrap().join("Pictures/Wallpapers")
}
fn default_transition_type() -> String { "fade".into() }
fn default_transition_duration() -> f32 { 1.0 }
fn default_star_active() -> String { "󰓎".into() }
fn default_star_inactive() -> String { "󰓎".into() }
fn default_trash_active() -> String { "󰩹".into() }
fn default_trash_inactive() -> String { "󰩹".into() }
fn default_random() -> String { "󰒟󰋩".into() }
fn default_starred() -> String { "󰒟󰓎".into() }

impl Default for Icons {
    fn default() -> Self {
        Self {
            star_active: default_star_active(),
            star_inactive: default_star_inactive(),
            trash_active: default_trash_active(),
            trash_inactive: default_trash_inactive(),
            random: default_random(),
            starred: default_starred(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            wallpaper_dir: default_wallpaper_dir(),
            transition_type: default_transition_type(),
            transition_duration: default_transition_duration(),
            icons: Icons::default(),
        }
    }
}

fn expand_tilde(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if s.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&s[2..]);
        }
    }
    path.to_path_buf()
}

impl Config {
    pub fn load() -> Self {
        let config_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("wproulette/config.toml");

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path).unwrap_or_default();
            let mut config: Self = toml::from_str(&content).unwrap_or_default();
            config.wallpaper_dir = expand_tilde(&config.wallpaper_dir);
            config
        } else {
            Self::default()
        }
    }
}
