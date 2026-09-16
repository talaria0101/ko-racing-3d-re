//! Sky, horizon and distance: the `.bck` background resource (`al.q(int)`).
//!
//! Each track picks one of five backgrounds by its theme byte - `al.q(j)` is
//! called with the theme while the race is set up - and the file names an image
//! under `/images/`, tried as `.jpg` first and falling back to `.png`, which is
//! exactly what the reader does.
//!
//! The images are 256x128 strips: sky at the top, the horizon across the
//! middle, terrain below.  M3G draws a 2D background image scaled to the
//! viewport, so this stretches the strip to the screen and draws the scene over
//! it.  The four packed colours are parsed and kept for the record, but the
//! strip is opaque, so only the fourth is used - as the clear colour, in case
//! the viewport is wider than the strip after scaling.

use macroquad::prelude::*;
use macroquad::texture::Texture2D;

use crate::format;
use crate::pack::Resources;

/// The five backgrounds, in the order `al.a` lists them.  The theme byte of a
/// race record indexes this.
pub const THEMES: [&str; 5] = ["clear", "rain", "snow", "desert", "sunset"];

/// Where a backdrop strip lands on screen: fit to the full width keeping
/// its aspect, top-aligned.  Stretching the 256x128 strip fullscreen puts
/// its sun (at 0.52 height in `bss.png`) mid-screen, where the track hills
/// cover it, leaving flat grey; the original shows the sun up at ~0.3 with
/// the strip's dark mountains overlapping the 3D hills.  Pure helper so
/// tests can pin the mapping: returns the drawn `(width, height)`.
pub fn sky_rect(screen_width: f32, _screen_height: f32, image_width: f32, image_height: f32) -> (f32, f32) {
    if image_width <= 0.0 || image_height <= 0.0 {
        return (screen_width, _screen_height);
    }
    (screen_width, screen_width * image_height / image_width)
}

/// `KORA_SKYFIT=stretch` restores the old fullscreen stretch.
pub fn sky_stretch() -> bool {
    std::env::var("KORA_SKYFIT").is_ok_and(|value| value == "stretch")
}

pub struct Sky {
    texture: Option<Texture2D>,
    clear: Color,
}

impl Sky {
    pub fn load(resources: &Resources, theme: u8) -> Option<Sky> {
        let name = THEMES.get(theme as usize).copied().unwrap_or(THEMES[0]);
        let background =
            format::Background::parse(resources.get(&format!("back/{name}.bck"))?)?;
        let clear = background.colours[3];
        Some(Sky {
            texture: load_image(resources, &background.texture),
            clear: Color::from_rgba(
                (clear >> 16) as u8,
                (clear >> 8) as u8,
                clear as u8,
                255,
            ),
        })
    }

    /// Paint the background.  Call this before setting the 3D camera, so the
    /// scene is drawn over it.
    pub fn draw(&self) {
        clear_background(self.clear);
        if let Some(texture) = &self.texture {
            let dest_size = if sky_stretch() {
                vec2(screen_width(), screen_height())
            } else {
                let (w, h) = sky_rect(
                    screen_width(),
                    screen_height(),
                    texture.width(),
                    texture.height(),
                );
                vec2(w, h)
            };
            draw_texture_ex(
                texture,
                0.0,
                0.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(dest_size),
                    ..Default::default()
                },
            );
        }
    }
}

/// The MIDlet's own rule: `/images/<stem>.jpg` first, then `<stem>.png`.
fn load_image(resources: &Resources, name: &str) -> Option<Texture2D> {
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    for candidate in [
        format!("images/{stem}.jpg"),
        format!("images/{stem}.png"),
    ] {
        if let Some(bytes) = resources.get(&candidate) {
            let texture = Texture2D::from_file_with_format(bytes, None);
            texture.set_filter(FilterMode::Linear);
            return Some(texture);
        }
    }
    None
}
