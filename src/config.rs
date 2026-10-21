use anyhow::{Context, Result, ensure};
use std::{env, path::PathBuf, str::FromStr};

#[derive(Clone)]
pub struct Config {
    pub cdn: crate::cdn::Settings,
    pub port: u16,
    pub max_concurrent: usize,
    pub max_queue: usize,
    pub max_requests: usize,
    pub max_source_bytes: usize,
    pub max_input_pixels: u64,
    pub mp_limit: f64,
    pub mem_cache_bytes: u64,
    pub cache_ttl: u64,
    pub cache_idle: u64,
    pub disk_dir: PathBuf,
    pub disk_bytes: u64,
    pub source_dir: Option<PathBuf>,
    pub static_dir: PathBuf,
    pub watermark: PathBuf,
    pub signing_key: String,
    pub open_max_w: u32,
    pub open_max_h: u32,
    pub open_max_mp: f64,
    pub open_max_blur: f64,
    pub open_formats: Vec<String>,
    pub open_rps: u32,
    pub open_window: u64,
    pub trust_proxy: bool,
}

pub fn number<T: FromStr>(name: &str, fallback: T) -> Result<T> {
    match env::var(name) {
        Ok(value) => value.parse().map_err(|_| anyhow::anyhow!("Invalid {name}")),
        Err(env::VarError::NotPresent) => Ok(fallback),
        Err(e) => Err(e.into()),
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let c = Self {
            cdn: crate::cdn::Settings::load()?,
            port: number("PORT", 3000)?,
            max_concurrent: number("MAX_CONCURRENT", 1)?,
            max_queue: number("MAX_QUEUE", 16)?,
            max_requests: number("MAX_REQUESTS", 128)?,
            max_source_bytes: number::<usize>("MAX_SOURCE_MB", 20)?
                .checked_mul(1024 * 1024)
                .context("MAX_SOURCE_MB overflow")?,
            max_input_pixels: number("MAX_INPUT_PIXELS", 40_000_000)?,
            mp_limit: number("MP_LIMIT", 12.0)?,
            mem_cache_bytes: number::<u64>("MEM_CACHE_MB", 8)?
                .checked_mul(1024 * 1024)
                .context("MEM_CACHE_MB overflow")?,
            cache_ttl: number("CACHE_TTL", 86400)?,
            cache_idle: number("CACHE_IDLE_SECS", 300)?,
            disk_dir: env::var("DISK_CACHE_DIR")
                .unwrap_or_else(|_| ".cache/images".into())
                .into(),
            disk_bytes: number::<u64>("DISK_CACHE_MB", 256)?
                .checked_mul(1024 * 1024)
                .context("DISK_CACHE_MB overflow")?,
            source_dir: env::var_os("SOURCE_DIR")
                .filter(|v| !v.is_empty())
                .map(PathBuf::from),
            static_dir: env::var("STATIC_DIR")
                .unwrap_or_else(|_| "static".into())
                .into(),
            watermark: env::var("OG_WATERMARK")
                .unwrap_or_else(|_| "assets/og_wm.png".into())
                .into(),
            signing_key: env::var("SIGNING_KEY").unwrap_or_default(),
            open_max_w: number("OPEN_MAX_W", 1280)?,
            open_max_h: number("OPEN_MAX_H", 1200)?,
            open_max_mp: number("OPEN_MAX_MP", 3.0)?,
            open_max_blur: number("OPEN_MAX_BLUR", 10.0)?,
            open_formats: env::var("OPEN_ALLOWED_FORMATS")
                .unwrap_or_else(|_| "avif,webp,jpeg".into())
                .split(',')
                .map(|s| s.trim().to_owned())
                .collect(),
            open_rps: number("OPEN_RPS", 5)?,
            open_window: number("OPEN_WINDOW", 60)?,
            trust_proxy: number("TRUST_PROXY", false)?,
        };
        ensure!(
            c.max_concurrent > 0 && c.max_requests > 0,
            "Concurrency and request limits must be positive"
        );
        ensure!(
            c.max_concurrent.checked_add(c.max_queue).is_some(),
            "MAX_QUEUE overflow"
        );
        ensure!(
            c.max_source_bytes > 0 && c.max_input_pixels > 0,
            "Source limits must be positive"
        );
        ensure!(
            c.cache_ttl > 0 && c.cache_idle > 0 && c.open_window > 0 && c.open_rps > 0,
            "TTL and rate limits must be positive"
        );
        ensure!(
            c.mp_limit.is_finite()
                && c.mp_limit > 0.0
                && c.open_max_mp.is_finite()
                && c.open_max_mp > 0.0
                && c.open_max_blur.is_finite()
                && c.open_max_blur >= 0.3,
            "Invalid image limits"
        );
        ensure!(
            !c.cdn.private || !c.signing_key.is_empty(),
            "Private mode requires SIGNING_KEY"
        );
        Ok(c)
    }
}
