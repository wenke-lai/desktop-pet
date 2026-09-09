use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::Path};
use crate::geometry::Point;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Settings {
    pub active_pet: Option<String>,
    pub scale: f64,
    pub always_on_top: bool,
    /// Desktop anchor in physical pixels, including negative monitor coordinates.
    pub last_position: Option<Point>,
    pub alpha_threshold: u8,
}
impl Default for Settings {
    fn default() -> Self { Self { active_pet: None, scale: 1.0, always_on_top: true, last_position: None, alpha_threshold: 20 } }
}
impl Settings {
    pub fn load(path: &Path) -> Self {
        match fs::read(path) {
            Ok(bytes) => match serde_json::from_slice::<Self>(&bytes) {
                Ok(settings) if settings.scale.is_finite() && settings.scale > 0.0 && settings.scale <= 10.0 && settings.last_position.is_none_or(|p| p.x.is_finite() && p.y.is_finite()) => settings,
                Ok(_) => { log::warn!("Invalid settings values; using defaults"); Self::default() },
                Err(e) => { log::warn!("Invalid settings.json: {e}; using defaults"); Self::default() }
            },
            Err(e) => { log::info!("Using default settings: {e}"); Self::default() }
        }
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let parent = path.parent().ok_or("settings path has no parent")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        temp.write_all(&bytes).map_err(|e| e.to_string())?;
        temp.as_file().sync_all().map_err(|e| e.to_string())?;
        temp.persist(path).map_err(|e| e.to_string())?;
        Ok(())
    }
}
