use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_wallpaper_dir")]
    pub wallpaper_dir: PathBuf,
    #[serde(default = "default_transition_type")]
    pub transition_type: String,
    #[serde(default = "default_transition_duration")]
    pub transition_duration: f32,
    #[serde(default = "default_badge")]
    pub badge: String,
}

fn default_wallpaper_dir() -> PathBuf {
    dirs::home_dir().unwrap().join("Pictures/Wallpapers")
}
fn default_transition_type() -> String { "fade".into() }
fn default_transition_duration() -> f32 { 1.0 }
fn default_badge() -> String { "●".into() }

impl Default for Config {
    fn default() -> Self {
        Self {
            wallpaper_dir: default_wallpaper_dir(),
            transition_type: default_transition_type(),
            transition_duration: default_transition_duration(),
            badge: default_badge(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("wproulette/config.toml");

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path).unwrap_or_default();
            toml::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }
}
