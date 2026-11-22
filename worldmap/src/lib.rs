//! Worldmap for Horizon: a minimap drawn over the GUI.
mod sampler;
use mod_api::*;
use sampler::{map_rgba, Scan};
use std::collections::HashMap;
use std::sync::Mutex;

/// Map side in pixels (one block per pixel); settings come in T5.
const SIDE: u32 = 128;
/// Columns read per tick (spec A6).
const BUDGET: u32 = 4096;
/// Vanilla map brightness of a flat column.
const FLAT: u32 = 220;
/// The HUD atlas slot `native_mod_image` fills.
const SPRITE: &str = "horizon/modimage";

struct State {
    scan: Scan,
    rgba: Vec<u8>,
    /// Block id to map colour id, filled on first sight (a host call each).
    colors: HashMap<String, u8>,
    /// Centre of the pass being sampled, latched when it starts.
    centre: (i32, i32),
    /// True once a finished pass was uploaded.
    ready: bool,
}

struct Mod(Mutex<State>);

impl Default for Mod {
    fn default() -> Self {
        Self(Mutex::new(State {
            scan: Scan::new(SIDE),
            rgba: vec![0; (SIDE * SIDE * 4) as usize],
            colors: HashMap::new(),
            centre: (0, 0),
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
        let Some(pos) = host.camera().map(|c| c.position) else { return };
        let mut s = lock(&self.0);
        let range = s.scan.batch(BUDGET);
        if range.start == 0 {
            s.centre = (pos[0].floor() as i32, pos[2].floor() as i32);
        }
        let cols: Vec<(i32, i32)> = range.clone().map(|i| s.scan.column(i, s.centre.0, s.centre.1)).collect();
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
                    map_rgba(id, FLAT)
                }
                None => [0; 4],
            };
            let at = i as usize * 4;
            s.rgba[at..at + 4].copy_from_slice(&px);
        }
        if range.end == SIDE * SIDE && host.mod_image(SIDE, SIDE, &s.rgba) {
            s.ready = true;
        }
    }

    fn gui_render_post(&self, _: &dyn ClientHost, event: &mut GuiRenderPostEvent<'_>) {
        if !lock(&self.0).ready {
            return;
        }
        let [w, _] = event.batch.size();
        let side = SIDE as f32;
        event.batch.sprite(SPRITE, [0.0, 0.0, side, side], [w - side - 4.0, 4.0, side, side], None);
    }
}
