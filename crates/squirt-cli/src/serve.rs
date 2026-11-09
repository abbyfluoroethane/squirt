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

pub fn serve(port: u16) -> Result<()> {
    let lib = crate::open_library()?;
    let server = Server::http(("127.0.0.1", port)).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("squirt dev API on http://127.0.0.1:{port}/api");
    let json_header = Header::from_bytes("Content-Type", "application/json").unwrap();
    let mut device_cache: Option<(Instant, Value)> = None;

    for req in server.incoming_requests() {
        let url = req.url().split('?').next().unwrap_or("").to_string();
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
