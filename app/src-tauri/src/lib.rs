//! The Squirt desktop app: Tauri commands over `squirt_service::Service`,
//! plus a `squirt://` protocol that serves album art to the webview.

mod media;

use squirt_service::{AlbumDto, DeviceStatus, LastFmProfile, PlayerStateDto, Service, SocialStatus};
use std::sync::Arc;
use tauri::http::Response;
use tauri::{Manager, State};

type Svc<'a> = State<'a, Arc<Service>>;

// Commands run on a worker thread (`async`), so slow device reads never
// block the window.

#[tauri::command(async)]
fn collection(svc: Svc) -> Result<Vec<AlbumDto>, String> {
    svc.collection()
}

#[tauri::command(async)]
fn device_status(svc: Svc) -> DeviceStatus {
    svc.device_status()
}

#[tauri::command(async)]
fn device_collection(svc: Svc) -> Result<Vec<AlbumDto>, String> {
    svc.device_collection()
}

#[tauri::command(async)]
fn player_state(svc: Svc) -> PlayerStateDto {
    svc.player_state()
}

#[tauri::command(async)]
fn player_play(svc: Svc, ids: Vec<i64>, start: usize) -> Result<PlayerStateDto, String> {
    svc.play(ids, start)
}

#[tauri::command(async)]
fn player_toggle(svc: Svc) -> Result<PlayerStateDto, String> {
    svc.toggle()
}

#[tauri::command(async)]
fn player_next(svc: Svc) -> Result<PlayerStateDto, String> {
    svc.next()
}

#[tauri::command(async)]
fn player_prev(svc: Svc) -> Result<PlayerStateDto, String> {
    svc.prev()
}

#[tauri::command(async)]
fn player_seek(svc: Svc, ms: u64) -> Result<PlayerStateDto, String> {
    svc.seek(ms)
}

#[tauri::command(async)]
fn player_volume(svc: Svc, v: f32) -> Result<PlayerStateDto, String> {
    svc.volume(v)
}

#[tauri::command(async)]
fn social_status(svc: Svc) -> SocialStatus {
    svc.social_status()
}

#[tauri::command(async)]
fn lastfm_set_keys(svc: Svc, key: String, secret: String) -> Result<SocialStatus, String> {
    svc.lastfm_set_keys(&key, &secret)
}

#[tauri::command(async)]
fn lastfm_begin_auth(svc: Svc) -> Result<String, String> {
    svc.lastfm_begin_auth()
}

#[tauri::command(async)]
fn lastfm_finish_auth(svc: Svc) -> Result<SocialStatus, String> {
    svc.lastfm_finish_auth()
}

#[tauri::command(async)]
fn lastfm_disconnect(svc: Svc) -> Result<SocialStatus, String> {
    svc.lastfm_disconnect()
}

#[tauri::command(async)]
fn lastfm_set_enabled(svc: Svc, on: bool) -> Result<SocialStatus, String> {
    svc.lastfm_set_enabled(on)
}

#[tauri::command(async)]
fn lastfm_profile(svc: Svc) -> Result<LastFmProfile, String> {
    svc.lastfm_profile()
}

#[tauri::command(async)]
fn discord_set_client_id(svc: Svc, id: String) -> Result<SocialStatus, String> {
    svc.discord_set_client_id(&id)
}

#[tauri::command(async)]
fn discord_set_enabled(svc: Svc, on: bool) -> Result<SocialStatus, String> {
    svc.discord_set_enabled(on)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let svc = Arc::new(Service::open().expect("could not open the Squirt library"));
    svc.start_background();
    let art_svc = svc.clone();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(svc)
        .setup(|app| {
            let svc = app.state::<Arc<Service>>().inner().clone();
            media::init(app.handle(), svc);
            Ok(())
        })
        // squirt://localhost/<key>: album art by key (lib/<hash> or dev/<handle>).
        .register_asynchronous_uri_scheme_protocol("squirt", move |_ctx, request, responder| {
            let svc = art_svc.clone();
            let key = percent_decode(request.uri().path().trim_start_matches('/'));
            std::thread::spawn(move || {
                let resp = match svc.art(&key) {
                    Some(bytes) => Response::builder()
                        .header("Content-Type", "image/jpeg")
                        .header("Cache-Control", "max-age=3600")
                        .body(bytes),
                    None => Response::builder().status(404).body(Vec::new()),
                };
                responder.respond(resp.unwrap());
            });
        })
        .invoke_handler(tauri::generate_handler![
            collection,
            device_status,
            device_collection,
            player_state,
            player_play,
            player_toggle,
            player_next,
            player_prev,
            player_seek,
            player_volume,
            social_status,
            lastfm_set_keys,
            lastfm_begin_auth,
            lastfm_finish_auth,
            lastfm_disconnect,
            lastfm_set_enabled,
            lastfm_profile,
            discord_set_client_id,
            discord_set_enabled
        ])
        .run(tauri::generate_context!())
        .expect("error while running Squirt");
}

/// Decodes %XX escapes (convertFileSrc encodes the '/' in art keys).
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
