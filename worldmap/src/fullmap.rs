//! The fullscreen map (spec A15, A16): the image of the stored tiles around a centre, and the view state.
use crate::sampler::map_rgba;
use crate::screen::Act;
use crate::tiles::{self, Store};
use crate::waypoints::Waypoint;
use mod_api::client::widgets::{Button, Outcome, WidgetId, Widgets};
use mod_api::Text;
use std::collections::HashSet;

/// Side limit of the host's `mod_image`.
const IMAGE_MAX: f32 = 1024.0;
/// Colour of an area nothing is known about (RGBA).
const UNKNOWN: [u8; 4] = [16, 16, 20, 255];
/// The image is made again at least this often (ticks) while the explorer fills tiles in.
const REFRESH_TICKS: u64 = 20;
/// A click this close (GUI pixels) to a waypoint's icon picks it.
const PICK_RADIUS: f32 = 6.0;
/// Tiles read from disk per frame while the view fills.
pub const LOADS_PER_FRAME: usize = 48;

/// Image pixels per GUI pixel for a screen of `gui` pixels: 1 up to the host limit, less above it.
pub fn image_factor(gui: [f32; 2]) -> f32 {
    (IMAGE_MAX / gui[0].max(gui[1]).max(1.0)).min(1.0)
}

/// RGBA of a `w` x `h` image: pixel `(w/2, h/2)` is world column `centre`, each pixel `scale` blocks.
/// Scale 1 reads the full tiles, larger scales the overview (ADR 0004).
/// Columns not stored or without a colour are `UNKNOWN`.
pub fn render(store: &Store, mip: &tiles::Mip, centre: (i32, i32), scale: i32, w: usize, h: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(w * h * 4);
    for py in 0..h {
        let z = centre.1 + (py as i32 - (h / 2) as i32) * scale;
        let mut cached: Option<(tiles::Key, Option<&tiles::Tile>, Option<&Vec<u8>>)> = None;
        for px in 0..w {
            let x = centre.0 + (px as i32 - (w / 2) as i32) * scale;
            let key = tiles::tile_of(x, z);
            let (tile, cells) = match cached {
                Some((k, t, c)) if k == key => (t, c),
                _ => {
                    let (t, c) = (store.peek(key), mip.get(&key));
                    cached = Some((key, t, c));
                    (t, c)
                }
            };
            let (color, shade) = if scale == 1 {
                tile.map_or((0, 0), |t| {
                    let c = t.get(x, z);
                    (c.color, c.shade)
                })
            } else {
                cells.map_or((0, 0), |m| {
                    let i = tiles::mip_index(x, z) * 2;
                    (m[i], m[i + 1])
                })
            };
            let rgba = tiles::brightness(shade).map(|b| map_rgba(color, b)).filter(|p| p[3] != 0).unwrap_or(UNKNOWN);
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
    centre_btn: WidgetId,
    tp_btn: WidgetId,
    /// Where a plain click chose (X, Y to land on, Z); the Teleport button works while it is set.
    pub target: Option<[i32; 3]>,
    /// Left press position while a click may still turn out to be one (no drag since).
    pub press: Option<[f32; 2]>,
    pub moved: bool,
    /// Pan left over after whole blocks were taken, in blocks.
    frac: [f32; 2],
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
    /// Tiles already asked from disk (found or not), for the full tile (true) or the overview (false).
    pub tried: HashSet<(tiles::Key, bool)>,
}

impl MapView {
    pub fn new(gui: [f32; 2], centre: (i32, i32)) -> Self {
        let mut widgets = Widgets::new();
        let close = widgets.add_button(Button::new([(gui[0] / 2.0 - 40.0).floor(), gui[1] - 28.0, 80.0, 20.0], Text::translate("worldmap.wp.close", Vec::new())));
        let f = image_factor(gui);
        let image = (((gui[0] * f) as usize).max(1), ((gui[1] * f) as usize).max(1));
        let centre_btn = widgets.add_button(Button::new([(gui[0] / 2.0 - 126.0).floor(), gui[1] - 28.0, 80.0, 20.0], Text::translate("worldmap.map.centre", Vec::new())));
        let mut tp = Button::new([(gui[0] / 2.0 + 46.0).floor(), gui[1] - 28.0, 80.0, 20.0], Text::translate("worldmap.wp.tp", Vec::new()));
        tp.active = false;
        let tp_btn = widgets.add_button(tp);
        Self { widgets, close, centre_btn, tp_btn, target: None, press: None, moved: false, frac: [0.0; 2], centre, scale: 1, gui, image, uploaded: false, last: None, tried: HashSet::new() }
    }

    /// `here` is the player's column, where the centre button goes back to.
    pub fn act(&mut self, outcome: Outcome, here: (i32, i32)) -> Act {
        if outcome == Outcome::Pressed(self.centre_btn) {
            self.centre = here;
            self.frac = [0.0; 2];
        }
        Act { close: outcome == Outcome::Pressed(self.close), teleport: self.target.filter(|_| outcome == Outcome::Pressed(self.tp_btn)), ..Act::default() }
    }

    /// Chooses (or clears) the click target and arms the Teleport button.
    pub fn set_target(&mut self, t: Option<[i32; 3]>) {
        self.target = t;
        if let Some(b) = self.widgets.button_mut(self.tp_btn) {
            b.active = t.is_some();
        }
    }

    /// Moves the view with the mouse: content follows a drag of `delta` GUI pixels.
    pub fn pan(&mut self, delta: [f32; 2]) {
        // The image is stretched over the whole screen, so a GUI pixel is `factor` image pixels.
        let per_pixel = self.scale as f32 * image_factor(self.gui);
        for (axis, d) in delta.into_iter().enumerate() {
            let blocks = self.frac[axis] - d * per_pixel;
            let whole = blocks.trunc();
            self.frac[axis] = blocks - whole;
            if axis == 0 {
                self.centre.0 += whole as i32;
            } else {
                self.centre.1 += whole as i32;
            }
        }
    }

    /// Wheel up zooms in, down zooms out; blocks a pixel stay in 1, 2, 4, 8.
    pub fn zoom(&mut self, wheel: f32) {
        if wheel > 0.0 {
            self.scale = (self.scale / 2).max(1);
        } else if wheel < 0.0 {
            self.scale = (self.scale * 2).min(8);
        }
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
        let k = 1.0 / (image_factor(self.gui) * self.scale as f32);
        [self.gui[0] / 2.0 + (x - f64::from(self.centre.0)) as f32 * k, self.gui[1] / 2.0 + (z - f64::from(self.centre.1)) as f32 * k]
    }

    /// World column under GUI position `at`; the inverse of `to_gui`.
    pub fn to_world(&self, at: [f32; 2]) -> (i32, i32) {
        let k = 1.0 / (image_factor(self.gui) * self.scale as f32);
        (self.centre.0 + ((at[0] - self.gui[0] / 2.0) / k).floor() as i32, self.centre.1 + ((at[1] - self.gui[1] / 2.0) / k).floor() as i32)
    }

    /// Index of the visible waypoint of `dimension` nearest to GUI position `at`, within `PICK_RADIUS`.
    pub fn waypoint_at(&self, wps: &[Waypoint], dimension: &str, at: [f32; 2]) -> Option<usize> {
        let d2 = |w: &Waypoint| {
            let p = self.to_gui(f64::from(w.pos[0]) + 0.5, f64::from(w.pos[2]) + 0.5);
            (p[0] - at[0]).powi(2) + (p[1] - at[1]).powi(2)
        };
        wps.iter()
            .enumerate()
            .filter(|(_, w)| w.visible && w.dimension == dimension && d2(w) <= PICK_RADIUS * PICK_RADIUS)
            .min_by(|a, b| d2(a.1).total_cmp(&d2(b.1)))
            .map(|(i, _)| i)
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

    fn mip_of(s: &Store, key: tiles::Key) -> tiles::Mip {
        std::iter::once((key, s.peek(key).unwrap().mip())).collect()
    }

    #[test]
    fn unknown_is_dark_and_a_known_column_has_its_colour() {
        // colour 1 (grass) at full brightness, one block left of the centre column
        let s = store_with(-1, 0, Column { color: 1, shade: 4, height: 70 });
        let img = render(&s, &tiles::Mip::new(), (0, 0), 1, 4, 4);
        let at = |x: usize, y: usize| img[(y * 4 + x) * 4..][..4].to_vec();
        assert_eq!(at(1, 2), vec![0x7F, 0xB2, 0x38, 255], "centre pixel (2,2) is column (0,0), so (1,2) is (-1,0)");
        assert_eq!(at(2, 2), UNKNOWN.to_vec());
        assert_eq!(img.len(), 4 * 4 * 4);
    }

    #[test]
    fn scale_skips_columns_and_colourless_columns_stay_dark() {
        let s = store_with(2, 0, Column { color: 1, shade: 4, height: 70 });
        assert_eq!(&render(&Store::new(1), &mip_of(&s, (0, 0)), (0, 0), 2, 4, 4)[(2 * 4 + 3) * 4..][..4], &[0x7F, 0xB2, 0x38, 255], "pixel (3,2) is column (2,0) at 2 blocks a pixel");
        let none = store_with(0, 0, Column { color: 0, shade: 4, height: 70 });
        assert_eq!(&render(&none, &tiles::Mip::new(), (0, 0), 1, 4, 4)[(2 * 4 + 2) * 4..][..4], &UNKNOWN);
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
    fn pan_follows_the_drag_and_zoom_stays_in_steps() {
        let mut v = MapView::new([800.0, 400.0], (100, 100));
        v.pan([10.0, -4.0]);
        assert_eq!(v.centre, (90, 104), "dragging right moves the view west");
        v.scale = 2;
        v.pan([0.25, 0.0]);
        assert_eq!(v.centre, (90, 104), "less than a block waits");
        v.pan([0.25, 0.0]);
        assert_eq!(v.centre, (89, 104), "and adds up");
        for _ in 0..5 {
            v.zoom(-1.0);
        }
        assert_eq!(v.scale, 8);
        for _ in 0..5 {
            v.zoom(1.0);
        }
        assert_eq!(v.scale, 1);
        v.zoom(0.0);
        assert_eq!(v.scale, 1);
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

    #[test]
    fn teleport_works_only_with_a_target() {
        let mut v = MapView::new([800.0, 400.0], (0, 0));
        let tp = Outcome::Pressed(v.tp_btn);
        assert!(v.act(tp, (0, 0)).teleport.is_none());
        v.set_target(Some([5, 70, -6]));
        assert!(v.widgets.button_mut(v.tp_btn).unwrap().active);
        assert_eq!(v.act(tp, (0, 0)).teleport, Some([5, 70, -6]));
        assert!(v.act(Outcome::Pressed(v.close), (0, 0)).teleport.is_none());
        v.set_target(None);
        assert!(!v.widgets.button_mut(v.tp_btn).unwrap().active);
    }

    #[test]
    fn big_screen_maps_the_stretched_image() {
        // 2560 wide: the image is 1024 px (factor 0.4) stretched over the screen, 2.5 GUI px a block.
        let mut v = MapView::new([2560.0, 1440.0], (100, 100));
        assert_eq!(v.to_world([1280.0 + 25.0, 720.0]), (110, 100));
        assert_eq!(v.to_gui(110.0, 100.0), [1305.0, 720.0]);
        v.pan([-25.0, 0.0]);
        assert_eq!(v.centre, (110, 100), "dragging 25 GUI px moves 10 blocks");
    }

    #[test]
    fn click_maps_to_the_column_and_picks_the_near_waypoint() {
        let mut v = MapView::new([800.0, 400.0], (100, 100));
        assert_eq!(v.to_world([400.0, 200.0]), (100, 100));
        assert_eq!(v.to_world([410.0, 195.0]), (110, 95));
        v.scale = 2;
        assert_eq!(v.to_world([410.0, 200.0]), (120, 100));
        let w = |x, dim: &str, visible| Waypoint { name: String::new(), pos: [x, 64, 100], dimension: dim.to_owned(), color: 0, icon: String::new(), visible };
        let list = [w(300, "a", true), w(104, "a", true), w(104, "b", true), w(104, "a", false)];
        assert_eq!(v.waypoint_at(&list, "a", [402.0, 200.0]), Some(1));
        assert_eq!(v.waypoint_at(&list, "a", [430.0, 200.0]), None, "too far");
        assert_eq!(v.waypoint_at(&list, "b", [402.0, 200.0]), Some(2), "other dimension picks only its own");
    }
}
