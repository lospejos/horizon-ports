//! Worldmap for Horizon: a minimap drawn over the GUI. T1 scaffold: one test rectangle.
mod sampler;
use mod_api::{ClientHost, GuiRenderPostEvent, Hooks};

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn gui_render_post(&self, _: &dyn ClientHost, event: &mut GuiRenderPostEvent<'_>) {
        event.batch.fill([4.0, 4.0, 64.0, 64.0], 0xA0_20_80_20);
    }
}
