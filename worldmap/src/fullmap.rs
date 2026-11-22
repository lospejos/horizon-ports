//! The fullscreen map (spec A15, A16): the image of the stored tiles around a centre, and the view state.
use crate::sampler::map_rgba;
use crate::screen::Act;
use crate::tiles::{self, Store};
use mod_api::client::widgets::{Button, Outcome, WidgetId, Widgets};
use mod_api::Text;
use std::collections::HashSet;

/// Side limit of the host's `mod_image`.
const IMAGE_MAX: f32 = 1024.0;
/// Colour of an area nothing is known about (RGBA).
const UNKNOWN: [u8; 4] = [16, 16, 20, 255];
/// The image is made again at least this often (ticks) while the explorer fills tiles in.
const REFRESH_TICKS: u64 = 20;
/// Tiles read from disk per frame while the view fills.
pub const LOADS_PER_FRAME: usize = 24;

/// Image pixels per GUI pixel for a screen of `gui` pixels: 1 up to the host limit, less above it.
pub fn image_factor(gui: [f32; 2]) -> f32 {
    (IMAGE_MAX / gui[0].max(gui[1]).max(1.0)).min(1.0)
}

/// RGBA of a `w` x `h` image: pixel `(w/2, h/2)` is world column `centre`, each pixel `scale` blocks.
/// Columns not stored or without a colour are `UNKNOWN`.
pub fn render(store: &Store, centre: (i32, i32), scale: i32, w: usize, h: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(w * h * 4);
    for py in 0..h {
        let z = centre.1 + (py as i32 - (h / 2) as i32) * scale;
        let mut cached: Option<(tiles::Key, Option<&tiles::Tile>)> = None;
        for px in 0..w {
            let x = centre.0 + (px as i32 - (w / 2) as i32) * scale;
            let key = tiles::tile_of(x, z);
            let tile = match cached {
                Some((k, t)) if k == key => t,
                _ => {
                    let t = store.peek(key);
                    cached = Some((key, t));
                    t
                }
            };
            let rgba = tile
                .map(|t| t.get(x, z))
                .and_then(|c| tiles::brightness(c.shade).map(|b| map_rgba(c.color, b)))
                .filter(|p| p[3] != 0)
                .unwrap_or(UNKNOWN);
            out.extend_from_slice(&rgba);
        }
    }
    out
}

/// Tiles the image of `w` x `h` pixels at `scale` shows around `centre`, for loading from disk.
pub fn visible_tiles(centre: (i32, i32), scale: i32, w: usize, h: usize) -> Vec<tiles::Key> {
    let (x0, z0) = (centre.0 - (w / 2) as i32 * scale, centre.1 - (h / 2) as i32 * scale);
    let (x1, z1) = (x0 + w as i32 * scale - 1, z0 + h as i32 * scale - 1);
    let (a, b) = (tiles::tile_of(x0, z0), tiles::tile_of(x1, z1));
    (a.1..=b.1).flat_map(|tz| (a.0..=b.0).map(move |tx| (tx, tz))).collect()
}

pub struct MapView {
    pub widgets: Widgets,
    close: WidgetId,
    /// World column in the middle of the screen.
    pub centre: (i32, i32),
    /// Blocks per image pixel.
    pub scale: i32,
    /// GUI size of the screen the widgets were laid out for.
    pub gui: [f32; 2],
    /// Image size in pixels, and whether the host holds the image of the current view.
    pub image: (usize, usize),
    pub uploaded: bool,
    /// (centre, scale, image size, tick) of the last image.
    last: Option<((i32, i32), i32, (usize, usize), u64)>,
    /// Tiles already asked from disk (found or not).
    pub tried: HashSet<tiles::Key>,
}

impl MapView {
    pub fn new(gui: [f32; 2], centre: (i32, i32)) -> Self {
        let mut widgets = Widgets::new();
        let close = widgets.add_button(Button::new([(gui[0] / 2.0 - 40.0).floor(), gui[1] - 28.0, 80.0, 20.0], Text::translate("worldmap.wp.close", Vec::new())));
        let f = image_factor(gui);
        let image = (((gui[0] * f) as usize).max(1), ((gui[1] * f) as usize).max(1));
        Self { widgets, close, centre, scale: 1, gui, image, uploaded: false, last: None, tried: HashSet::new() }
    }

    pub fn act(&mut self, outcome: Outcome) -> Act {
        Act { close: outcome == Outcome::Pressed(self.close), ..Act::default() }
    }

    /// True when the image no longer matches the view, or is old while tiles may be filling in.
    pub fn stale(&self, now: u64) -> bool {
        self.last.is_none_or(|(c, s, i, t)| (c, s, i) != (self.centre, self.scale, self.image) || now.saturating_sub(t) >= REFRESH_TICKS)
    }

    /// Forces the image to be made again on the next tick.
    pub fn invalidate(&mut self) {
        self.last = None;
    }

    pub fn made(&mut self, now: u64) {
        self.last = Some((self.centre, self.scale, self.image, now));
        self.uploaded = true;
    }

    /// GUI position of world point `(x, z)`.
    pub fn to_gui(&self, x: f64, z: f64) -> [f32; 2] {
        let f = image_factor(self.gui);
        let k = f / self.scale as f32;
        [self.gui[0] / 2.0 + (x - f64::from(self.centre.0)) as f32 * k, self.gui[1] / 2.0 + (z - f64::from(self.centre.1)) as f32 * k]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tiles::{Column, Tile};

    fn store_with(x: i32, z: i32, c: Column) -> Store {
        let mut s = Store::new(8);
        let mut t = Tile::default();
        t.set(x, z, c);
        s.insert(tiles::tile_of(x, z), t);
        s
    }

    #[test]
    fn unknown_is_dark_and_a_known_column_has_its_colour() {
        // colour 1 (grass) at full brightness, one block left of the centre column
        let s = store_with(-1, 0, Column { color: 1, shade: 4, height: 70 });
        let img = render(&s, (0, 0), 1, 4, 4);
        let at = |x: usize, y: usize| img[(y * 4 + x) * 4..][..4].to_vec();
        assert_eq!(at(1, 2), vec![0x7F, 0xB2, 0x38, 255], "centre pixel (2,2) is column (0,0), so (1,2) is (-1,0)");
        assert_eq!(at(2, 2), UNKNOWN.to_vec());
        assert_eq!(img.len(), 4 * 4 * 4);
    }

    #[test]
    fn scale_skips_columns_and_colourless_columns_stay_dark() {
        let s = store_with(2, 0, Column { color: 1, shade: 4, height: 70 });
        assert_eq!(&render(&s, (0, 0), 2, 4, 4)[(2 * 4 + 3) * 4..][..4], &[0x7F, 0xB2, 0x38, 255], "pixel (3,2) is column (2,0) at 2 blocks a pixel");
        let none = store_with(0, 0, Column { color: 0, shade: 4, height: 70 });
        assert_eq!(&render(&none, (0, 0), 1, 4, 4)[(2 * 4 + 2) * 4..][..4], &UNKNOWN);
    }

    #[test]
    fn visible_tiles_cover_the_image() {
        let v = visible_tiles((0, 0), 1, 128, 64);
        assert_eq!(v, vec![(-1, -1), (0, -1), (-1, 0), (0, 0)]);
        assert_eq!(visible_tiles((10, 10), 1, 4, 4), vec![(0, 0)]);
    }

    #[test]
    fn image_stays_within_the_host_limit() {
        assert_eq!(image_factor([853.0, 480.0]), 1.0);
        let f = image_factor([2560.0, 1440.0]);
        assert!(2560.0 * f <= IMAGE_MAX);
    }

    #[test]
    fn stale_after_a_move_or_age() {
        let mut v = MapView::new([800.0, 400.0], (0, 0));
        assert!(v.stale(0));
        v.made(5);
        assert!(!v.stale(6));
        assert!(v.stale(5 + REFRESH_TICKS));
        v.centre = (3, 0);
        assert!(v.stale(6));
        assert_eq!(v.to_gui(3.0, 0.0), [400.0, 200.0]);
    }
}
