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

/// Decodes %XX escapes and '+' in a query value.
fn url_decode(s: &str) -> String {
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
        out.push(if b[i] == b'+' { b' ' } else { b[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn json_response<T: serde::Serialize>(v: &T) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string(serde_json::to_string(v).unwrap_or_else(|_| "null".into()))
        .with_header(Header::from_bytes("Content-Type", "application/json").unwrap())
}

fn error(e: String) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string(json!({ "error": e }).to_string()).with_status_code(500)
}

fn content_type(path: &std::path::Path) -> &'static str {
    match path.extension().map(|e| e.to_string_lossy().to_lowercase()).as_deref() {
        Some("mp4" | "m4v") => "video/mp4",
        Some("mov") => "video/quicktime",
        Some("webm") => "video/webm",
        Some("mkv") => "video/x-matroska",
        Some("wmv") => "video/x-ms-wmv",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("bmp") => "image/bmp",
        _ => "application/octet-stream",
    }
}

/// Serves a file, honouring a `Range: bytes=a-b` header so video can seek.
fn file_response(path: &std::path::Path, range: Option<&str>) -> Response<Box<dyn std::io::Read + Send>> {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut f) = std::fs::File::open(path) else {
        return Response::new(404.into(), vec![], Box::new(std::io::empty()), Some(0), None);
    };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let ct = Header::from_bytes("Content-Type", content_type(path)).unwrap();
    let accept = Header::from_bytes("Accept-Ranges", "bytes").unwrap();
    let parsed = range.and_then(|r| r.strip_prefix("bytes=")).and_then(|r| {
        let (a, b) = r.split_once('-')?;
        let start: u64 = a.parse().ok()?;
        let end: u64 = if b.is_empty() { len.saturating_sub(1) } else { b.parse().ok()? };
        (start <= end && end < len).then_some((start, end))
    });
    match parsed {
        Some((start, end)) => {
            let _ = f.seek(SeekFrom::Start(start));
            let n = end - start + 1;
            let cr = Header::from_bytes("Content-Range", format!("bytes {start}-{end}/{len}")).unwrap();
            Response::new(206.into(), vec![ct, accept, cr], Box::new(f.take(n)), Some(n as usize), None)
        }
        None => Response::new(200.into(), vec![ct, accept], Box::new(f), Some(len as usize), None),
    }
}

fn handle(svc: &Arc<Service>, req: Request) {
    let full = req.url().to_string();
    let path = full.split('?').next().unwrap_or("").to_string();
    let q = |k: &str| query(&full, k);
    let resp = match path.as_str() {
        "/api/health" => json_response(&json!({ "ok": true })),
        "/api/collection" => svc.collection().map(|v| json_response(&v)).unwrap_or_else(error),
        "/api/device" => json_response(&svc.device_status()),
        "/api/folders" => svc.folders().map(|v| json_response(&v)).unwrap_or_else(error),
        "/api/folders/add" => {
            let (path, kind) = (url_decode(&q("path").unwrap_or_default()), q("kind").unwrap_or_default());
            svc.add_folder(&path, &kind).map(|v| json_response(&v)).unwrap_or_else(error)
        }
        "/api/folders/remove" => svc.remove_folder(q("id").and_then(|v| v.parse().ok()).unwrap_or(-1)).map(|v| json_response(&v)).unwrap_or_else(error),
        "/api/scan" => json_response(&svc.scan_status()),
        "/api/scan/start" => {
            svc.rescan();
            json_response(&svc.scan_status())
        }
        "/api/videos" => svc.videos().map(|v| json_response(&v)).unwrap_or_else(error),
        "/api/pictures" => svc.pictures().map(|v| json_response(&v)).unwrap_or_else(error),
        "/api/podcasts" => svc.podcasts().map(|v| json_response(&v)).unwrap_or_else(error),
        p if p.starts_with("/api/media/") => {
            let mut parts = p.trim_start_matches("/api/media/").splitn(2, '/');
            let (kind, id) = (parts.next().unwrap_or(""), parts.next().and_then(|i| i.parse().ok()).unwrap_or(-1));
            let range = req.headers().iter().find(|h| h.field.equiv("Range")).map(|h| h.value.as_str().to_string());
            match svc.media_path(kind, id) {
                Some(path) => {
                    let _ = req.respond(file_response(&path, range.as_deref()));
                    return;
                }
                None => Response::from_string("").with_status_code(404),
            }
        }
        "/api/device/info" => json_response(&svc.device_info()),
        "/api/device/rename" => {
            let name = q("name").map(|n| url_decode(&n)).unwrap_or_default();
            svc.device_rename(&name).map(|v| json_response(&v)).unwrap_or_else(error)
        }
        "/api/device/refresh" => svc.device_refresh().map(|v| json_response(&v)).unwrap_or_else(error),
        "/api/device/release" => json_response(&svc.device_release()),
        "/api/device/connect" => json_response(&svc.device_connect()),
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
