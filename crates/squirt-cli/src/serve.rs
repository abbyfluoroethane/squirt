//! `squirt serve`: a local JSON API over the library, for developing the UI
//! in a plain browser. Listens on 127.0.0.1 only. The Tauri app does not use it.

use anyhow::Result;
use serde_json::{json, Value};
use squirt_library::Library;
use std::time::{Duration, Instant};
use tiny_http::{Header, Response, Server};

fn hue(s: &str) -> u32 {
    s.chars().fold(0u32, |h, c| h.wrapping_mul(31).wrapping_add(c as u32)) % 360
}

fn collection(lib: &Library) -> Result<Value> {
    let mut out = Vec::new();
    for a in lib.albums(None)? {
        let tracks: Vec<Value> = lib
            .album_tracks(&a.artist, &a.title)?
            .into_iter()
            .map(|t| {
                json!({
                    "id": t.id,
                    "disc": t.disc_no.unwrap_or(1),
                    "n": t.track_no.unwrap_or(0),
                    "title": t.title,
                    "artist": t.artist,
                    "durationMs": t.duration_ms,
                    "size": t.size,
                    "codec": t.codec,
                    "onDevice": false,
                })
            })
            .collect();
        out.push(json!({
            "id": format!("{}\u{1f}{}", a.artist, a.title),
            "title": a.title,
            "artist": a.artist,
            "year": a.year,
            "genre": a.genre,
            "hue": hue(&format!("{}{}", a.artist, a.title)),
            "art": a.art.map(|k| format!("/api/art/{k}.jpg")),
            "tracks": tracks,
        }));
    }
    Ok(Value::Array(out))
}

/// Device status, read with a fresh connection. Cached briefly because
/// connecting (with the MTPZ handshake) takes about a second.
fn device_status() -> Value {
    let disconnected = json!({ "connected": false, "name": "", "model": "", "firmware": "",
        "capacity": 0, "free": 0, "usage": { "music": 0, "videos": 0, "pictures": 0, "other": 0 } });
    let keys = match squirt_mtp::mtpz::MtpzKeys::default_path().map(|p| squirt_mtp::mtpz::MtpzKeys::load(&p)) {
        Some(Ok(k)) => k,
        _ => return disconnected,
    };
    let Ok(mut z) = squirt_mtp::device::Zune::connect(Some(&keys)) else { return disconnected };
    let Some(storage) = z.storage_ids().ok().and_then(|s| s.first().copied()) else { return disconnected };
    let Ok(si) = z.storage_info(storage) else { return disconnected };
    let name = z.raw(0x1015, &[0xD402]).ok().and_then(|d| squirt_mtp::dataset::Reader::new(&d).string().ok()).unwrap_or_default();
    let used = si.max_capacity.saturating_sub(si.free_bytes);
    json!({
        "connected": true,
        "name": name,
        "model": z.info.model,
        "firmware": z.info.device_version.split('-').next().unwrap_or(""),
        "capacity": si.max_capacity,
        "free": si.free_bytes,
        // Per-type usage needs a full object walk; report it all as music for now.
        "usage": { "music": used, "videos": 0, "pictures": 0, "other": 0 },
    })
}

fn query(url: &str, key: &str) -> Option<String> {
    url.split_once('?')?.1.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then(|| v.to_string())
    })
}

/// Player plus the library ids of its queue, so events map back to tracks.
struct Playback {
    player: squirt_player::Player,
    queue: Vec<i64>,
}

fn player_state(pb: &Option<Playback>) -> Value {
    let Some(pb) = pb else {
        return json!({ "status": "stopped", "trackId": null, "positionMs": 0, "durationMs": 0, "volume": 0.7, "queue": [] });
    };
    let s = pb.player.state();
    json!({
        "status": format!("{:?}", s.status).to_lowercase(),
        "trackId": s.index.and_then(|i| pb.queue.get(i).copied()),
        "positionMs": s.position_ms,
        "durationMs": s.duration_ms,
        "volume": s.volume,
        "queue": pb.queue,
    })
}

pub fn serve(port: u16) -> Result<()> {
    let lib = crate::open_library()?;
    let mut playback: Option<Playback> = None;
    let server = Server::http(("127.0.0.1", port)).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("squirt dev API on http://127.0.0.1:{port}/api");
    let json_header = Header::from_bytes("Content-Type", "application/json").unwrap();
    let mut device_cache: Option<(Instant, Value)> = None;

    for req in server.incoming_requests() {
        let full = req.url().to_string();
        let url = full.split('?').next().unwrap_or("").to_string();

        // Count plays for tracks that finished since the last request.
        if let Some(pb) = &playback {
            for e in pb.player.take_events() {
                if let squirt_player::Event::TrackCompleted(i) = e {
                    if let Some(id) = pb.queue.get(i) {
                        let _ = lib.record_play(*id);
                    }
                }
            }
        }

        if let Some(action) = url.strip_prefix("/api/player/") {
            if action != "state" {
                if playback.is_none() {
                    match squirt_player::Player::new() {
                        Ok(p) => {
                            p.set_volume(0.7);
                            playback = Some(Playback { player: p, queue: Vec::new() });
                        }
                        Err(e) => {
                            let _ = req.respond(Response::from_string(e.to_string()).with_status_code(500));
                            continue;
                        }
                    }
                }
                let pb = playback.as_mut().unwrap();
                match action {
                    "play" => {
                        let ids: Vec<i64> = query(&full, "ids")
                            .unwrap_or_default()
                            .split(',')
                            .filter_map(|x| x.parse().ok())
                            .collect();
                        let start: usize = query(&full, "start").and_then(|v| v.parse().ok()).unwrap_or(0);
                        let paths: Vec<_> = ids
                            .iter()
                            .filter_map(|id| lib.track(*id).ok().flatten().map(|t| t.path))
                            .collect();
                        if !paths.is_empty() {
                            pb.queue = ids;
                            pb.player.play_queue(paths, start.min(pb.queue.len() - 1));
                        }
                    }
                    "toggle" => pb.player.toggle(),
                    "next" => pb.player.next(),
                    "prev" => pb.player.prev(),
                    "stop" => pb.player.stop(),
                    "seek" => {
                        if let Some(ms) = query(&full, "ms").and_then(|v| v.parse().ok()) {
                            pb.player.seek(ms);
                        }
                    }
                    "volume" => {
                        if let Some(v) = query(&full, "v").and_then(|v| v.parse().ok()) {
                            pb.player.set_volume(v);
                        }
                    }
                    "jump" => {
                        if let Some(i) = query(&full, "i").and_then(|v| v.parse().ok()) {
                            pb.player.jump(i);
                        }
                    }
                    _ => {}
                }
            }
            // Give the player a moment to act so the returned state is current.
            if action != "state" {
                std::thread::sleep(Duration::from_millis(30));
            }
            let _ = req.respond(Response::from_string(player_state(&playback).to_string()).with_header(json_header.clone()));
            continue;
        }
        let resp = match url.as_str() {
            "/api/health" => Response::from_string(json!({ "ok": true }).to_string()).with_header(json_header.clone()).boxed(),
            "/api/collection" => match collection(&lib) {
                Ok(v) => Response::from_string(v.to_string()).with_header(json_header.clone()).boxed(),
                Err(e) => Response::from_string(e.to_string()).with_status_code(500).boxed(),
            },
            "/api/device" => {
                let fresh = device_cache.as_ref().is_some_and(|(t, _)| t.elapsed() < Duration::from_secs(15));
                if !fresh {
                    device_cache = Some((Instant::now(), device_status()));
                }
                let v = &device_cache.as_ref().unwrap().1;
                Response::from_string(v.to_string()).with_header(json_header.clone()).boxed()
            }
            u if u.starts_with("/api/art/") => {
                let key = u.trim_start_matches("/api/art/").trim_end_matches(".jpg");
                if key.chars().all(|c| c.is_ascii_hexdigit()) {
                    match std::fs::read(lib.art_path(key)) {
                        Ok(b) => Response::from_data(b)
                            .with_header(Header::from_bytes("Content-Type", "image/jpeg").unwrap())
                            .with_header(Header::from_bytes("Cache-Control", "max-age=86400").unwrap())
                            .boxed(),
                        Err(_) => Response::empty(404).boxed(),
                    }
                } else {
                    Response::empty(400).boxed()
                }
            }
            _ => Response::empty(404).boxed(),
        };
        let _ = req.respond(resp);
    }
    Ok(())
}
