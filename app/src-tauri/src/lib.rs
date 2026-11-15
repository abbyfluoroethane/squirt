//! The Squirt desktop app: Tauri commands over `squirt_service::Service`,
//! plus a `squirt://` protocol that serves album art to the webview.

mod media;
#[cfg(target_os = "macos")]
mod menu;

use squirt_service::{SyncStatus, SetupStatus, SuggestedFolder, AlbumDto, DeviceInfoDto, DeviceStatus, FolderDto, PictureAlbumDto, ScanStatus, VideoDto, LastFmProfile, PlayerStateDto, Service, SocialStatus};
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
fn folders(svc: Svc) -> Result<Vec<FolderDto>, String> {
    svc.folders()
}

#[tauri::command(async)]
fn add_folder(svc: Svc, path: String, kind: String) -> Result<Vec<FolderDto>, String> {
    svc.inner().add_folder(&path, &kind)
}

#[tauri::command(async)]
fn remove_folder(svc: Svc, id: i64) -> Result<Vec<FolderDto>, String> {
    svc.remove_folder(id)
}

#[tauri::command(async)]
fn scan_status(svc: Svc) -> ScanStatus {
    svc.scan_status()
}

#[tauri::command(async)]
fn rescan(svc: Svc) -> ScanStatus {
    svc.inner().rescan();
    svc.scan_status()
}

#[tauri::command(async)]
fn videos(svc: Svc) -> Result<Vec<VideoDto>, String> {
    svc.videos()
}

#[tauri::command(async)]
fn pictures(svc: Svc) -> Result<Vec<PictureAlbumDto>, String> {
    svc.pictures()
}

#[tauri::command(async)]
fn podcasts(svc: Svc) -> Result<Vec<AlbumDto>, String> {
    svc.podcasts()
}

#[tauri::command(async)]
fn device_info(svc: Svc) -> DeviceInfoDto {
    svc.device_info()
}

#[tauri::command(async)]
fn device_rename(svc: Svc, name: String) -> Result<DeviceInfoDto, String> {
    svc.device_rename(&name)
}

#[tauri::command(async)]
fn device_refresh(svc: Svc) -> Result<DeviceInfoDto, String> {
    svc.device_refresh()
}

#[tauri::command(async)]
fn device_release(svc: Svc) -> DeviceInfoDto {
    svc.device_release()
}

#[tauri::command(async)]
fn device_connect(svc: Svc) -> DeviceInfoDto {
    svc.device_connect()
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
fn player_spectrum(svc: Svc, bands: usize) -> Vec<f32> {
    svc.player_spectrum(bands)
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
fn discord_set_enabled(svc: Svc, on: bool) -> Result<SocialStatus, String> {
    svc.discord_set_enabled(on)
}

#[tauri::command(async)]
fn sync_tracks(svc: Svc, ids: Vec<i64>) -> Result<SyncStatus, String> {
    svc.inner().sync_tracks(ids)
}

#[tauri::command(async)]
fn device_remove(svc: Svc, ids: Vec<i64>) -> Result<usize, String> {
    svc.device_remove(ids)
}

#[tauri::command(async)]
fn sync_status(svc: Svc) -> SyncStatus {
    svc.sync_status()
}

#[tauri::command(async)]
fn setup_status(svc: Svc) -> SetupStatus {
    svc.setup_status()
}

#[tauri::command(async)]
fn setup_suggestions(svc: Svc) -> Vec<SuggestedFolder> {
    svc.setup_suggestions()
}

#[tauri::command(async)]
fn import_keys(svc: Svc, path: String) -> Result<SetupStatus, String> {
    svc.import_keys(&path)
}

#[tauri::command(async)]
fn finish_setup(svc: Svc) -> Result<SetupStatus, String> {
    svc.finish_setup()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let svc = Arc::new(Service::open().expect("could not open the Squirt library"));
    svc.start_background();
    let art_svc = svc.clone();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(svc)
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                let m = menu::build(app.handle())?;
                app.set_menu(m)?;
                app.on_menu_event(|app, event| menu::forward(app, event.id().as_ref()));
            }
            let svc = app.state::<Arc<Service>>().inner().clone();
            media::init(app.handle(), svc);
            // The window starts hidden and the page shows it as soon as its
            // splash is in place (app.html). If the page never does (a slow
            // dev server), show it anyway.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(3));
                if let Some(w) = handle.get_webview_window("main").filter(|w| !w.is_visible().unwrap_or(true)) {
                    let _ = w.show();
                }
            });
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
                        // Lets the page read the pixels (for colors from the cover).
                        .header("Access-Control-Allow-Origin", "*")
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
            device_info,
            folders,
            add_folder,
            remove_folder,
            scan_status,
            rescan,
            videos,
            pictures,
            podcasts,
            device_rename,
            device_refresh,
            device_release,
            device_connect,
            player_state,
            player_spectrum,
            player_play,
            player_toggle,
            player_next,
            player_prev,
            player_seek,
            player_volume,
            social_status,
            lastfm_begin_auth,
            lastfm_finish_auth,
            lastfm_disconnect,
            lastfm_set_enabled,
            lastfm_profile,
            discord_set_enabled,
            setup_status,
            sync_tracks,
            sync_status,
            device_remove,
            setup_suggestions,
            import_keys,
            finish_setup
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
