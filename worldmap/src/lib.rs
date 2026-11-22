//! Worldmap for Horizon: a minimap drawn over the GUI.
mod sampler;
use mod_api::*;
use sampler::{map_rgba, slope_brightness, water_brightness, Scan, WATER};
use std::collections::HashMap;
use std::sync::Mutex;

/// Sampled grid side in cells; the drawn size and blocks per cell are settings.
const SIDE: u32 = 128;
/// Columns read per tick (spec A6).
const BUDGET: u32 = 4096;
/// The HUD atlas slot `native_mod_image` fills.
/// Gap to the screen edge in GUI pixels.
const MARGIN: f32 = 4.0;
const SPRITE: &str = "horizon/modimage";

struct State {
    scan: Scan,
    rgba: Vec<u8>,
    /// Surface level per grid index (water top where wet), for slope shading against the north column.
    heights: Vec<Option<i32>>,
    /// Block id to map colour id, filled on first sight (a host call each).
    colors: HashMap<String, u8>,
    /// Centre of the pass being sampled, latched when it starts.
    centre: (i32, i32),
    /// Blocks per cell and shape of the pass being sampled, latched when it starts.
    scale: i32,
    round: bool,
    /// True once a finished pass was uploaded.
    ready: bool,
}

struct Mod(Mutex<State>);

impl Default for Mod {
    fn default() -> Self {
        Self(Mutex::new(State {
            scan: Scan::new(SIDE),
            rgba: vec![0; (SIDE * SIDE * 4) as usize],
            heights: vec![None; (SIDE * SIDE) as usize],
            colors: HashMap::new(),
            centre: (0, 0),
            scale: 1,
            round: false,
            ready: false,
        }))
    }
}

fn lock(m: &Mutex<State>) -> std::sync::MutexGuard<'_, State> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn client_tick_post(&self, host: &mut dyn ClientHost, _: &mut ClientTickPostEvent) {
        if host.setting_bool("enabled") == Some(false) {
            return;
        }
        let Some(pos) = host.camera().map(|c| c.position) else { return };
        let mut s = lock(&self.0);
        let range = s.scan.batch(BUDGET);
        if range.start == 0 {
            s.centre = (pos[0].floor() as i32, pos[2].floor() as i32);
            s.scale = 1 << host.setting_choice("scale").unwrap_or(0).min(2);
            s.round = host.setting_choice("shape") == Some(1);
        }
        let cols: Vec<(i32, i32)> = range.clone().map(|i| s.scan.column(i, s.centre.0, s.centre.1, s.scale)).collect();
        let Ok(found) = host.client_far_surface(&cols) else { return };
        for (i, col) in range.clone().zip(found) {
            let px = match col {
                Some(c) => {
                    let id = match s.colors.get(&c.block) {
                        Some(&id) => id,
                        None => {
                            let id = host.block_state_id(&c.block).and_then(|b| host.block_states().map_color(b)).unwrap_or(0);
                            s.colors.insert(c.block, id);
                            id
                        }
                    };
                    match c.water {
                        Some(w) => {
                            s.heights[i as usize] = Some(w);
                            map_rgba(WATER, water_brightness(w - c.height))
                        }
                        None => {
                            s.heights[i as usize] = Some(c.height);
                            // Row 0 and an unread north neighbour stay flat.
                            let north = i.checked_sub(SIDE).and_then(|n| s.heights[n as usize]);
                            map_rgba(id, slope_brightness(c.height, north))
                        }
                    }
                }
                None => {
                    s.heights[i as usize] = None;
                    [0; 4]
                }
            };
            let px = if s.round && !s.scan.in_circle(i) { [0; 4] } else { px };
            let at = i as usize * 4;
            s.rgba[at..at + 4].copy_from_slice(&px);
        }
        if range.end == SIDE * SIDE && host.mod_image(SIDE, SIDE, &s.rgba) {
            s.ready = true;
        }
    }

    fn register(&self, registrar: &mut dyn Registrar, _: &RegistrationContext) {
        let settings = [
            SettingSpec::toggle("enabled", "worldmap.setting.enabled", true),
            SettingSpec::choice(
                "corner",
                "worldmap.setting.corner",
                ["worldmap.corner.top_right", "worldmap.corner.top_left", "worldmap.corner.bottom_right", "worldmap.corner.bottom_left"],
                0,
            ),
            SettingSpec::range("size", "worldmap.setting.size", 64.0, 256.0, 16.0, 128.0),
            SettingSpec::choice("scale", "worldmap.setting.scale", ["worldmap.scale.1", "worldmap.scale.2", "worldmap.scale.4"], 0),
            SettingSpec::choice("shape", "worldmap.setting.shape", ["worldmap.shape.square", "worldmap.shape.round"], 0),
        ];
        for s in settings {
            if let Err(e) = registrar.register_setting(s) {
                eprintln!("[worldmap] setting refused: {e:?}");
            }
        }
    }

    fn gui_render_post(&self, host: &dyn ClientHost, event: &mut GuiRenderPostEvent<'_>) {
        if !lock(&self.0).ready || host.setting_bool("enabled") == Some(false) {
            return;
        }
        let [w, h] = event.batch.size();
        let side = host.setting_f64("size").unwrap_or(128.0).clamp(64.0, 256.0) as f32;
        let corner = host.setting_choice("corner").unwrap_or(0);
        let x = if corner % 2 == 0 { w - side - MARGIN } else { MARGIN };
        let y = if corner < 2 { MARGIN } else { h - side - MARGIN };
        event.batch.sprite(SPRITE, [0.0, 0.0, SIDE as f32, SIDE as f32], [x, y, side, side], None);
    }
}
