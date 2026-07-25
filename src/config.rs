use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use toml_edit::DocumentMut;

const FONT_TABLES: [&str; 4] = ["normal", "bold", "italic", "bold_italic"];

/// Resolve the Alacritty config path, following Alacritty's own precedence:
/// $XDG_CONFIG_HOME/alacritty/alacritty.toml, then ~/.config/alacritty/alacritty.toml,
/// then ~/.alacritty.toml.
pub fn resolve_config_path() -> Result<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        let candidate = Path::new(&xdg).join("alacritty/alacritty.toml");
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    let home = dirs::home_dir().context("could not determine home directory")?;

    let candidate = home.join(".config/alacritty/alacritty.toml");
    if candidate.exists() {
        return Ok(candidate);
    }

    let candidate = home.join(".alacritty.toml");
    if candidate.exists() {
        return Ok(candidate);
    }

    anyhow::bail!("could not find an Alacritty config file")
}

pub struct Config {
    path: PathBuf,
    original: String,
    doc: DocumentMut,
}

impl Config {
    pub fn load(path: PathBuf) -> Result<Self> {
        let original =
            fs::read_to_string(&path).with_context(|| format!("failed to read {:?}", path))?;
        let doc: DocumentMut = original
            .parse()
            .with_context(|| format!("failed to parse {:?} as TOML", path))?;

        // One-time safety net backup, independent of in-memory restore.
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let backup_path = path.with_extension(format!("toml.bak-{ts}"));
        fs::write(&backup_path, &original)
            .with_context(|| format!("failed to write backup {:?}", backup_path))?;

        Ok(Self {
            path,
            original,
            doc,
        })
    }

    pub fn live_config_reload_enabled(&self) -> bool {
        self.doc
            .get("general")
            .and_then(|g| g.get("live_config_reload"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    /// Set `family` on whichever font.* tables already exist in the document.
    /// Never creates tables that weren't already present.
    pub fn apply_family(&mut self, family: &str) -> Result<()> {
        let Some(font) = self.doc.get_mut("font").and_then(|f| f.as_table_like_mut()) else {
            anyhow::bail!("config has no [font] section to update");
        };

        for table_name in FONT_TABLES {
            if let Some(table) = font.get_mut(table_name).and_then(|t| t.as_table_like_mut()) {
                table.insert("family", toml_edit::value(family));
            }
        }

        Ok(())
    }

    pub fn write(&self) -> Result<()> {
        fs::write(&self.path, self.doc.to_string())
            .with_context(|| format!("failed to write {:?}", self.path))
    }

    /// Restore the file to its original on-disk content.
    pub fn restore(&self) -> Result<()> {
        fs::write(&self.path, &self.original)
            .with_context(|| format!("failed to restore {:?}", self.path))
    }
}
