use crate::{
    App, Entry, Failure, cdn, image, metrics, mime, now,
    params::{self, Transforms},
};
use axum::{
    body::Body,
    extract::{ConnectInfo, OriginalUri, Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
    time::{Duration, Instant},
};

pub async fn handle(
    State(app): State<Arc<App>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
) -> Result<Response, Failure> {
    if !app.config.cdn.origin_token.is_empty()
        && !headers
            .get("x-img-origin-token")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| params::verify_bearer(&app.config.cdn.origin_token, v))
    {
        return Err(Failure(StatusCode::FORBIDDEN, "Origin access denied"));
    }
    let _request = app
        .requests
        .try_acquire()
        .map_err(|_| Failure(StatusCode::SERVICE_UNAVAILABLE, "Busy"))?;
    let path = percent_encoding::percent_decode_str(uri.path().trim_start_matches('/'))
        .decode_utf8()
        .map_err(|_| Failure(StatusCode::BAD_REQUEST, "Invalid path"))?
        .into_owned();
    let pairs: Vec<(String, String)> = serde_urlencoded::from_str(uri.query().unwrap_or(""))
        .map_err(|_| Failure(StatusCode::BAD_REQUEST, "Invalid query"))?;
    let mut query = HashMap::new();
    for (key, value) in pairs {
        if query.insert(key, value).is_some() {
            return Err(Failure(
                StatusCode::BAD_REQUEST,
                "Duplicate query parameter",
            ));
        }
    }
    let static_file = path.starts_with("static/");
    let og = path.starts_with("og/");
    let token = path.starts_with("tr:");
    let query_transform =
        params::KEYS.iter().any(|k| query.contains_key(*k)) || query.contains_key("p");
    let (file, t, mut auth, transform) = if static_file {
        (
            path[7..].to_owned(),
            Transforms::default(),
            HashMap::new(),
            false,
        )
    } else if og {
        (
            path[3..].to_owned(),
            Transforms::preset("og").unwrap(),
            HashMap::new(),
            true,
        )
    } else if token {
        let (token, file) = path
            .split_once('/')
            .ok_or(Failure(StatusCode::BAD_REQUEST, "Missing source path"))?;
        let map = params::checked_token_map(&token[3..])
            .map_err(|_| Failure(StatusCode::BAD_REQUEST, "Invalid transform token"))?;
        let t = Transforms::parse(&map)
            .map_err(|_| Failure(StatusCode::BAD_REQUEST, "Invalid transform"))?;
        (file.to_owned(), t, map, true)
    } else {
        let t = Transforms::parse(&query)
            .map_err(|_| Failure(StatusCode::BAD_REQUEST, "Invalid transform"))?;
        (path, t, HashMap::new(), query_transform)
    };
    params::validate_path(&file)
        .map_err(|_| Failure(StatusCode::BAD_REQUEST, "Invalid source path"))?;
    for (long, short) in [("sig", "s"), ("exp", "e"), ("asig", "asig")] {
        if let Some(value) = query.get(long).or_else(|| query.get(short)) {
            auth.insert(long.into(), value.clone());
        }
    }
    let open = app.config.signing_key.is_empty();
    let pv = query.get("pv").map(String::as_str);
    if auth.contains_key("asig")
        && (auth.contains_key("sig")
            || auth.contains_key("s")
            || auth.contains_key("exp")
            || auth.contains_key("e"))
    {
        return Err(Failure(StatusCode::BAD_REQUEST, "Ambiguous signature"));
    }
    if let Some(sig) = auth.get("asig") {
        if app.config.cdn.private
            || open
            || cdn::source_hash(&file).is_none()
            || pv != Some(app.config.cdn.pipeline.as_str())
            || (transform && !og && t.f.as_deref().is_none_or(|f| f == "auto"))
        {
            return Err(Failure(
                StatusCode::FORBIDDEN,
                "Public asset signature not allowed",
            ));
        }
        params::verify_asset(
            &app.config.signing_key,
            &file,
            &t,
            &app.config.cdn.pipeline,
            sig,
        )
        .map_err(|_| Failure(StatusCode::FORBIDDEN, "Forbidden"))?;
    } else if app.config.cdn.private || (!open && transform && !og) {
        params::verify(&app.config.signing_key, &file, &t, &auth, now())
            .map_err(|_| Failure(StatusCode::FORBIDDEN, "Forbidden"))?;
    }
    if pv.is_some_and(|pv| pv != app.config.cdn.pipeline) {
        return Err(Failure(StatusCode::GONE, "Pipeline version unavailable"));
    }
    let no_auth = HashMap::new();
    let canonical = cdn::canonical_url(
        &file,
        &t,
        og,
        static_file,
        pv,
        if open { &no_auth } else { &auth },
    );
    if app.config.cdn.redirects
        && uri
            .path_and_query()
            .is_some_and(|u| u.as_str() != canonical)
    {
        return Ok(Response::builder()
            .status(StatusCode::PERMANENT_REDIRECT)
            .header(header::LOCATION, canonical)
            .header(header::CACHE_CONTROL, "no-store")
            .body(Body::empty())
            .unwrap());
    }
    if transform && open {
        let ip = if app.config.trust_proxy {
            headers
                .get("cf-connecting-ip")
                .or_else(|| headers.get("x-real-ip"))
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok())
                .unwrap_or(peer.ip())
        } else {
            peer.ip()
        };
        if !app.allow(ip) {
            return Err(Failure(
                StatusCode::TOO_MANY_REQUESTS,
                "Rate limit exceeded",
            ));
        }
    }
    let format = if !transform {
        "raw".into()
    } else if og {
        "jpeg".into()
    } else {
        t.format(
            headers
                .get(header::ACCEPT)
                .and_then(|v| v.to_str().ok())
                .unwrap_or(""),
        )
    };
    if transform {
        t.policy(&app.config, &format, open && !og)
            .map_err(|_| Failure(StatusCode::PAYLOAD_TOO_LARGE, "Transform limit exceeded"))?;
        if app.config.cdn.presets_only && !og && !t.is_preset() {
            return Err(Failure(StatusCode::BAD_REQUEST, "Use a supported preset"));
        }
    }
    let vary = transform && !og && t.f.as_deref().is_none_or(|f| f == "auto");
    let immutable = cdn::source_hash(&file).is_some() && pv.is_some() && !vary;
    let output_mime = if transform {
        mime(&format)
    } else {
        mime_guess::from_path(&file)
            .first_or_octet_stream()
            .to_string()
    };
    let key = hex::encode(Sha256::digest(
        serde_json::to_vec(&(
            &app.namespace,
            &file,
            t.cache_normalized(&format),
            og,
            transform,
            static_file,
        ))
        .unwrap(),
    ));
    let label = Arc::new(AtomicU8::new(0));
    let (entry, cache_status) = if let Some(entry) = app.cache.get(&key).await {
        metrics::inc(&app.stats.ram_hits);
        (entry, "RAM")
    } else {
        let worker_app = app.clone();
        let worker_key = key.clone();
        let worker_label = label.clone();
        let entry =
            app.cache
                .try_get_with(key, async move {
                    let app = worker_app;
                    let admission = app
                        .admission
                        .clone()
                        .try_acquire_owned()
                        .map_err(|_| Failure(StatusCode::SERVICE_UNAVAILABLE, "Queue full"))?;
                    let permit =
                        app.workers.clone().acquire_owned().await.map_err(|_| {
                            Failure(StatusCode::SERVICE_UNAVAILABLE, "Shutting down")
                        })?;
                    if let Some((bytes, ttl)) = app.disk.get(&worker_key).await {
                        worker_label.store(1, Ordering::Relaxed);
                        metrics::inc(&app.stats.disk_hits);
                        return Ok(Entry::new(bytes, output_mime, ttl));
                    }
                    worker_label.store(2, Ordering::Relaxed);
                    metrics::inc(&app.stats.misses);
                    metrics::inc(&app.stats.source_reads);
                    let source = if static_file {
                        &app.static_source
                    } else {
                        &app.source
                    };
                    let input = match tokio::time::timeout(
                        Duration::from_secs(30),
                        source.read(&file, app.config.max_source_bytes),
                    )
                    .await
                    {
                        Ok(Ok(bytes)) => bytes,
                        Ok(Err(e)) => {
                            metrics::inc(&app.stats.source_errors);
                            tracing::warn!(error=%e,"Source read failed");
                            return Err(if e.to_string().contains("too large") {
                                Failure(StatusCode::PAYLOAD_TOO_LARGE, "Source too large")
                            } else if e
                                .downcast_ref::<object_store::Error>()
                                .is_some_and(|e| matches!(e, object_store::Error::NotFound { .. }))
                                || e.downcast_ref::<std::io::Error>()
                                    .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound)
                            {
                                Failure(StatusCode::NOT_FOUND, "Source unavailable")
                            } else {
                                Failure(StatusCode::BAD_GATEWAY, "Source unavailable")
                            });
                        }
                        Err(_) => {
                            metrics::inc(&app.stats.source_errors);
                            return Err(Failure(StatusCode::GATEWAY_TIMEOUT, "Source timeout"));
                        }
                    };
                    metrics::add(&app.stats.source_bytes, input.len() as u64);
                    if !cdn::verify_source(&file, &input) {
                        metrics::inc(&app.stats.source_errors);
                        return Err(Failure(StatusCode::CONFLICT, "Source fingerprint mismatch"));
                    }
                    let (bytes, _admission, _permit) = if transform {
                        metrics::inc(&app.stats.transforms);
                        let config = app.config.clone();
                        let watermark = app.watermark.clone();
                        let (result, admission, permit) = tokio::task::spawn_blocking(move || {
                            let result = image::transform(
                                input,
                                &t,
                                &format,
                                og,
                                &config,
                                watermark.as_ref(),
                                open,
                            );
                            (result, admission, permit)
                        })
                        .await
                        .map_err(|_| {
                            metrics::inc(&app.stats.transform_errors);
                            Failure(StatusCode::INTERNAL_SERVER_ERROR, "Transform worker failed")
                        })?;
                        let bytes = result.map_err(|e| {
                            metrics::inc(&app.stats.transform_errors);
                            tracing::warn!(error=%e,"Transform failed");
                            if e.to_string().contains("limit")
                                || e.to_string().contains("dimensions exceeded")
                            {
                                Failure(StatusCode::PAYLOAD_TOO_LARGE, "Image limit exceeded")
                            } else {
                                Failure(
                                    StatusCode::UNPROCESSABLE_ENTITY,
                                    "Unsupported or invalid image",
                                )
                            }
                        })?;
                        (bytes, admission, permit)
                    } else {
                        (input, admission, permit)
                    };
                    if let Err(e) = app.disk.put(&worker_key, &bytes).await {
                        tracing::warn!(error=%e,"Disk cache write failed");
                    }
                    Ok::<_, Failure>(Entry::new(
                        bytes,
                        output_mime,
                        Duration::from_secs(app.config.cache_ttl),
                    ))
                })
                .await
                .map_err(|e| (*e).clone())?;
        let status = match label.load(Ordering::Relaxed) {
            1 => "DISK",
            2 => "MISS",
            _ => {
                metrics::inc(&app.stats.shared);
                "SHARED"
            }
        };
        (entry, status)
    };
    Ok(response(
        entry,
        &headers,
        &app.config.cdn,
        immutable,
        vary,
        cache_status,
    ))
}

pub fn response(
    entry: Entry,
    headers: &HeaderMap,
    settings: &cdn::Settings,
    immutable: bool,
    vary: bool,
    status: &str,
) -> Response {
    let conditional = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|s| {
            s.split(',').any(|tag| {
                tag.trim() == "*"
                    || tag.trim().trim_start_matches("W/") == entry.etag.trim_start_matches("W/")
            })
        });
    let remaining = entry
        .expires
        .saturating_duration_since(Instant::now())
        .as_secs();
    let mut builder = Response::builder()
        .status(if conditional {
            StatusCode::NOT_MODIFIED
        } else {
            StatusCode::OK
        })
        .header(header::ETAG, &entry.etag)
        .header(
            header::CACHE_CONTROL,
            settings.cache_control(immutable, remaining),
        )
        .header("X-Img-Cache", status)
        .header("X-Content-Type-Options", "nosniff");
    if vary {
        builder = builder.header(header::VARY, "Accept");
    }
    if !conditional {
        builder = builder
            .header(header::CONTENT_TYPE, &entry.mime)
            .header(header::CONTENT_LENGTH, entry.bytes.len());
    }
    builder
        .body(if conditional {
            Body::empty()
        } else {
            Body::from(entry.bytes)
        })
        .unwrap()
}

pub async fn observe(State(app): State<Arc<App>>, request: Request, next: Next) -> Response {
    let measured = !matches!(request.uri().path(), "/health" | "/metrics");
    let head = request.method() == axum::http::Method::HEAD;
    let start = Instant::now();
    let mut response = next.run(request).await;
    if measured {
        let bytes = if head {
            0
        } else {
            response
                .headers()
                .get(header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok())
                .unwrap_or(0)
        };
        app.stats.observe(
            response.status().as_u16(),
            bytes,
            start.elapsed(),
            response
                .headers()
                .get("x-img-cache")
                .and_then(|v| v.to_str().ok()),
        );
        response.headers_mut().insert(
            "Server-Timing",
            format!("img;dur={:.3}", start.elapsed().as_secs_f64() * 1000.0)
                .parse()
                .unwrap(),
        );
    } else {
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    }
    response
}

pub async fn snapshot(State(app): State<Arc<App>>, headers: HeaderMap) -> Response {
    let token = &app.config.cdn.metrics_token;
    let provided = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    if token.is_empty() || !provided.is_some_and(|v| params::verify_bearer(token, v)) {
        return Failure(StatusCode::NOT_FOUND, "Not Found").into_response();
    }
    app.cache.run_pending_tasks().await;
    let active = app.config.max_concurrent - app.workers.available_permits();
    let admitted =
        app.config.max_concurrent + app.config.max_queue - app.admission.available_permits();
    #[allow(unused_mut)] // Linux appends RSS; other platforms expose cache memory only.
    let mut body = app.stats.render(
        app.cache.weighted_size(),
        app.cache.entry_count(),
        active,
        admitted.saturating_sub(active),
    );
    #[cfg(target_os = "linux")]
    if let Ok(status) = tokio::fs::read_to_string("/proc/self/status").await {
        if let Some(kib) = status.lines().find_map(|line| {
            line.strip_prefix("VmRSS:")
                .and_then(|v| v.split_whitespace().next())
                .and_then(|v| v.parse::<u64>().ok())
        }) {
            body.push_str(&format!("# TYPE img_process_resident_memory_bytes gauge\nimg_process_resident_memory_bytes {}\n",kib*1024));
        }
    }
    (
        [
            (
                header::CONTENT_TYPE,
                "text/plain; version=0.0.4; charset=utf-8",
            ),
            (header::CACHE_CONTROL, "no-store"),
        ],
        body,
    )
        .into_response()
}
