mod cdn;
mod config;
mod disk;
mod image;
mod metrics;
mod params;
mod service;
mod source;

use anyhow::{Context, Result};
#[cfg(test)]
use axum::http::HeaderMap;
use axum::{
    Router,
    http::{StatusCode, header},
    middleware,
    response::{IntoResponse, Response},
    routing::get,
};
use bytes::Bytes;
use config::{Config, number};
use disk::DiskCache;
use moka::future::Cache;
use params::Transforms;
use sha2::{Digest, Sha256};
use source::Source;
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::Semaphore;

#[derive(Clone, Debug)]
struct Entry {
    bytes: Bytes,
    mime: String,
    etag: String,
    expires: Instant,
}
impl Entry {
    fn new(bytes: Bytes, mime: String, ttl: Duration) -> Self {
        let hash = bytes.iter().fold(2166136261u32, |h, b| {
            (h ^ u32::from(*b)).wrapping_mul(16777619)
        });
        Self {
            bytes,
            mime,
            etag: format!("W/\"{hash:x}\""),
            expires: Instant::now() + ttl,
        }
    }
}

struct Expiration;
impl moka::Expiry<String, Entry> for Expiration {
    fn expire_after_create(&self, _: &String, value: &Entry, created: Instant) -> Option<Duration> {
        Some(value.expires.saturating_duration_since(created))
    }
}

#[derive(Clone, Debug)]
struct Failure(StatusCode, &'static str);
impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        let mut response = (self.0, self.1).into_response();
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
        if self.0 == StatusCode::SERVICE_UNAVAILABLE || self.0 == StatusCode::TOO_MANY_REQUESTS {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, "1".parse().unwrap());
        }
        response
    }
}

struct App {
    config: Config,
    source: Source,
    static_source: Source,
    namespace: String,
    stats: metrics::Metrics,
    cache: Cache<String, Entry>,
    disk: DiskCache,
    watermark: Option<Bytes>,
    requests: Semaphore,
    admission: Arc<Semaphore>,
    workers: Arc<Semaphore>,
    rates: Mutex<HashMap<IpAddr, (u32, u64)>>,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn mime(format: &str) -> String {
    format!("image/{format}")
}

impl App {
    fn allow(&self, ip: IpAddr) -> bool {
        let now = now();
        let mut rates = self.rates.lock().unwrap();
        // Bound metadata even if clients rotate IPs. No idle cleanup timer.
        if rates.len() >= 4096 {
            rates.retain(|_, (_, expiry)| *expiry > now);
        }
        if rates.len() >= 4096 && !rates.contains_key(&ip) {
            return false;
        }
        let bucket = rates
            .entry(ip)
            .or_insert((0, now + self.config.open_window));
        if bucket.1 <= now {
            *bucket = (0, now + self.config.open_window);
        }
        if bucket.0 >= self.config.open_rps {
            return false;
        }
        bucket.0 += 1;
        true
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Read only pixerve/.env (or the working directory's .env), never silently
    // discover the parent application's production secrets.
    let _ = dotenvy::from_path(".env");
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pixerve=info".into()),
        )
        .init();
    if std::env::args().nth(1).is_some() {
        return cli();
    }
    let config = Config::load()?;
    rs_vips::Vips::init("pixerve").context("Cannot initialize libvips")?;
    rs_vips::Vips::concurrency_set(number::<i32>("VIPS_CONCURRENCY", 1)?.clamp(1, 4));
    // Disable the native operation cache: encoded result caches are sufficient,
    // and this avoids retaining source buffers and decode pipelines at idle.
    rs_vips::Vips::cache_set_max(0);
    rs_vips::Vips::cache_set_max_mem(0);
    rs_vips::Vips::cache_set_max_files(0);
    let source = Source::new(&config).await?;
    let disk = DiskCache::new(config.disk_dir.clone(), config.disk_bytes, config.cache_ttl).await?;
    let watermark = Some(Bytes::from(
        tokio::fs::read(&config.watermark)
            .await
            .context("OG_WATERMARK unavailable")?,
    ));
    let static_root = tokio::fs::canonicalize(&config.static_dir)
        .await
        .context("STATIC_DIR unavailable")?;
    let source_id = if let Some(root) = &config.source_dir {
        tokio::fs::canonicalize(root)
            .await?
            .to_string_lossy()
            .into_owned()
    } else {
        format!(
            "{}|{}|{}",
            std::env::var("S3_ENDPOINT")?,
            std::env::var("S3_BUCKET")?,
            std::env::var("S3_REGION").unwrap_or_else(|_| "auto".into())
        )
    };
    let namespace = hex::encode(Sha256::digest(
        serde_json::to_vec(&(
            "cdn-v2",
            &config.cdn.pipeline,
            source_id,
            &static_root,
            watermark.as_ref().map(|b| hex::encode(Sha256::digest(b))),
        ))
        .unwrap(),
    ));
    let cache = Cache::builder()
        .max_capacity(config.mem_cache_bytes)
        .expire_after(Expiration)
        .weigher(|key: &String, v: &Entry| {
            (key.len() + v.bytes.len() + v.mime.len() + v.etag.len() + 128).min(u32::MAX as usize)
                as u32
        })
        .time_to_live(Duration::from_secs(config.cache_ttl))
        .time_to_idle(Duration::from_secs(config.cache_idle))
        .build();
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let app = Arc::new(App {
        requests: Semaphore::new(config.max_requests),
        admission: Arc::new(Semaphore::new(config.max_concurrent + config.max_queue)),
        workers: Arc::new(Semaphore::new(config.max_concurrent)),
        config,
        source,
        static_source: Source::Local(static_root),
        namespace,
        stats: metrics::Metrics::default(),
        cache,
        disk,
        watermark,
        rates: Mutex::new(HashMap::new()),
    });
    let router = Router::new()
        .route(
            "/health",
            get(|| async { axum::Json(serde_json::json!({"ok":true})) }),
        )
        .route("/metrics", get(service::snapshot))
        .fallback(get(service::handle))
        .layer(middleware::from_fn_with_state(
            app.clone(),
            service::observe,
        ))
        .with_state(app.clone());
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr,"pixerve ready");
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown())
    .await?;
    app.cache.run_pending_tasks().await;
    // Tokio waits for blocking transforms during runtime shutdown. Keep vips
    // initialized until then rather than destroying it while workers exist.
    Ok(())
}

async fn shutdown() {
    #[cfg(unix)]
    {
        if let Ok(mut terminate) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {},
                _ = terminate.recv() => {},
            }
            return;
        }
    }
    let _ = tokio::signal::ctrl_c().await;
}

fn cli() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    anyhow::ensure!(
        args.len() >= 3,
        "Usage: pixerve version FILE [prefix] | url PATH PARAMS | asset PATH PARAMS | sign PATH PARAMS [ttl] | sign-until PATH PARAMS EXPIRY"
    );
    if args[1] == "version" {
        use std::io::Read;
        let path = std::path::Path::new(&args[2]);
        let mut file = std::fs::File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 65536];
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }
        let hash = hex::encode(hasher.finalize());
        let stem = path
            .file_stem()
            .and_then(|v| v.to_str())
            .context("Invalid filename")?;
        let stem = if let Some(old_hash) = path
            .file_name()
            .and_then(|v| v.to_str())
            .and_then(cdn::source_hash)
        {
            stem.strip_suffix(&format!("-{old_hash}")).unwrap_or(stem)
        } else {
            stem
        };
        let extension = path
            .extension()
            .and_then(|v| v.to_str())
            .map(|v| format!(".{v}"))
            .unwrap_or_default();
        let prefix = args.get(3).map(|s| s.trim_matches('/')).unwrap_or("");
        let key = format!(
            "{}{stem}-{hash}{extension}",
            if prefix.is_empty() {
                String::new()
            } else {
                format!("{prefix}/")
            }
        );
        params::validate_path(&key)?;
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({"sha256":hash,"key":key}))?
        );
        return Ok(());
    }
    anyhow::ensure!(
        ["url", "asset", "sign", "sign-until"].contains(&args[1].as_str()) && args.len() >= 4,
        "Unknown command or missing parameters"
    );
    let settings = cdn::Settings::load()?;
    anyhow::ensure!(
        !settings.private || args[1] != "url",
        "Private mode requires sign or sign-until"
    );
    let path = args[2].trim_start_matches('/');
    let og = path.starts_with("og/");
    let static_file = path.starts_with("static/");
    let path = if og {
        &path[3..]
    } else if static_file {
        &path[7..]
    } else {
        path
    };
    params::validate_path(path)?;
    let map = args[3]
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|pair| {
            pair.split_once('=')
                .map(|(k, v)| (k.into(), v.into()))
                .context("Expected key=value")
        })
        .collect::<Result<HashMap<String, String>>>()?;
    let mut t = if og {
        Transforms::preset("og")?
    } else if static_file {
        Transforms::default()
    } else {
        Transforms::parse(&map)?
    };
    if !og && !static_file && !map.is_empty() && t.f.is_none() {
        t.f = Some("webp".into());
    }
    let mut auth = HashMap::new();
    if args[1] == "url" {
        println!(
            "{}",
            cdn::canonical_url(path, &t, og, static_file, Some(&settings.pipeline), &auth)
        );
        return Ok(());
    }
    let secret = std::env::var("SIGNING_KEY").context("SIGNING_KEY required")?;
    anyhow::ensure!(!secret.is_empty(), "SIGNING_KEY must not be empty");
    if args[1] == "asset" {
        anyhow::ensure!(
            !settings.private
                && cdn::source_hash(path).is_some()
                && (og
                    || static_file
                    || map.is_empty()
                    || t.f.as_deref().is_some_and(|f| f != "auto")),
            "asset requires public mode, a full source hash and a fixed output format"
        );
        auth.insert(
            "asig".into(),
            params::asset_signature(&secret, path, &t, &settings.pipeline),
        );
        println!(
            "{}",
            cdn::canonical_url(path, &t, og, static_file, Some(&settings.pipeline), &auth)
        );
        return Ok(());
    }
    let expiry = if args[1] == "sign-until" {
        args.get(4).context("Absolute expiry required")?.parse()?
    } else {
        now()
            .checked_add(args.get(4).map(|s| s.parse()).transpose()?.unwrap_or(86400))
            .context("Expiry overflow")?
    };
    anyhow::ensure!(expiry > now(), "Expiry must be in the future");
    let sig = params::signature(&secret, path, &t, expiry);
    auth.insert("sig".into(), sig);
    auth.insert("exp".into(), expiry.to_string());
    println!(
        "{}",
        cdn::canonical_url(path, &t, og, static_file, Some(&settings.pipeline), &auth)
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn etag_and_conditional_headers_are_compatible() {
        let entry = Entry::new(
            Bytes::from_static(b"hello"),
            "image/jpeg".into(),
            Duration::from_secs(60),
        );
        assert_eq!(entry.etag, "W/\"4f9f2cab\"");
        let headers = HeaderMap::from_iter([(
            header::IF_NONE_MATCH,
            "\"other\", \"4f9f2cab\"".parse().unwrap(),
        )]);
        let response = service::response(
            entry,
            &headers,
            &cdn::Settings::load().unwrap(),
            false,
            true,
            "RAM",
        );
        assert_eq!(response.status(), 304);
        assert_eq!(response.headers()[header::VARY], "Accept");
        assert!(response.headers().get(header::CONTENT_LENGTH).is_none());
    }
    #[tokio::test]
    async fn coalesces_identical_requests() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let calls = Arc::new(AtomicUsize::new(0));
        let cache = Cache::<String, u32>::new(100);
        let mut jobs = Vec::new();
        for _ in 0..20 {
            let calls = calls.clone();
            let cache = cache.clone();
            jobs.push(tokio::spawn(async move {
                cache
                    .try_get_with("same".into(), async {
                        calls.fetch_add(1, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_millis(20)).await;
                        Ok::<_, Failure>(42)
                    })
                    .await
                    .unwrap()
            }));
        }
        for job in jobs {
            assert_eq!(job.await.unwrap(), 42);
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
