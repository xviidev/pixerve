use crate::config::Config;
use anyhow::{Result, bail, ensure};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Transforms {
    pub w: Option<u32>,
    pub h: Option<u32>,
    pub q: Option<u32>,
    pub bl: Option<f64>,
    pub bg: Option<String>,
    pub f: Option<String>,
    pub fit: Option<String>,
    pub fo: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub enum PolicyError {
    OutputPixels,
    OpenDimensions,
    OpenBlur,
    OpenFormat,
}

impl PolicyError {
    pub fn message(self) -> &'static str {
        match self {
            Self::OutputPixels => "Output pixel limit exceeded",
            Self::OpenDimensions => "Unsigned image dimensions exceed configured limits",
            Self::OpenBlur => "Unsigned image blur exceeds configured limit",
            Self::OpenFormat => "Output format is not allowed for unsigned requests",
        }
    }
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for PolicyError {}

pub const KEYS: [&str; 8] = ["w", "h", "q", "bl", "bg", "f", "fit", "fo"];
pub const PRESETS: [&str; 5] = ["thumbnail", "card", "article", "wide", "og"];

#[cfg(test)]
pub fn token_map(token: &str) -> HashMap<String, String> {
    token
        .split(',')
        .filter_map(|s| s.split_once('-'))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
}

pub fn checked_token_map(token: &str) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    for segment in token.split(',') {
        let (key, value) = segment
            .split_once('-')
            .ok_or_else(|| anyhow::anyhow!("Invalid token"))?;
        ensure!(!key.is_empty() && !value.is_empty(), "Invalid token");
        ensure!(
            map.insert(key.to_owned(), value.to_owned()).is_none(),
            "Duplicate token parameter"
        );
    }
    Ok(map)
}

impl Transforms {
    pub fn preset(name: &str) -> Result<Self> {
        let mut t = Self {
            q: Some(75),
            f: Some("webp".into()),
            fit: Some("cover".into()),
            ..Default::default()
        };
        match name {
            "thumbnail" => {
                t.w = Some(320);
                t.h = Some(180);
            }
            "card" => {
                t.w = Some(640);
                t.h = Some(360);
            }
            "article" => {
                t.w = Some(960);
                t.q = Some(80);
                t.fit = Some("inside".into());
            }
            "wide" => {
                t.w = Some(1280);
                t.q = Some(80);
                t.fit = Some("inside".into());
            }
            "og" => {
                t.w = Some(1200);
                t.h = Some(630);
                t.q = Some(70);
                t.f = Some("jpeg".into());
                t.fo = Some("entropy".into());
            }
            _ => bail!("Unknown preset"),
        }
        Ok(t)
    }

    pub fn is_preset(&self) -> bool {
        PRESETS.iter().any(|name| {
            let mut preset = Self::preset(name).unwrap();
            preset.f = self.f.clone();
            self.normalize() == preset.normalize()
        })
    }

    pub fn cache_normalized(&self, format: &str) -> Self {
        let mut t = self.clone();
        t.q = Some(t.q.unwrap_or(95));
        t.fit = Some(t.fit.unwrap_or_else(|| "cover".into()));
        t.fo = Some(t.fo.unwrap_or_else(|| "center".into()));
        t.f = Some(format.into());
        if let Some(bg) = t.bg.take() {
            let bg = bg.to_ascii_lowercase();
            t.bg = Some(if bg.len() <= 4 {
                bg.chars().flat_map(|c| [c, c]).collect()
            } else {
                bg
            });
        }
        t
    }

    pub fn parse(map: &HashMap<String, String>) -> Result<Self> {
        let mut expanded = HashMap::new();
        if let Some(name) = map.get("p") {
            let preset = Self::preset(name)?;
            for pair in preset.normalize().split(',') {
                if let Some((k, v)) = pair.split_once('=') {
                    expanded.insert(k.to_owned(), v.to_owned());
                }
            }
        }
        expanded.extend(map.clone());
        let map = &expanded;
        let numeric = |key: &str| -> Result<Option<u32>> {
            map.get(key)
                .map(|s| s.parse().map_err(Into::into))
                .transpose()
        };
        let t = Self {
            w: numeric("w")?,
            h: numeric("h")?,
            q: numeric("q")?,
            bl: map.get("bl").map(|s| s.parse()).transpose()?,
            bg: map.get("bg").cloned(),
            f: map.get("f").cloned(),
            fit: map.get("fit").cloned(),
            fo: map.get("fo").cloned(),
        };
        ensure!(
            t.w.is_none_or(|v| (1..=2400).contains(&v))
                && t.h.is_none_or(|v| (1..=2400).contains(&v)),
            "Invalid dimensions"
        );
        ensure!(
            t.q.is_none_or(|v| (1..=100).contains(&v)),
            "Invalid quality"
        );
        ensure!(
            t.bl.is_none_or(|v| v.is_finite() && (0.3..=100.0).contains(&v)),
            "Invalid blur"
        );
        if let Some(bg) = &t.bg {
            ensure!(
                [3, 4, 6, 8].contains(&bg.len()) && bg.bytes().all(|c| c.is_ascii_hexdigit()),
                "Invalid background"
            );
        }
        if let Some(f) = &t.f {
            ensure!(
                ["auto", "jpeg", "png", "webp", "avif"].contains(&f.as_str()),
                "Invalid format"
            );
        }
        if let Some(fit) = &t.fit {
            ensure!(
                ["cover", "contain", "fill", "inside", "outside"].contains(&fit.as_str()),
                "Invalid fit"
            );
        }
        if let Some(fo) = &t.fo {
            ensure!(
                [
                    "center",
                    "top",
                    "left",
                    "bottom",
                    "right",
                    "top_left",
                    "top_right",
                    "bottom_left",
                    "bottom_right",
                    "entropy",
                    "attention"
                ]
                .contains(&fo.as_str()),
                "Invalid focus"
            );
        }
        Ok(t)
    }

    pub fn normalize(&self) -> String {
        let values = [
            self.w.map(|v| v.to_string()),
            self.h.map(|v| v.to_string()),
            self.q.map(|v| v.to_string()),
            self.bl.map(|v| v.to_string()),
            self.bg.clone(),
            self.f.clone(),
            self.fit.clone(),
            self.fo.clone(),
        ];
        KEYS.into_iter()
            .zip(values)
            .filter_map(|(k, v)| v.map(|v| format!("{k}={v}")))
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn format(&self, accept: &str) -> String {
        if let Some(f) = &self.f {
            if f != "auto" {
                return f.clone();
            }
        }
        // Respect explicit q=0 instead of caching a format the client rejected.
        let accepts = |mime: &str| {
            accept.split(',').any(|item| {
                let mut fields = item.trim().split(';');
                fields.next().is_some_and(|v| v.eq_ignore_ascii_case(mime))
                    && !fields.any(|p| {
                        p.trim()
                            .strip_prefix("q=")
                            .and_then(|v| v.parse::<f32>().ok())
                            .is_some_and(|q| q <= 0.0)
                    })
            })
        };
        if accepts("image/avif") {
            "avif"
        } else if accepts("image/webp") {
            "webp"
        } else {
            "jpeg"
        }
        .into()
    }

    pub fn policy(
        &self,
        c: &Config,
        format: &str,
        open: bool,
    ) -> std::result::Result<(), PolicyError> {
        let mp = f64::from(self.w.unwrap_or(0)) * f64::from(self.h.unwrap_or(0)) / 1_000_000.0;
        if mp > c.mp_limit {
            return Err(PolicyError::OutputPixels);
        }
        if open {
            if self.w.is_some_and(|v| v > c.open_max_w)
                || self.h.is_some_and(|v| v > c.open_max_h)
                || mp > c.open_max_mp
            {
                return Err(PolicyError::OpenDimensions);
            }
            if self.bl.is_some_and(|v| v > c.open_max_blur) {
                return Err(PolicyError::OpenBlur);
            }
            if !c.open_formats.iter().any(|s| s == format) {
                return Err(PolicyError::OpenFormat);
            }
        }
        Ok(())
    }
}

/// Constant-time MAC verification without comparing bearer tokens directly.
pub fn verify_bearer(expected: &str, provided: &str) -> bool {
    let key = b"pixerve metrics bearer comparison";
    let mut mac = Hmac::<Sha256>::new_from_slice(key).unwrap();
    mac.update(expected.as_bytes());
    let expected = mac.finalize().into_bytes();
    let mut actual = Hmac::<Sha256>::new_from_slice(key).unwrap();
    actual.update(provided.as_bytes());
    actual.verify_slice(&expected).is_ok()
}

pub fn signature(secret: &str, path: &str, t: &Transforms, expiry: u64) -> String {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key size");
    mac.update(format!("{path}|{}|{expiry}", t.normalize()).as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

pub fn asset_signature(secret: &str, path: &str, t: &Transforms, pipeline: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(
        format!(
            "pixerve-public-asset-v1|{path}|{}|{pipeline}",
            t.normalize()
        )
        .as_bytes(),
    );
    hex::encode(mac.finalize().into_bytes())
}

pub fn verify_asset(
    secret: &str,
    path: &str,
    t: &Transforms,
    pipeline: &str,
    sig: &str,
) -> Result<()> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())?;
    mac.update(
        format!(
            "pixerve-public-asset-v1|{path}|{}|{pipeline}",
            t.normalize()
        )
        .as_bytes(),
    );
    mac.verify_slice(&hex::decode(sig)?)?;
    Ok(())
}

pub fn verify(
    secret: &str,
    path: &str,
    t: &Transforms,
    auth: &HashMap<String, String>,
    now: u64,
) -> Result<()> {
    let expiry = auth
        .get("exp")
        .or_else(|| auth.get("e"))
        .and_then(|v| v.parse::<u64>().ok())
        .ok_or_else(|| anyhow::anyhow!("Missing expiry"))?;
    if expiry < now {
        bail!("Expired");
    }
    let sig = hex::decode(
        auth.get("sig")
            .or_else(|| auth.get("s"))
            .ok_or_else(|| anyhow::anyhow!("Missing signature"))?,
    )?;
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())?;
    mac.update(format!("{path}|{}|{expiry}", t.normalize()).as_bytes());
    mac.verify_slice(&sig)?;
    Ok(())
}

pub fn validate_path(path: &str) -> Result<()> {
    ensure!(
        !path.is_empty()
            && !path.starts_with('/')
            && !path.contains(['\\', '\0', ':'])
            && !path.chars().any(char::is_control),
        "Invalid path"
    );
    ensure!(
        path.split('/')
            .all(|s| !s.is_empty() && s != "." && s != ".."),
        "Invalid path"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn presets_and_canonical_cache_defaults_preserve_signing_input() {
        let preset = Transforms::parse(&HashMap::from([("p".into(), "card".into())])).unwrap();
        assert!(preset.is_preset());
        assert_eq!(preset.normalize(), "w=640,h=360,q=75,f=webp,fit=cover");
        let a = Transforms::parse(&token_map("w-320,f-webp")).unwrap();
        let b = Transforms::parse(&token_map("w-320,q-95,f-webp,fit-cover,fo-center")).unwrap();
        assert_eq!(
            a.cache_normalized("webp").normalize(),
            b.cache_normalized("webp").normalize()
        );
        assert_ne!(a.normalize(), b.normalize());
        assert!(checked_token_map("f-webp,f-auto").is_err());
        assert!(verify_bearer("secret", "secret"));
        assert!(!verify_bearer("secret", "wrong"));
    }
    #[test]
    fn stable_asset_signatures_bind_the_pipeline_path_and_transform() {
        let t = Transforms::preset("card").unwrap();
        let sig = asset_signature("secret", "news/a.jpg", &t, "1");
        assert!(verify_asset("secret", "news/a.jpg", &t, "1", &sig).is_ok());
        assert!(verify_asset("secret", "news/a.jpg", &t, "2", &sig).is_err());
        assert!(verify_asset("secret", "news/b.jpg", &t, "1", &sig).is_err());
        assert!(
            verify_asset(
                "secret",
                "news/a.jpg",
                &Transforms::preset("article").unwrap(),
                "1",
                &sig
            )
            .is_err()
        );
    }
    #[test]
    fn signatures_keep_bun_parameter_order() {
        let t = Transforms::parse(&token_map("h-600,w-800,q-70,bl-0.3,fit-cover,fo-attention"))
            .unwrap();
        assert_eq!(
            t.normalize(),
            "w=800,h=600,q=70,bl=0.3,fit=cover,fo=attention"
        );
        let mut auth = HashMap::from([
            ("exp".into(), "2000000000".into()),
            (
                "sig".into(),
                signature("secret", "images/a.jpg", &t, 2000000000),
            ),
        ]);
        assert!(verify("secret", "images/a.jpg", &t, &auth, 100).is_ok());
        assert!(verify("secret", "images/b.jpg", &t, &auth, 100).is_err());
        auth.insert("exp".into(), "1".into());
        assert!(verify("secret", "images/a.jpg", &t, &auth, 100).is_err());
    }
    #[test]
    fn rejects_bad_input_and_negotiates() {
        for token in ["w-0", "q-101", "bl-NaN", "fit-nope", "bg-fffff"] {
            assert!(Transforms::parse(&token_map(token)).is_err());
        }
        let t = Transforms::default();
        assert_eq!(t.format("image/avif;q=0, image/webp"), "webp");
        for path in ["../secret", "a/../b", "a\\b", "/a", "a//b", "C:secret"] {
            assert!(validate_path(path).is_err());
        }
    }
}
