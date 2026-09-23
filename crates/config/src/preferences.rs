//! Preferências do usuário independentes do documento e do toolkit.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UserPreferences {
    /// Inverte apenas a resposta vertical das ferramentas de manipulação.
    pub invert_vertical_drag: bool,
    /// Cor do destaque de seleção, RGB sRGB de 8 bits.
    pub selection_rgb: [u8; 3],
    /// Espessura visual em pixels lógicos (1..=6).
    pub selection_thickness: f32,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            invert_vertical_drag: false,
            selection_rgb: [233, 106, 0],
            selection_thickness: 2.0,
        }
    }
}

impl UserPreferences {
    pub fn default_path() -> PathBuf {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .unwrap_or_else(std::env::temp_dir);
        base.join("petunia3d").join("preferences.toml")
    }

    pub fn load_from_path(path: &Path) -> io::Result<Self> {
        let source = fs::read_to_string(path)?;
        let mut preferences: Self = toml::from_str(&source)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if !preferences.selection_thickness.is_finite()
            || !(1.0..=6.0).contains(&preferences.selection_thickness)
        {
            preferences.selection_thickness = Self::default().selection_thickness;
        }
        Ok(preferences)
    }

    pub fn load() -> Self {
        Self::load_from_path(&Self::default_path()).unwrap_or_default()
    }

    pub fn save_to_path(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let source = toml::to_string_pretty(self)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let temporary = path.with_extension(format!("toml.{}.tmp", std::process::id()));
        fs::write(&temporary, &source)?;
        match fs::rename(&temporary, path) {
            Ok(()) => Ok(()),
            #[cfg(windows)]
            Err(_) => {
                // Windows não substitui sempre o destino de rename; fallback
                // de melhor esforço para gravações subsequentes.
                let result = fs::write(path, source);
                let _ = fs::remove_file(&temporary);
                result
            }
            #[cfg(not(windows))]
            Err(error) => Err(error),
        }
    }

    pub fn save(&self) -> io::Result<()> {
        self.save_to_path(&Self::default_path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preference_round_trip_and_missing_field_default() {
        let path =
            std::env::temp_dir().join(format!("petunia-preferences-{}.toml", std::process::id()));
        let preferences = UserPreferences {
            invert_vertical_drag: true,
            selection_rgb: [32, 180, 240],
            selection_thickness: 3.5,
        };
        preferences.save_to_path(&path).unwrap();
        assert_eq!(UserPreferences::load_from_path(&path).unwrap(), preferences);
        let updated = UserPreferences {
            selection_thickness: 5.0,
            ..preferences
        };
        updated.save_to_path(&path).unwrap();
        assert_eq!(UserPreferences::load_from_path(&path).unwrap(), updated);
        fs::write(
            &path,
            "selection_thickness = nan\ninvert_vertical_drag = true\n",
        )
        .unwrap();
        let sanitized = UserPreferences::load_from_path(&path).unwrap();
        assert_eq!(sanitized.selection_thickness, 2.0);
        assert!(sanitized.invert_vertical_drag);
        fs::remove_file(&path).unwrap();
        assert_eq!(
            toml::from_str::<UserPreferences>("").unwrap(),
            UserPreferences::default()
        );
    }
}
