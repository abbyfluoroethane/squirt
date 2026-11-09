//! Social settings, stored as JSON next to the library database. The file
//! holds the Last.fm session key, so it is written readable by the user only.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LastFmConfig {
    /// Set after the user authorizes Squirt on last.fm.
    pub session_key: String,
    pub username: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DiscordConfig {
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SocialConfig {
    pub lastfm: LastFmConfig,
    pub discord: DiscordConfig,
}

impl SocialConfig {
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp: PathBuf = path.with_extension("tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
        }
        std::fs::rename(tmp, path)
    }
}
