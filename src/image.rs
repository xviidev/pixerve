use crate::{
    config::Config,
    params::{PolicyError, Transforms},
};
use anyhow::{Result, ensure};
use bytes::Bytes;
use rs_vips::{
    VipsImage,
    enums::{BlendMode, Extend, Interesting, Interpretation, Size},
    voption::{Setter, VOption},
};

pub fn background(hex: Option<&str>, transparent: bool) -> Vec<f64> {
    let Some(hex) = hex else {
        return vec![255.0, 255.0, 255.0, if transparent { 0.0 } else { 255.0 }];
    };
    let expanded = if hex.len() <= 4 {
        hex.chars().flat_map(|c| [c, c]).collect::<String>()
    } else {
        hex.to_owned()
    };
    let mut c: Vec<f64> = expanded
        .as_bytes()
        .chunks(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap() as f64)
        .collect();
    if c.len() == 3 {
        c.push(255.0);
    }
    c
}

fn crop(image: VipsImage, w: i32, h: i32, focus: &str) -> Result<VipsImage> {
    let w = w.min(image.get_width());
    let h = h.min(image.get_height());
    if matches!(focus, "entropy" | "attention") {
        // Smart crop inspects pixels more than once. Materialize the already
        // reduced image so a sequential JPEG decoder is never read backwards.
        let image = image.copy_memory()?;
        return Ok(image.smartcrop_with_opts(
            w,
            h,
            VOption::new().set(
                "interesting",
                if focus == "entropy" {
                    Interesting::Entropy as i32
                } else {
                    Interesting::Attention as i32
                },
            ),
        )?);
    }
    let dx = image.get_width() - w;
    let dy = image.get_height() - h;
    let x = if focus.contains("left") {
        0
    } else if focus.contains("right") {
        dx
    } else {
        dx / 2
    };
    let y = if focus.contains("top") {
        0
    } else if focus.contains("bottom") {
        dy
    } else {
        dy / 2
    };
    Ok(image.extract_area(x, y, w, h)?)
}

pub fn transform(
    input: Bytes,
    t: &Transforms,
    format: &str,
    og: bool,
    c: &Config,
    watermark: Option<&Bytes>,
    open: bool,
) -> Result<Bytes> {
    // Keep the input alive throughout lazy evaluation and avoid sharing native
    // image pipelines between HTTP tasks. The operation cache is disabled.
    let header = VipsImage::new_from_buffer(&input, "")?;
    let (mut iw, mut ih) = (header.get_width(), header.get_height());
    ensure!(
        iw > 0 && ih > 0 && (iw as u64) * (ih as u64) <= c.max_input_pixels,
        "Input pixel limit exceeded"
    );
    if (5..=8).contains(&header.get_orientation()) {
        std::mem::swap(&mut iw, &mut ih);
    }
    let focus = t.fo.as_deref().unwrap_or("center");
    let fit = t.fit.as_deref().unwrap_or("cover");
    let w = t.w.map(|v| v as i32);
    let h = t.h.map(|v| v as i32);
    let mut image = if w.is_some() || h.is_some() {
        let sx = w.map(|v| f64::from(v) / f64::from(iw));
        let sy = h.map(|v| f64::from(v) / f64::from(ih));
        let scale = match (sx, sy) {
            (Some(x), Some(y)) if matches!(fit, "cover" | "outside") => x.max(y),
            (Some(x), Some(y)) => x.min(y),
            (Some(x), None) => x,
            (None, Some(y)) => y,
            _ => 1.0,
        };
        let scale = if og { scale } else { scale.min(1.0) };
        let (tw, th) = if fit == "fill" && w.is_some() && h.is_some() {
            (w.unwrap_or(iw).min(iw), h.unwrap_or(ih).min(ih))
        } else {
            (
                (f64::from(iw) * scale).round().max(1.0) as i32,
                (f64::from(ih) * scale).round().max(1.0) as i32,
            )
        };
        drop(header);
        let mut resized = VipsImage::thumbnail_buffer_with_opts(
            &input,
            tw,
            VOption::new().set("height", th).set(
                "size",
                if fit == "fill" {
                    Size::Force as i32
                } else if og {
                    Size::Both as i32
                } else {
                    Size::Down as i32
                },
            ),
        )?;
        if let (Some(w), Some(h)) = (w, h) {
            if fit == "cover" {
                resized = crop(resized, w, h, focus)?;
            }
            if fit == "contain" {
                resized = resized.colourspace(Interpretation::Srgb)?;
                if !resized.hasalpha() {
                    resized = resized.bandjoin_const(&[255.0])?;
                }
                let bg = background(t.bg.as_deref(), true);
                resized = resized.embed_with_opts(
                    (w - resized.get_width()) / 2,
                    (h - resized.get_height()) / 2,
                    w,
                    h,
                    VOption::new()
                        .set("extend", Extend::Background as i32)
                        .set("background", bg.as_slice()),
                )?;
            }
        }
        resized
    } else {
        header.autorot()?
    };
    let (ow, oh) = (image.get_width() as u64, image.get_height() as u64);
    if (ow * oh) as f64 > c.mp_limit * 1_000_000.0 {
        return Err(PolicyError::OutputPixels.into());
    }
    if open && !og {
        if ow > c.open_max_w as u64
            || oh > c.open_max_h as u64
            || (ow * oh) as f64 > c.open_max_mp * 1_000_000.0
        {
            return Err(PolicyError::OpenDimensions.into());
        }
    }
    if let Some(sigma) = t.bl {
        image = image.gaussblur(sigma)?;
    }
    if og {
        let wm_data = watermark.ok_or_else(|| anyhow::anyhow!("OG watermark unavailable"))?;
        let wm = VipsImage::new_from_buffer(wm_data, "")?;
        ensure!(
            wm.get_width() <= image.get_width() && wm.get_height() <= image.get_height(),
            "Watermark is larger than output"
        );
        image = image.composite2(&wm, BlendMode::Over)?;
        return encode(&image, t, format);
    }
    encode(&image, t, format)
}

fn encode(image: &VipsImage, t: &Transforms, format: &str) -> Result<Bytes> {
    let rgb = image.colourspace(Interpretation::Srgb)?;
    let flat;
    let image = if format == "jpeg" && rgb.hasalpha() {
        let bg = background(t.bg.as_deref(), false);
        flat = rgb.flatten_with_opts(VOption::new().set("background", &bg[..3]))?;
        &flat
    } else {
        &rgb
    };
    let q = t.q.unwrap_or(95);
    let suffix = match format {
        // Discover HEIF support before invoking its saver so a missing module
        // returns an error. Explicit AV1 compression produces AVIF output.
        "avif" => format!(".heic[compression=av1,Q={q},effort=2,bitdepth=8,strip]"),
        "webp" => format!(".webp[Q={q},strip]"),
        "png" => ".png[compression=9,strip]".into(),
        // Progressive coding and Huffman optimization work with both
        // libjpeg-turbo and mozjpeg; do not send mozjpeg-only options.
        _ => format!(".jpg[Q={q},interlace,optimize-coding,strip]"),
    };
    Ok(Bytes::from(image.write_to_buffer(&suffix)?))
}
