use anyhow::{Context, Result, ensure};
use axum::http::StatusCode;
use bytes::Bytes;
use rs_vips::{
    VipsImage,
    enums::Interpretation,
    voption::{Setter, VOption},
};
use std::collections::HashMap;

pub const FAVICON: &[u8] = include_bytes!("../assets/favicon.ico");
const WIDTH: i32 = 1672;
const HEIGHT: i32 = 940;

// A finite set of pre-rendered responses. Requests never enter the native pipeline.
const MESSAGES: &[(u16, &str)] = &[
    (400, "Invalid image request"),
    (403, "Access denied"),
    (404, "Image unavailable"),
    (405, "Method not allowed"),
    (409, "Source fingerprint mismatch"),
    (410, "Image version unavailable"),
    (413, "Image limit exceeded"),
    (422, "Unable to process image"),
    (429, "Too many requests"),
    (500, "Unable to process image"),
    (502, "Image source unavailable"),
    (503, "Service busy"),
    (504, "Image source timeout"),
];

pub struct ErrorImages {
    images: HashMap<u16, Bytes>,
}

impl ErrorImages {
    pub fn load() -> Result<Self> {
        let mut images = HashMap::new();
        let base = background().context("Unable to prepare error artwork")?;
        for &(status, message) in MESSAGES {
            let bytes = render(&base, status, message)
                .with_context(|| format!("Unable to render HTTP {status} error image"))?;
            images.insert(status, bytes);
        }
        Ok(Self { images })
    }

    pub fn get(&self, status: StatusCode) -> Bytes {
        self.images
            .get(&status.as_u16())
            .unwrap_or(&self.images[&500])
            .clone()
    }
}

fn artwork(svg: &[u8], width: i32) -> Result<VipsImage> {
    let image = VipsImage::svgload_buffer(svg)?
        .flatten_with_opts(VOption::new().set("background", &[255.0, 255.0, 255.0][..]))?;
    let (left, top, w, h) = image.find_trim()?;
    ensure!(w > 0 && h > 0, "Empty error artwork");
    let cropped = image.extract_area(left, top, w, h)?;
    Ok(cropped.resize(f64::from(width) / f64::from(w))?)
}

fn background() -> Result<VipsImage> {
    let canvas =
        VipsImage::new_from_image(&VipsImage::black(WIDTH, HEIGHT)?, &[255.0, 255.0, 255.0])?
            .copy_with_opts(VOption::new().set("interpretation", Interpretation::Srgb as i32))?;
    let icon = artwork(include_bytes!("../assets/error.svg"), 380)?;
    let logo = artwork(include_bytes!("../assets/logo.svg"), 310)?;
    // Align the icon's main frame with the text; its detached pixels extend right.
    let canvas = canvas.insert(&icon, (WIDTH - icon.get_width()) / 2 + 48, 218)?;
    Ok(canvas
        .insert(
            &logo,
            (WIDTH - logo.get_width()) / 2,
            HEIGHT - logo.get_height() - 96,
        )?
        .copy_memory()?)
}

fn text(message: &str, font: &str) -> Result<VipsImage> {
    Ok(VipsImage::text_with_opts(
        message,
        VOption::new()
            .set("font", font)
            .set("width", 1240)
            .set("dpi", 72)
            .set("rgba", true),
    )?
    .flatten_with_opts(VOption::new().set("background", &[255.0, 255.0, 255.0][..]))?)
}

fn render(base: &VipsImage, status: u16, message: &str) -> Result<Bytes> {
    let title = text(message, "monospace bold 64")?;
    let caption = text(
        &format!("<span foreground=\"#71717a\">HTTP {status}</span>"),
        "monospace 24",
    )?;
    let image = base
        .insert(&title, (WIDTH - title.get_width()) / 2, 555)?
        .insert(&caption, (WIDTH - caption.get_width()) / 2, 660)?;
    Ok(Bytes::from(image.pngsave_buffer_with_opts(
        VOption::new().set("compression", 6).set("strip", true),
    )?))
}
