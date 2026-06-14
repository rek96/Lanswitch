//! Load and save the user's presets file. On first run (file missing) we seed
//! it from the embedded defaults so there's immediately something to click.

use std::path::Path;

use crate::types::PresetsFile;

pub fn parse(json: &str) -> serde_json::Result<PresetsFile> {
    serde_json::from_str(json)
}

pub fn to_pretty_json(file: &PresetsFile) -> serde_json::Result<String> {
    serde_json::to_string_pretty(file)
}

/// Load presets from `path`, seeding from `default_json` if the file does not
/// exist yet. The caller supplies the path (platform-specific app-data dir).
pub fn load_or_seed(path: &Path, default_json: &str) -> std::io::Result<PresetsFile> {
    if path.exists() {
        let text = std::fs::read_to_string(path)?;
        match parse(&text) {
            Ok(f) => Ok(f),
            Err(e) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("presets.json is not valid: {e}"),
            )),
        }
    } else {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, default_json)?;
        parse(default_json).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })
    }
}

pub fn save(path: &Path, file: &PresetsFile) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = to_pretty_json(file)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    std::fs::write(path, json)
}
