//! `squirt serve`: the app's backend as a local JSON API, for developing the
//! UI in a plain browser. Listens on 127.0.0.1 only. The Tauri app calls the
//! same `squirt_service::Service` directly.

use anyhow::Result;
use serde_json::json;
use squirt_service::Service;
use std::sync::Arc;
use tiny_http::{Header, Request, Response, Server};

fn query(url: &str, key: &str) -> Option<String> {
    url.split_once('?')?.1.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then(|| v.to_string())
    })
}

fn json_response<T: serde::Serialize>(v: &T) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string(serde_json::to_string(v).unwrap_or_else(|_| "null".into()))
        .with_header(Header::from_bytes("Content-Type", "application/json").unwrap())
}

fn error(e: String) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string(json!({ "error": e }).to_string()).with_status_code(500)
}

fn handle(svc: &Service, req: Request) {
    let full = req.url().to_string();
    let path = full.split('?').next().unwrap_or("").to_string();
    let q = |k: &str| query(&full, k);
    let resp = match path.as_str() {
        "/api/health" => json_response(&json!({ "ok": true })),
        "/api/collection" => svc.collection().map(|v| json_response(&v)).unwrap_or_else(error),
        "/api/device" => json_response(&svc.device_status()),
        "/api/device/collection" => svc.device_collection().map(|v| json_response(&v)).unwrap_or_else(error),
        "/api/player/state" => json_response(&svc.player_state()),
        "/api/player/play" => {
            let ids = q("ids").unwrap_or_default().split(',').filter_map(|x| x.parse().ok()).collect();
            let start = q("start").and_then(|v| v.parse().ok()).unwrap_or(0);
            svc.play(ids, start).map(|v| json_response(&v)).unwrap_or_else(error)
        }
        "/api/player/toggle" => svc.toggle().map(|v| json_response(&v)).unwrap_or_else(error),
        "/api/player/next" => svc.next().map(|v| json_response(&v)).unwrap_or_else(error),
        "/api/player/prev" => svc.prev().map(|v| json_response(&v)).unwrap_or_else(error),
        "/api/player/seek" => {
            let ms = q("ms").and_then(|v| v.parse().ok()).unwrap_or(0);
            svc.seek(ms).map(|v| json_response(&v)).unwrap_or_else(error)
        }
        "/api/player/volume" => {
            let v = q("v").and_then(|v| v.parse().ok()).unwrap_or(0.7);
            svc.volume(v).map(|v| json_response(&v)).unwrap_or_else(error)
        }
        "/api/social/status" => json_response(&svc.social_status()),
        "/api/social/lastfm/begin" => svc.lastfm_begin_auth().map(|u| json_response(&json!({ "url": u }))).unwrap_or_else(error),
        "/api/social/lastfm/finish" => svc.lastfm_finish_auth().map(|v| json_response(&v)).unwrap_or_else(error),
        "/api/social/lastfm/disconnect" => svc.lastfm_disconnect().map(|v| json_response(&v)).unwrap_or_else(error),
        "/api/social/lastfm/enabled" => svc.lastfm_set_enabled(q("on").as_deref() == Some("true")).map(|v| json_response(&v)).unwrap_or_else(error),
        "/api/social/lastfm/profile" => svc.lastfm_profile().map(|v| json_response(&v)).unwrap_or_else(error),
        "/api/social/discord/enabled" => svc.discord_set_enabled(q("on").as_deref() == Some("true")).map(|v| json_response(&v)).unwrap_or_else(error),
        p if p.starts_with("/api/art/") => match svc.art(p.trim_start_matches("/api/art/")) {
            Some(bytes) => Response::from_data(bytes)
                .with_header(Header::from_bytes("Content-Type", "image/jpeg").unwrap())
                .with_header(Header::from_bytes("Cache-Control", "max-age=3600").unwrap()),
            None => Response::from_string("").with_status_code(404),
        },
        _ => Response::from_string("").with_status_code(404),
    };
    let _ = req.respond(resp);
}

pub fn serve(port: u16) -> Result<()> {
    let svc = Arc::new(Service::open().map_err(|e| anyhow::anyhow!(e))?);
    svc.start_background();
    let server = Server::http(("127.0.0.1", port)).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("squirt dev API on http://127.0.0.1:{port}/api");
    for req in server.incoming_requests() {
        let svc = svc.clone();
        // One thread per request: a slow device read must not hold up playback controls.
        std::thread::spawn(move || handle(&svc, req));
    }
    Ok(())
}
