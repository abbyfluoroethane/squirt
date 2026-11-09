//! Discord Rich Presence over the local IPC socket: "Listening to Squirt",
//! with the song, artist, album, cover art and a progress bar.

use discord_rich_presence::activity::{Activity, ActivityType, Assets, StatusDisplayType, Timestamps};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};

pub struct Discord {
    client_id: String,
    client: Option<DiscordIpcClient>,
}

/// What to show.
pub struct Presence<'a> {
    pub title: &'a str,
    pub artist: &'a str,
    pub album: &'a str,
    /// Unix ms when the track (notionally) started and will end; drives
    /// Discord's progress bar. `None` while paused.
    pub span_ms: Option<(i64, i64)>,
    /// A web URL for the cover art; falls back to the app's "squirt" asset.
    pub image_url: Option<&'a str>,
}

impl Discord {
    pub fn new(client_id: &str) -> Self {
        Self { client_id: client_id.into(), client: None }
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    pub fn connected(&self) -> bool {
        self.client.is_some()
    }

    fn ensure(&mut self) -> Result<&mut DiscordIpcClient, String> {
        if self.client.is_none() {
            let mut c = DiscordIpcClient::new(&self.client_id);
            c.connect().map_err(|e| format!("could not reach Discord: {e}"))?;
            self.client = Some(c);
        }
        Ok(self.client.as_mut().unwrap())
    }

    pub fn set(&mut self, p: &Presence) -> Result<(), String> {
        // Discord needs these fields to be at least 2 characters.
        let pad = |s: &str| if s.chars().count() < 2 { format!("{s}  ") } else { s.to_string() };
        let (title, artist, album) = (pad(p.title), pad(p.artist), pad(p.album));
        let mut assets = Assets::new().large_text(&album);
        assets = match p.image_url {
            Some(url) => assets.large_image(url),
            None => assets.large_image("squirt"),
        };
        let mut activity = Activity::new()
            .activity_type(ActivityType::Listening)
            .status_display_type(StatusDisplayType::Details)
            .details(&title)
            .state(&artist)
            .assets(assets);
        if let Some((start, end)) = p.span_ms {
            activity = activity.timestamps(Timestamps::new().start(start).end(end));
        }
        let result = self.ensure()?.set_activity(activity);
        if let Err(e) = result {
            // Discord was closed or restarted: reconnect on the next update.
            self.client = None;
            return Err(format!("Discord update failed: {e}"));
        }
        Ok(())
    }

    pub fn clear(&mut self) {
        if let Some(c) = self.client.as_mut() {
            if c.clear_activity().is_err() {
                self.client = None;
            }
        }
    }

    pub fn disconnect(&mut self) {
        if let Some(mut c) = self.client.take() {
            let _ = c.close();
        }
    }
}
