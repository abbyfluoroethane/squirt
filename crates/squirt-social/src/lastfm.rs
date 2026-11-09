//! A small Last.fm API client: desktop authorization, now playing, scrobbling,
//! and the profile/recent-tracks reads the Social tab shows.
//! API docs: https://www.last.fm/api

use md5::{Digest, Md5};
use serde_json::Value;
use std::collections::BTreeMap;

const API: &str = "https://ws.audioscrobbler.com/2.0/";
const USER_AGENT: &str = "Squirt/0.1 (+https://github.com/abbyfluoroethane/squirt)";

pub type Result<T> = std::result::Result<T, String>;

pub struct LastFm {
    pub api_key: String,
    pub api_secret: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentTrack {
    pub artist: String,
    pub title: String,
    pub album: String,
    pub image: Option<String>,
    pub now_playing: bool,
    /// Unix seconds; absent while now playing.
    pub played_at: Option<i64>,
    pub url: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    pub name: String,
    pub playcount: u64,
    pub image: Option<String>,
    pub url: String,
    pub registered: Option<i64>,
}

/// One track to report: what now-playing and scrobble calls need.
pub struct TrackInfo<'a> {
    pub artist: &'a str,
    pub title: &'a str,
    pub album: &'a str,
    pub album_artist: &'a str,
    pub duration_s: u64,
}

/// Last.fm's signature: md5 of the sorted key+value pairs plus the secret.
/// `format` and `callback` are excluded.
fn sign(params: &BTreeMap<&str, String>, secret: &str) -> String {
    let mut s = String::new();
    for (k, v) in params {
        if *k != "format" && *k != "callback" {
            s.push_str(k);
            s.push_str(v);
        }
    }
    s.push_str(secret);
    hex::encode(Md5::digest(s.as_bytes()))
}

/// The largest image URL in a Last.fm `image` array, if any is non-empty.
fn best_image(v: &Value) -> Option<String> {
    v.as_array()?
        .iter()
        .rev()
        .filter_map(|i| i.get("#text")?.as_str())
        .find(|s| !s.is_empty())
        .map(str::to_owned)
}

fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.get("#text").and_then(Value::as_str).unwrap_or_default().to_string(),
    }
}

impl LastFm {
    pub fn new(api_key: &str, api_secret: &str) -> Self {
        Self { api_key: api_key.into(), api_secret: api_secret.into() }
    }

    /// Calls a Last.fm method. Temporary failures (HTTP 5xx, or Last.fm errors
    /// 8 "operation failed", 11/16 "service unavailable", 29 "rate limit") are
    /// retried twice with a short pause; Last.fm returns these often and they
    /// usually clear within seconds. Errors name the method and carry
    /// Last.fm's own message.
    fn call(&self, method: &str, extra: &[(&str, &str)], signed: bool, post: bool) -> Result<Value> {
        let mut params: BTreeMap<&str, String> = BTreeMap::new();
        params.insert("method", method.into());
        params.insert("api_key", self.api_key.clone());
        for (k, v) in extra {
            params.insert(k, v.to_string());
        }
        if signed {
            let sig = sign(&params, &self.api_secret);
            params.insert("api_sig", sig);
        }
        params.insert("format", "json".into());

        let mut last_err = String::new();
        for attempt in 0..3 {
            if attempt > 0 {
                std::thread::sleep(std::time::Duration::from_millis(800 * attempt));
            }
            match self.send(&params, post) {
                Ok(v) => return Ok(v),
                Err((e, temporary)) => {
                    last_err = format!("{method}: {e}");
                    if !temporary {
                        break;
                    }
                }
            }
        }
        Err(last_err)
    }

    /// One request. On failure, returns the message and whether it is worth retrying.
    fn send(&self, params: &BTreeMap<&str, String>, post: bool) -> std::result::Result<Value, (String, bool)> {
        // Read error responses too: Last.fm puts the real reason in the JSON body.
        let agent: ureq::Agent = ureq::Agent::config_builder().http_status_as_error(false).build().into();
        let resp = if post {
            agent.post(API).header("User-Agent", USER_AGENT).send_form(params.iter().map(|(k, v)| (*k, v.as_str())))
        } else {
            let mut req = agent.get(API).header("User-Agent", USER_AGENT);
            for (k, v) in params {
                req = req.query(*k, v);
            }
            req.call()
        };
        let mut resp = resp.map_err(|e| (format!("could not reach last.fm: {e}"), true))?;
        let status = resp.status().as_u16();
        let body: Option<Value> = resp.body_mut().read_json().ok();
        if let Some(code) = body.as_ref().and_then(|b| b.get("error")).and_then(Value::as_u64) {
            let msg = body.as_ref().and_then(|b| b.get("message")).and_then(Value::as_str).unwrap_or("unknown error");
            let temporary = matches!(code, 8 | 11 | 16 | 29);
            return Err((format!("last.fm error {code}: {msg}"), temporary));
        }
        match body {
            Some(v) if status < 400 => Ok(v),
            _ => Err((format!("last.fm returned HTTP {status}"), status >= 500)),
        }
    }

    // ---------------------------------------------------------------- auth

    /// Step 1 of desktop auth: a request token.
    pub fn get_token(&self) -> Result<String> {
        let v = self.call("auth.getToken", &[], true, false)?;
        v["token"].as_str().map(str::to_owned).ok_or_else(|| "no token in response".into())
    }

    /// Step 2: the page where the user approves Squirt.
    pub fn auth_url(&self, token: &str) -> String {
        format!("https://www.last.fm/api/auth/?api_key={}&token={}", self.api_key, token)
    }

    /// Step 3, after approval: a session key and the username.
    pub fn get_session(&self, token: &str) -> Result<(String, String)> {
        let v = self.call("auth.getSession", &[("token", token)], true, false)?;
        let s = &v["session"];
        match (s["key"].as_str(), s["name"].as_str()) {
            (Some(k), Some(n)) => Ok((k.into(), n.into())),
            _ => Err("no session in response".into()),
        }
    }

    // ----------------------------------------------------------- scrobbling

    fn track_params<'a>(t: &'a TrackInfo<'a>, dur: &'a str) -> Vec<(&'static str, &'a str)> {
        let mut p = vec![("artist", t.artist), ("track", t.title)];
        if !t.album.is_empty() {
            p.push(("album", t.album));
        }
        if !t.album_artist.is_empty() && t.album_artist != t.artist {
            p.push(("albumArtist", t.album_artist));
        }
        if t.duration_s > 0 {
            p.push(("duration", dur));
        }
        p
    }

    pub fn update_now_playing(&self, sk: &str, t: &TrackInfo) -> Result<()> {
        let dur = t.duration_s.to_string();
        let mut p = Self::track_params(t, &dur);
        p.push(("sk", sk));
        self.call("track.updateNowPlaying", &p, true, true).map(|_| ())
    }

    /// Scrobbles a play that started at `started_at` (unix seconds).
    pub fn scrobble(&self, sk: &str, t: &TrackInfo, started_at: i64) -> Result<()> {
        let dur = t.duration_s.to_string();
        let ts = started_at.to_string();
        let mut p = Self::track_params(t, &dur);
        p.push(("timestamp", &ts));
        p.push(("sk", sk));
        let v = self.call("track.scrobble", &p, true, true)?;
        let ignored = v["scrobbles"]["@attr"]["ignored"].as_u64().unwrap_or(0);
        if ignored > 0 {
            let why = text(&v["scrobbles"]["scrobble"]["ignoredMessage"]);
            return Err(format!("last.fm ignored the scrobble: {why}"));
        }
        Ok(())
    }

    // ---------------------------------------------------------------- reads

    pub fn user_info(&self, user: &str) -> Result<UserInfo> {
        let v = self.call("user.getInfo", &[("user", user)], false, false)?;
        let u = &v["user"];
        Ok(UserInfo {
            name: text(&u["name"]),
            playcount: text(&u["playcount"]).parse().unwrap_or(0),
            image: best_image(&u["image"]),
            url: text(&u["url"]),
            registered: u["registered"]["unixtime"].as_str().and_then(|s| s.parse().ok()),
        })
    }

    pub fn recent_tracks(&self, user: &str, limit: u32) -> Result<Vec<RecentTrack>> {
        let limit = limit.to_string();
        let v = self.call("user.getRecentTracks", &[("user", user), ("limit", &limit)], false, false)?;
        let items = match &v["recenttracks"]["track"] {
            Value::Array(a) => a.clone(),
            one @ Value::Object(_) => vec![one.clone()],
            _ => Vec::new(),
        };
        Ok(items
            .iter()
            .map(|t| RecentTrack {
                artist: text(&t["artist"]),
                title: text(&t["name"]),
                album: text(&t["album"]),
                image: best_image(&t["image"]),
                now_playing: t["@attr"]["nowplaying"].as_str() == Some("true"),
                played_at: t["date"]["uts"].as_str().and_then(|s| s.parse().ok()),
                url: text(&t["url"]),
            })
            .collect())
    }

    /// Cover art URL for an album, for Discord (which needs a web URL).
    pub fn album_image(&self, artist: &str, album: &str) -> Option<String> {
        let v = self.call("album.getInfo", &[("artist", artist), ("album", album)], false, false).ok()?;
        best_image(&v["album"]["image"])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_sorts_params_and_skips_format() {
        let mut p = BTreeMap::new();
        p.insert("method", "auth.getSession".to_string());
        p.insert("api_key", "KEY".to_string());
        p.insert("token", "TOK".to_string());
        p.insert("format", "json".to_string());
        // md5("api_keyKEYmethodauth.getSessiontokenTOKSECRET")
        let expected = hex::encode(Md5::digest(b"api_keyKEYmethodauth.getSessiontokenTOKSECRET"));
        assert_eq!(sign(&p, "SECRET"), expected);
    }

    #[test]
    fn picks_largest_nonempty_image() {
        let v: Value = serde_json::json!([
            { "#text": "s.jpg", "size": "small" },
            { "#text": "xl.jpg", "size": "extralarge" },
            { "#text": "", "size": "mega" }
        ]);
        assert_eq!(best_image(&v).as_deref(), Some("xl.jpg"));
    }
}
