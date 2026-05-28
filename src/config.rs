use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub extension_map: HashMap<String, String>,
    pub sort_targets: HashMap<String, PathBuf>,
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut extension_map = HashMap::new();
        let categories = vec![
            (
                "Images",
                vec![
                    "jpg", "jpeg", "png", "gif", "bmp", "svg", "webp", "tiff", "ico",
                ],
            ),
            (
                "Documents",
                vec![
                    "pdf", "docx", "doc", "txt", "md", "odt", "rtf", "xlsx", "csv", "pptx", "ppt",
                ],
            ),
            (
                "Videos",
                vec!["mp4", "mkv", "avi", "mov", "wmv", "flv", "webm"],
            ),
            (
                "Audio",
                vec!["mp3", "wav", "flac", "aac", "ogg", "wma", "m4a"],
            ),
            (
                "Archives",
                vec!["zip", "tar", "gz", "rar", "7z", "bz2", "xz"],
            ),
            (
                "Code",
                vec![
                    "rs", "py", "js", "ts", "c", "cpp", "h", "java", "go", "rb", "php", "sh",
                    "html", "css",
                ],
            ),
            ("Installers", vec!["exe", "msi", "deb", "rpm", "dmg", "apk"]),
        ];

        for (category, exts) in categories {
            for ext in exts {
                extension_map.insert(ext.to_string(), category.to_string());
            }
        }
        let mut sort_targets = HashMap::new();
        // Remove the home directory mapping and use clean relative paths
        for category in [
            "Images",
            "Documents",
            "Videos",
            "Audio",
            "Archives",
            "Code",
            "Installers",
            "Others",
        ] {
            sort_targets.insert(category.to_string(), PathBuf::from(category));
        }

        Self {
            extension_map,
            sort_targets,
        }
    }
}

impl AppConfig {
    // Helper to always point to the user's global home directory
    fn get_global_path() -> std::path::PathBuf {
        let home = std::env::var("USERPROFILE") // Windows
            .or_else(|_| std::env::var("HOME")) // Linux / Mac
            .unwrap_or_else(|_| ".".to_string()); // Fallback

        std::path::PathBuf::from(home).join(".atrus-config.json")
    }

    pub fn load() -> anyhow::Result<Self> {
        let config_path = Self::get_global_path();
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            let default_config = Self::default();
            let content = serde_json::to_string_pretty(&default_config)?;
            std::fs::write(&config_path, content)?;
            Ok(default_config)
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let config_path = Self::get_global_path();
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }
}
