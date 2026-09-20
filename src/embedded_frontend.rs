// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::body::Body;
use axum::http::{header, StatusCode, Uri};
use axum::response::Response;

const FRONTEND_DIR: &str = "frontend/dist/";
const INDEX: &str = "index.html";

/// Set by the server when TLS is active so HSTS is only ever emitted over
/// HTTPS (browsers ignore HSTS on plaintext HTTP anyway).
static TLS_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn set_tls_active(active: bool) {
    TLS_ACTIVE.store(active, Ordering::Relaxed);
}

/// Tight content policy: the bundled UI is fully self-hosted (scripts,
/// styles, fonts, images), so everything can be confined to `'self'`.
/// `connect-src` additionally names ws(s) explicitly for the streaming
/// socket. `frame-ancestors 'none'` blocks clickjacking outright.
const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self'; \
    font-src 'self'; img-src 'self' data:; connect-src 'self' ws: wss:; \
    object-src 'none'; base-uri 'self'; frame-ancestors 'none'";

fn content_type_for(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "woff" | "woff2" => "font/woff2",
        "json" => "application/json",
        "map" => "application/json",
        _ => "application/octet-stream",
    }
}

fn read_frontend_file(path: &str) -> Option<Vec<u8>> {
    let mut full = PathBuf::from(FRONTEND_DIR);
    full.push(path);
    std::fs::read(full).ok()
}

pub async fn serve_embedded(uri: Uri) -> Response<Body> {
    let mut path = uri.path().trim_start_matches('/').to_string();
    if path.is_empty() {
        path = INDEX.to_string();
    }

    let data = match read_frontend_file(&path) {
        Some(bytes) => bytes,
        None => match read_frontend_file(INDEX) {
            Some(bytes) => {
                path = INDEX.to_string();
                bytes
            }
            None => {
                return Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
                    .body(Body::from(
                        "Frontend assets missing; build the frontend first.",
                    ))
                    .unwrap()
            }
        },
    };

    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type_for(&path))
        .header(header::CACHE_CONTROL, "no-cache")
        .header("Content-Security-Policy", CSP)
        .header("X-Content-Type-Options", "nosniff")
        .header("Referrer-Policy", "no-referrer")
        .header(
            "Permissions-Policy",
            "camera=(), microphone=(), geolocation=(), payment=()",
        );
    if TLS_ACTIVE.load(Ordering::Relaxed) {
        builder = builder.header(
            "Strict-Transport-Security",
            "max-age=31536000; includeSubDomains",
        );
    }
    builder.body(Body::from(data)).unwrap()
}
