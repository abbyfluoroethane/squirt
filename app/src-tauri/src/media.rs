//! Media keys and the OS "now playing" panel (macOS Control Center, Windows
//! media overlay, Linux MPRIS), via souvlaki.
//!
//! - OS events (play, pause, next, seek...) are sent to the player.
//! - A sync thread watches the player and publishes the track's metadata and
//!   playback state. souvlaki's controls live on the main thread, so updates
//!   go through `run_on_main_thread`.

use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig, SeekDirection};
use squirt_service::Service;
use std::cell::RefCell;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::AppHandle;
#[cfg(windows)]
use tauri::Manager;

thread_local! {
    // Only touched on the main thread.
    static CONTROLS: RefCell<Option<MediaControls>> = const { RefCell::new(None) };
}

/// Sets up media controls. Call from Tauri's `setup` (on the main thread).
pub fn init(app: &AppHandle, svc: Arc<Service>) {
    #[cfg(windows)]
    let hwnd = app
        .get_webview_window("main")
        .and_then(|w| w.hwnd().ok())
        .map(|h| h.0 as *mut std::ffi::c_void);
    #[cfg(not(windows))]
    let hwnd = None;

    let config = PlatformConfig { display_name: "Squirt", dbus_name: "space.foid.squirt", hwnd };
    let mut controls = match MediaControls::new(config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("media controls unavailable: {e:?}");
            return;
        }
    };
    let events_svc = svc.clone();
    let attached = controls.attach(move |event| {
        let svc = events_svc.clone();
        // Player commands wait briefly for the player; keep that off the caller's thread.
        std::thread::spawn(move || handle(&svc, event));
    });
    if let Err(e) = attached {
        eprintln!("media controls unavailable: {e:?}");
        return;
    }
    CONTROLS.with(|c| *c.borrow_mut() = Some(controls));

    let app = app.clone();
    std::thread::Builder::new()
        .name("squirt-now-playing".into())
        .spawn(move || sync(app, svc))
        .expect("spawn now-playing thread");
}

fn handle(svc: &Service, event: MediaControlEvent) {
    let state = svc.player_state();
    let playing = state.status == "playing";
    let seek_by = |delta_ms: i64| {
        let target = (state.position_ms as i64 + delta_ms).clamp(0, state.duration_ms as i64);
        let _ = svc.seek(target as u64);
    };
    match event {
        MediaControlEvent::Play if !playing => drop(svc.toggle()),
        MediaControlEvent::Pause if playing => drop(svc.toggle()),
        MediaControlEvent::Toggle => drop(svc.toggle()),
        MediaControlEvent::Next => drop(svc.next()),
        MediaControlEvent::Previous => drop(svc.prev()),
        MediaControlEvent::Stop => drop(svc.stop()),
        MediaControlEvent::SetPosition(MediaPosition(d)) => drop(svc.seek(d.as_millis() as u64)),
        MediaControlEvent::SeekBy(dir, d) => seek_by(signed(dir, d.as_millis() as i64)),
        MediaControlEvent::Seek(dir) => seek_by(signed(dir, 10_000)),
        _ => {}
    }
}

fn signed(dir: SeekDirection, ms: i64) -> i64 {
    match dir {
        SeekDirection::Forward => ms,
        SeekDirection::Backward => -ms,
    }
}

fn on_main(app: &AppHandle, f: impl FnOnce(&mut MediaControls) + Send + 'static) {
    let _ = app.run_on_main_thread(move || {
        CONTROLS.with(|c| {
            if let Some(controls) = c.borrow_mut().as_mut() {
                f(controls);
            }
        })
    });
}

/// Publishes metadata when the track changes, and playback state when it
/// starts, stops, pauses, or the position jumps (a seek). The OS advances the
/// position itself between updates.
fn sync(app: AppHandle, svc: Arc<Service>) {
    let mut last_track: Option<i64> = None;
    let mut last_status = "";
    let mut last_pos = (0u64, Instant::now());
    loop {
        std::thread::sleep(Duration::from_millis(500));
        let st = svc.player_state();

        if st.track_id != last_track {
            last_track = st.track_id;
            match st.track_id.and_then(|id| svc.now_playing(id)) {
                Some(np) => on_main(&app, move |c| {
                    let cover = np.art_path.map(|p| format!("file://{}", p.display()));
                    let _ = c.set_metadata(MediaMetadata {
                        title: Some(&np.title),
                        artist: Some(&np.artist),
                        album: Some(&np.album),
                        duration: Some(Duration::from_millis(np.duration_ms)),
                        cover_url: cover.as_deref(),
                    });
                }),
                None => on_main(&app, |c| {
                    let _ = c.set_metadata(MediaMetadata::default());
                }),
            }
            last_status = ""; // Force a playback update for the new track.
        }

        let expected = if last_status == "playing" {
            last_pos.0 + last_pos.1.elapsed().as_millis() as u64
        } else {
            last_pos.0
        };
        let jumped = st.position_ms.abs_diff(expected) > 1500;
        if st.status != last_status || jumped {
            last_status = st.status;
            let progress = Some(MediaPosition(Duration::from_millis(st.position_ms)));
            let playback = match st.status {
                "playing" => MediaPlayback::Playing { progress },
                "paused" => MediaPlayback::Paused { progress },
                _ => MediaPlayback::Stopped,
            };
            on_main(&app, move |c| {
                let _ = c.set_playback(playback);
            });
        }
        last_pos = (st.position_ms, Instant::now());
    }
}
