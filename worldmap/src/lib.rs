//! Worldmap for Horizon: a minimap drawn over the GUI.
mod icons;
mod sampler;
mod screen;
mod waypoints;
use mod_api::*;
use sampler::{clamp_to_map, clock, map_rgba, pretty_id, marker_cell, slope_brightness, water_brightness, Scan, WATER};
use mod_api::client::widgets::Outcome;
use waypoints::Waypoints;
use std::collections::HashMap;
use std::sync::Mutex;

/// Sampled grid side in cells; the drawn size and blocks per cell are settings.
const SIDE: u32 = 128;
/// Columns read per tick (spec A6).
const BUDGET: u32 = 4096;
/// The HUD atlas slot `native_mod_image` fills.
/// Default gap to the screen edge in GUI pixels (the offset settings' default).
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
    /// Waypoints of the world in `world` (the key their file is named by).
    waypoints: Waypoints,
    world: Option<String>,
    /// The waypoint screen while it is open.
    screen: Option<screen::Screen>,
    /// A button asked to close the screen; the next client tick does it (input hooks only read).
    want_close: bool,
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
            waypoints: Waypoints::default(),
            world: None,
            screen: None,
            want_close: false,
        }))
    }
}

fn lock(m: &Mutex<State>) -> std::sync::MutexGuard<'_, State> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// This mod's id: names its screen and its key mapping.
const ID: &str = "worldmap";

/// Writes the world's waypoint file; a failure is logged and the list stays in memory.
fn save_waypoints(host: &dyn ClientHost, s: &State) {
    let Some(key) = s.world.as_deref() else { return };
    let Ok(path) = ConfigPath::new(&waypoints::file_name(key)) else { return };
    if let Err(e) = host.data_files().write(&path, s.waypoints.serialize().as_bytes()) {
        eprintln!("[worldmap] waypoints not saved: {e:?}");
    }
}

/// Runs `input` on the open waypoint screen's widgets, acts on the answer, and says whether the game should drop the input.
fn screen_input(m: &Mutex<State>, host: &dyn ClientHost, input: impl FnOnce(&mut mod_api::client::widgets::Widgets) -> Outcome) -> Flow {
    if host.mod_screen() != Some(ID) {
        return Flow::Continue;
    }
    let mut guard = lock(m);
    let s = &mut *guard;
    let Some(screen) = s.screen.as_mut() else { return Flow::Continue };
    let outcome = input(&mut screen.widgets);
    let act = screen.act(outcome, &mut s.waypoints);
    screen.refresh();
    if act.save {
        save_waypoints(host, s);
    }
    s.want_close |= act.close;
    if outcome.handled() { Flow::Cancel } else { Flow::Continue }
}

/// The key of the world the player is in: its save folder, else the server address.
fn world_key(host: &dyn ClientHost) -> Option<String> {
    let server = host.client_session().client_server()?;
    server.save_folder.or(server.address)
}

/// Loads the waypoint file of the current world when the world changed; a missing or broken file gives an empty list.
fn load_waypoints(host: &dyn ClientHost, s: &mut State) {
    let key = world_key(host);
    if key == s.world {
        return;
    }
    s.waypoints = key
        .as_deref()
        .and_then(|k| ConfigPath::new(&waypoints::file_name(k)).ok())
        .and_then(|path| host.data_files().read(&path).ok())
        .and_then(|bytes| Waypoints::parse(&String::from_utf8_lossy(&bytes)))
        .unwrap_or_default();
    s.world = key;
}

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn client_tick_pre(&self, host: &mut dyn ClientHost, _: &mut ClientTickPreEvent) {
        let mut s = lock(&self.0);
        if std::mem::take(&mut s.want_close) {
            host.mod_screen_close(ID);
        }
        // The key counts only while no screen is open.
        if host.consume_click(ID, "waypoints") {
            s.screen = None;
            host.mod_screen_open(ID);
        }
    }

    fn client_tick_post(&self, host: &mut dyn ClientHost, _: &mut ClientTickPostEvent) {
        if host.setting_bool("enabled") == Some(false) {
            return;
        }
        let Some(pos) = host.camera().map(|c| c.position) else { return };
        let mut s = lock(&self.0);
        load_waypoints(host, &mut s);
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
            SettingSpec::toggle("info_coords", "worldmap.setting.info_coords", true),
            SettingSpec::toggle("info_biome", "worldmap.setting.info_biome", false),
            SettingSpec::toggle("info_time", "worldmap.setting.info_time", false),
            SettingSpec::toggle("info_light", "worldmap.setting.info_light", false),
            SettingSpec::toggle("info_dimension", "worldmap.setting.info_dimension", false),
            SettingSpec::range("offset_x", "worldmap.setting.offset_x", 0.0, 640.0, 1.0, f64::from(MARGIN)),
            SettingSpec::range("offset_y", "worldmap.setting.offset_y", 0.0, 360.0, 1.0, f64::from(MARGIN)),
            SettingSpec::range("size", "worldmap.setting.size", 64.0, 256.0, 16.0, 128.0),
            SettingSpec::choice("scale", "worldmap.setting.scale", ["worldmap.scale.1", "worldmap.scale.2", "worldmap.scale.4"], 0),
            SettingSpec::choice("shape", "worldmap.setting.shape", ["worldmap.shape.square", "worldmap.shape.round"], 0),
        ];
        registrar.register_key_mapping(KeyRegistration::new("waypoints", "key.keyboard.n").label("worldmap.key.waypoints"));
        for s in settings {
            if let Err(e) = registrar.register_setting(s) {
                eprintln!("[worldmap] setting refused: {e:?}");
            }
        }
    }

    fn screen_render_post(&self, host: &dyn ClientHost, event: &mut ScreenRenderPostEvent<'_>) {
        if host.mod_screen() != Some(ID) {
            return;
        }
        let [width, _] = event.draw.size();
        let mut guard = lock(&self.0);
        let s = &mut *guard;
        let screen = s.screen.get_or_insert_with(|| {
            let here = host.camera().map_or([0; 3], |c| c.position.map(|v| v.floor() as i32));
            screen::Screen::new(width, here, host.dimension().to_string())
        });
        let x = (width / 2.0 - 100.0).floor();
        event.draw.fill([x - 6.0, 20.0, 212.0, 280.0], 0x80000000);
        let tr = |key: &str, fallback: &str| host.translate(key).unwrap_or_else(|| fallback.to_owned());
        let title = match screen.number() {
            Some(n) => format!("{n} / {}", s.waypoints.list().len()),
            None => tr("worldmap.wp.fresh", "New waypoint"),
        };
        let tw = event.draw.text_width(&title);
        event.draw.text(&title, [x + (200.0 - tw) / 2.0, 36.0], 0xFFFFFFFF, true);
        for (text, dx, y) in screen::Screen::CAPTIONS {
            let text = match text {
                "" => tr("worldmap.wp.name", "Name"),
                "worldmap.wp.colour" => tr(text, "Colour"),
                "worldmap.wp.icon" => tr(text, "Icon"),
                _ => text.to_owned(),
            };
            event.draw.text(&text, [x + dx, y], 0xFFA0A0A0, true);
        }
        // A refused label is not drawn; the screen stays usable.
        let _ = screen.widgets.draw(&mut *event.draw, host, event.screen.mouse);
        // Swatches, the colour preview and the icons go over their (blank) buttons.
        let sel = |on: bool| if on { 0xFFFFFFFF } else { 0xFF000000 };
        for (_, [bx, by, bw, bh], c) in &screen.swatches {
            event.draw.fill([bx + 1.0, by + 1.0, bw - 2.0, bh - 2.0], sel(*c == screen.color));
            event.draw.fill([bx + 2.0, by + 2.0, bw - 4.0, bh - 4.0], 0xFF000000 | c);
        }
        event.draw.fill([x + 70.0, 140.0, 20.0, 20.0], 0xFF000000);
        event.draw.fill([x + 71.0, 141.0, 18.0, 18.0], 0xFF000000 | screen.color);
        for (_, [bx, by, bw, bh], id) in &screen.icon_buttons {
            if *id == screen.icon {
                event.draw.fill([bx - 1.0, by - 1.0, bw + 2.0, bh + 2.0], 0xFFFFFFFF);
                event.draw.fill([*bx, *by, *bw, *bh], 0xFF303030);
            }
            icons::draw(&mut |r, argb| event.draw.fill(r, argb), id, [bx + bw / 2.0, by + bh / 2.0], screen.color);
        }
    }

    fn screen_mouse_button_pressed_pre(&self, host: &dyn ClientHost, event: &mut ScreenMouseButtonPressedPreEvent<'_>) -> Flow {
        let (at, button, modifiers) = (event.screen.mouse, event.button, event.modifiers);
        screen_input(&self.0, host, |w| w.mouse_pressed(host, at, button, modifiers))
    }

    fn screen_mouse_button_released_pre(&self, host: &dyn ClientHost, event: &mut ScreenMouseButtonReleasedPreEvent<'_>) -> Flow {
        let button = event.button;
        screen_input(&self.0, host, |w| w.mouse_released(button))
    }

    fn screen_mouse_dragged_pre(&self, host: &dyn ClientHost, event: &mut ScreenMouseDraggedPreEvent<'_>) -> Flow {
        let (at, button) = (event.screen.mouse, event.button);
        screen_input(&self.0, host, |w| w.mouse_dragged(host, at, button))
    }

    fn screen_key_pressed_pre(&self, host: &dyn ClientHost, event: &mut ScreenKeyPressedPreEvent<'_>) -> Flow {
        let (key, modifiers) = (event.key, event.modifiers);
        screen_input(&self.0, host, |w| w.key_pressed(host, key, modifiers))
    }

    fn screen_char_typed_pre(&self, host: &dyn ClientHost, event: &mut ScreenCharTypedPreEvent<'_>) -> Flow {
        let c = event.c;
        screen_input(&self.0, host, |w| w.char_typed(host, c))
    }

    fn gui_render_post(&self, host: &dyn ClientHost, event: &mut GuiRenderPostEvent<'_>) {
        if !lock(&self.0).ready || host.setting_bool("enabled") == Some(false) {
            return;
        }
        let [w, h] = event.batch.size();
        let side = host.setting_f64("size").unwrap_or(128.0).clamp(64.0, 256.0) as f32;
        let corner = host.setting_choice("corner").unwrap_or(0);
        let ox = host.setting_f64("offset_x").unwrap_or(MARGIN as f64) as f32;
        let oy = host.setting_f64("offset_y").unwrap_or(MARGIN as f64) as f32;
        // Offsets count inward from the corner; the clamp keeps the map on screen at any GUI scale.
        let x = (if corner % 2 == 0 { w - side - ox } else { ox }).clamp(0.0, (w - side).max(0.0));
        let y = (if corner < 2 { oy } else { h - side - oy }).clamp(0.0, (h - side).max(0.0));
        event.batch.sprite(SPRITE, [0.0, 0.0, SIDE as f32, SIDE as f32], [x, y, side, side], None);
        let Some(cam) = host.camera() else { return };
        let (centre, scale, round) = {
            let s = lock(&self.0);
            (s.centre, s.scale, s.round)
        };
        // The image is centred on where the last pass began, so the marker moves off-centre between passes.
        let half = SIDE / 2;
        let px = |pos: f64, c: i32| marker_cell(pos, c, scale, half).clamp(0.0, SIDE as f32) * side / SIDE as f32;
        let (mx, my) = (x + px(cam.position[0], centre.0), y + px(cam.position[2], centre.1));
        // Waypoints of this dimension; the ones out of range stick to the map edge.
        let dim = host.dimension().to_string();
        for w in lock(&self.0).waypoints.list().iter().filter(|w| w.visible && w.dimension == dim) {
            let at = |pos: i32, c: i32| marker_cell(f64::from(pos), c, scale, half) * side / SIDE as f32;
            let [wx, wy] = clamp_to_map([at(w.pos[0], centre.0), at(w.pos[2], centre.1)], side, 5.0, round);
            icons::draw(&mut |r, argb| event.batch.fill(r, argb), &w.icon, [x + wx, y + wy], w.color);
        }
        // Yaw 0 looks along +z; the red tip points where the player looks.
        let yaw = cam.yaw.to_radians();
        let (tx, ty) = (mx - yaw.sin() * 6.0, my + yaw.cos() * 6.0);
        event.batch.fill([mx - 3.0, my - 3.0, 6.0, 6.0], 0xFF000000);
        event.batch.fill([mx - 2.0, my - 2.0, 4.0, 4.0], 0xFFFFFFFF);
        event.batch.fill([tx - 1.5, ty - 1.5, 3.0, 3.0], 0xFFFF2020);
        let pos = cam.position.map(|v| v.floor() as i32);
        let on = |key: &str, default: bool| host.setting_bool(key).unwrap_or(default);
        let base: &dyn Host = host;
        let level = host.level(host.dimension());
        let name = |prefix: &str, id: String| host.translate(&format!("{prefix}.{}", id.replacen(':', ".", 1))).unwrap_or_else(|| pretty_id(&id));
        let mut lines: Vec<String> = Vec::new();
        if on("info_coords", true) {
            lines.push(format!("{} {} {}", pos[0], pos[1], pos[2]));
        }
        if on("info_biome", false) {
            if let Some(b) = level.and_then(|l| l.client_biome(base, pos)) {
                lines.push(name("biome", b));
            }
        }
        if on("info_time", false) {
            if let Some(t) = level.and_then(|l| l.day_time(base)) {
                lines.push(clock(t));
            }
        }
        if on("info_light", false) {
            let light = |layer| level.and_then(|l| l.brightness(base, layer, pos));
            if let (Some(b), Some(s)) = (light(LightLayer::Block), light(LightLayer::Sky)) {
                lines.push(format!("Light {b} / {s}"));
            }
        }
        if on("info_dimension", false) {
            lines.push(name("dimension", host.dimension().to_string()));
        }
        // Lines stack away from the map: down for a top corner, up for a bottom one.
        let top = if corner < 2 { y + side + 2.0 } else { y - 2.0 - 10.0 * lines.len() as f32 };
        for (i, line) in lines.iter().enumerate() {
            let tw = event.batch.text_width(line);
            event.batch.text(line, [x + (side - tw) / 2.0, top + 10.0 * i as f32], 0xFFFFFFFF, true);
        }
    }
}
