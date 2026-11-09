//! Last.fm and Discord, driven by what the player is doing.
//!
//! A background loop (see [`Service::start_background`]) checks the player
//! once a second. When a track starts it sends Last.fm "now playing" and sets
//! the Discord presence. It counts time actually listened (position advancing
//! while playing, not seeks) and scrobbles once a track longer than 30 s has
//! been heard for half its length or 4 minutes, whichever comes first, per
//! Last.fm's rules. Scrobbles that fail (offline) are retried.

use crate::{err, Result, Service};
use serde::Serialize;
use squirt_social::discord::{Discord, Presence};
use squirt_social::lastfm::{LastFm, RecentTrack, TrackInfo, UserInfo};
use squirt_social::{builtin, SocialConfig};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn now_s() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrobbleDto {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub at: i64,
    /// None if it went through; the error if it is waiting to be retried.
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastFmStatus {
    /// False when this build has no Last.fm secret compiled in.
    pub available: bool,
    pub connected: bool,
    pub username: String,
    pub enabled: bool,
    pub auth_pending: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscordStatus {
    pub enabled: bool,
    pub connected: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SocialStatus {
    pub lastfm: LastFmStatus,
    pub discord: DiscordStatus,
    /// This session's scrobbles, newest first.
    pub scrobbles: Vec<ScrobbleDto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LastFmProfile {
    pub user: UserInfo,
    pub recent: Vec<RecentTrack>,
}

/// A play waiting to be (re)sent to Last.fm.
#[derive(Clone)]
struct Pending {
    title: String,
    artist: String,
    album: String,
    album_artist: String,
    duration_s: u64,
    started_at: i64,
}

pub(crate) struct SocialState {
    path: PathBuf,
    config: SocialConfig,
    auth_token: Option<String>,
    lastfm_error: Option<String>,
    discord: Option<Discord>,
    discord_error: Option<String>,
    scrobbles: VecDeque<ScrobbleDto>,
    retry: Vec<Pending>,
    art: HashMap<(String, String), Option<String>>,
    /// Last profile fetched and when; reused for 30 s (cleared on a scrobble).
    profile: Option<(Instant, LastFmProfile)>,
}

impl SocialState {
    pub(crate) fn load(path: PathBuf) -> Self {
        let config = SocialConfig::load(&path);
        Self {
            path,
            config,
            auth_token: None,
            lastfm_error: None,
            discord: None,
            discord_error: None,
            scrobbles: VecDeque::new(),
            retry: Vec::new(),
            art: HashMap::new(),
            profile: None,
        }
    }

    fn save(&self) -> Result<()> {
        self.config.save(&self.path).map_err(err)
    }

    fn lastfm(&self) -> Option<LastFm> {
        builtin::LASTFM_API_SECRET.map(|secret| LastFm::new(builtin::LASTFM_API_KEY, secret))
    }

    /// Last.fm client and session key, when connected and enabled.
    fn scrobbler(&self) -> Option<(LastFm, String)> {
        let c = &self.config.lastfm;
        if c.enabled && !c.session_key.is_empty() {
            self.lastfm().map(|l| (l, c.session_key.clone()))
        } else {
            None
        }
    }

    fn record(&mut self, p: &Pending, error: Option<String>) {
        self.scrobbles.push_front(ScrobbleDto {
            title: p.title.clone(),
            artist: p.artist.clone(),
            album: p.album.clone(),
            at: p.started_at,
            error,
        });
        self.scrobbles.truncate(50);
    }
}

/// What the background loop knows about the current listen.
struct Listen {
    track_id: Option<i64>,
    meta: Option<Pending>,
    listened_ms: u64,
    last_pos: u64,
    last_tick: Instant,
    scrobbled: bool,
    status: &'static str,
}

impl Service {
    // ------------------------------------------------------------ settings

    pub fn social_status(&self) -> SocialStatus {
        let s = self.social.lock().unwrap();
        let c = &s.config;
        SocialStatus {
            lastfm: LastFmStatus {
                available: builtin::LASTFM_API_SECRET.is_some(),
                connected: !c.lastfm.session_key.is_empty(),
                username: c.lastfm.username.clone(),
                enabled: c.lastfm.enabled,
                auth_pending: s.auth_token.is_some(),
                error: s.lastfm_error.clone(),
            },
            discord: DiscordStatus {
                enabled: c.discord.enabled,
                connected: s.discord.as_ref().is_some_and(Discord::connected),
                error: s.discord_error.clone(),
            },
            scrobbles: s.scrobbles.iter().cloned().collect(),
        }
    }

    /// Starts authorization: returns the last.fm page to open in a browser.
    pub fn lastfm_begin_auth(&self) -> Result<String> {
        let lf = self.social.lock().unwrap().lastfm().ok_or("Last.fm isn't available in this build of Squirt")?;
        let token = lf.get_token()?;
        let url = lf.auth_url(&token);
        self.social.lock().unwrap().auth_token = Some(token);
        Ok(url)
    }

    /// Finishes authorization after the user approved Squirt on last.fm.
    pub fn lastfm_finish_auth(&self) -> Result<SocialStatus> {
        let (lf, token) = {
            let s = self.social.lock().unwrap();
            (s.lastfm().ok_or("no Last.fm API key")?, s.auth_token.clone().ok_or("authorization was not started")?)
        };
        let result = lf.get_session(&token);
        {
            let mut s = self.social.lock().unwrap();
            match result {
                Ok((key, name)) => {
                    s.config.lastfm.session_key = key;
                    s.config.lastfm.username = name;
                    s.config.lastfm.enabled = true;
                    s.auth_token = None;
                    s.lastfm_error = None;
                    s.save()?;
                }
                Err(e) => {
                    s.lastfm_error = Some(e.clone());
                    return Err(e);
                }
            }
        }
        Ok(self.social_status())
    }

    pub fn lastfm_disconnect(&self) -> Result<SocialStatus> {
        {
            let mut s = self.social.lock().unwrap();
            s.config.lastfm.session_key.clear();
            s.config.lastfm.username.clear();
            s.auth_token = None;
            s.profile = None;
            s.save()?;
        }
        Ok(self.social_status())
    }

    pub fn lastfm_set_enabled(&self, on: bool) -> Result<SocialStatus> {
        {
            let mut s = self.social.lock().unwrap();
            s.config.lastfm.enabled = on;
            s.save()?;
        }
        Ok(self.social_status())
    }

    pub fn discord_set_enabled(&self, on: bool) -> Result<SocialStatus> {
        {
            let mut s = self.social.lock().unwrap();
            s.config.discord.enabled = on;
            if !on {
                if let Some(d) = s.discord.as_mut() {
                    d.clear();
                    d.disconnect();
                }
                s.discord = None;
                s.discord_error = None;
            }
            s.save()?;
        }
        Ok(self.social_status())
    }

    /// The signed-in user's Last.fm profile and recent tracks. Cached for 30 s
    /// so reopening the Social tab doesn't hit Last.fm each time.
    pub fn lastfm_profile(&self) -> Result<LastFmProfile> {
        let (lf, user) = {
            let s = self.social.lock().unwrap();
            if let Some((at, p)) = &s.profile {
                if at.elapsed() < Duration::from_secs(30) && p.user.name.eq_ignore_ascii_case(&s.config.lastfm.username) {
                    return Ok(p.clone());
                }
            }
            (s.lastfm().ok_or("Last.fm is not set up")?, s.config.lastfm.username.clone())
        };
        if user.is_empty() {
            return Err("not signed in to Last.fm".into());
        }
        let p = LastFmProfile { user: lf.user_info(&user)?, recent: lf.recent_tracks(&user, 30)? };
        self.social.lock().unwrap().profile = Some((Instant::now(), p.clone()));
        Ok(p)
    }

    // -------------------------------------------------------- background

    /// Starts the background loop that drives Last.fm and Discord.
    pub fn start_background(self: &Arc<Self>) {
        let svc = self.clone();
        std::thread::Builder::new()
            .name("squirt-social".into())
            .spawn(move || svc.social_loop())
            .expect("spawn social thread");
    }

    fn social_loop(&self) {
        let mut l = Listen {
            track_id: None,
            meta: None,
            listened_ms: 0,
            last_pos: 0,
            last_tick: Instant::now(),
            scrobbled: false,
            status: "stopped",
        };
        let mut last_retry = Instant::now();
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let st = self.player_state();
            let elapsed = l.last_tick.elapsed().as_millis() as u64;
            l.last_tick = Instant::now();

            if st.track_id != l.track_id {
                l.track_id = st.track_id;
                l.listened_ms = 0;
                l.scrobbled = false;
                l.last_pos = st.position_ms;
                l.status = st.status;
                l.meta = st.track_id.and_then(|id| self.now_playing(id)).map(|np| Pending {
                    title: np.title,
                    artist: np.artist,
                    album: np.album,
                    album_artist: np.album_artist,
                    duration_s: np.duration_ms / 1000,
                    started_at: now_s() - (st.position_ms / 1000) as i64,
                });
                if let Some(m) = l.meta.clone() {
                    self.send_now_playing(&m);
                }
                self.update_discord(&l, st.position_ms, st.duration_ms);
                continue;
            }

            let advanced = st.position_ms.saturating_sub(l.last_pos);
            let seeked = st.position_ms < l.last_pos || advanced > elapsed + 2000;
            if st.status == "playing" && !seeked {
                l.listened_ms += advanced;
            }
            l.last_pos = st.position_ms;
            if st.status != l.status || seeked {
                l.status = st.status;
                self.update_discord(&l, st.position_ms, st.duration_ms);
            }

            // Scrobble: track over 30 s, heard for half its length or 4 min.
            if let Some(m) = l.meta.clone() {
                let threshold = (m.duration_s * 1000 / 2).min(240_000);
                if !l.scrobbled && m.duration_s > 30 && l.listened_ms >= threshold {
                    l.scrobbled = true;
                    self.scrobble(m);
                }
            }

            if last_retry.elapsed() > Duration::from_secs(60) {
                last_retry = Instant::now();
                self.retry_scrobbles();
            }
        }
    }

    fn send_now_playing(&self, m: &Pending) {
        let Some((lf, sk)) = self.social.lock().unwrap().scrobbler() else { return };
        let t = TrackInfo { artist: &m.artist, title: &m.title, album: &m.album, album_artist: &m.album_artist, duration_s: m.duration_s };
        let result = lf.update_now_playing(&sk, &t);
        self.social.lock().unwrap().lastfm_error = result.err();
    }

    fn scrobble(&self, m: Pending) {
        let Some((lf, sk)) = self.social.lock().unwrap().scrobbler() else { return };
        let t = TrackInfo { artist: &m.artist, title: &m.title, album: &m.album, album_artist: &m.album_artist, duration_s: m.duration_s };
        let result = lf.scrobble(&sk, &t, m.started_at);
        let mut s = self.social.lock().unwrap();
        match result {
            Ok(()) => {
                s.lastfm_error = None;
                s.profile = None; // Recent tracks changed.
                s.record(&m, None);
            }
            Err(e) => {
                s.lastfm_error = Some(e.clone());
                s.record(&m, Some(e));
                s.retry.push(m);
            }
        }
    }

    fn retry_scrobbles(&self) {
        let (pending, client) = {
            let mut s = self.social.lock().unwrap();
            (std::mem::take(&mut s.retry), s.scrobbler())
        };
        let Some((lf, sk)) = client else {
            self.social.lock().unwrap().retry = pending;
            return;
        };
        let mut still = Vec::new();
        for m in pending {
            let t = TrackInfo { artist: &m.artist, title: &m.title, album: &m.album, album_artist: &m.album_artist, duration_s: m.duration_s };
            if lf.scrobble(&sk, &t, m.started_at).is_err() {
                still.push(m);
            } else {
                let mut s = self.social.lock().unwrap();
                if let Some(rec) = s.scrobbles.iter_mut().find(|r| r.at == m.started_at && r.title == m.title) {
                    rec.error = None;
                }
            }
        }
        self.social.lock().unwrap().retry.extend(still);
    }

    /// Sets (or clears) the Discord presence for the current listen.
    fn update_discord(&self, l: &Listen, position_ms: u64, duration_ms: u64) {
        let (enabled, lf) = {
            let s = self.social.lock().unwrap();
            (s.config.discord.enabled, s.lastfm())
        };
        if !enabled {
            return;
        }
        let client_id = builtin::DISCORD_CLIENT_ID;
        // Cover art must be a web URL; Last.fm has one for most albums.
        let image = match (&l.meta, &lf) {
            (Some(m), Some(lf)) if l.status == "playing" => {
                let key = (m.album_artist.clone(), m.album.clone());
                let cached = self.social.lock().unwrap().art.get(&key).cloned();
                match cached {
                    Some(v) => v,
                    None => {
                        let v = lf.album_image(&m.album_artist, &m.album);
                        self.social.lock().unwrap().art.insert(key, v.clone());
                        v
                    }
                }
            }
            _ => None,
        };

        let mut s = self.social.lock().unwrap();
        if s.discord.as_ref().is_none_or(|d| d.client_id() != client_id) {
            s.discord = Some(Discord::new(client_id));
        }
        let d = s.discord.as_mut().unwrap();
        let result = match (&l.meta, l.status) {
            (Some(m), "playing") => {
                let start = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0) - position_ms as i64;
                d.set(&Presence {
                    title: &m.title,
                    artist: &m.artist,
                    album: &m.album,
                    span_ms: Some((start, start + duration_ms as i64)),
                    image_url: image.as_deref(),
                })
            }
            // Paused or stopped: show nothing, as most players do.
            _ => {
                d.clear();
                Ok(())
            }
        };
        s.discord_error = result.err();
    }
}
