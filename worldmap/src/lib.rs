//! Worldmap for Horizon: a minimap drawn over the GUI.
mod fullmap;
mod icons;
mod list;
mod sampler;
mod screen;
mod tiles;
mod waypoints;
use mod_api::*;
use sampler::{clamp_to_map, clock, map_rgba, marker_cell, pretty_id, shade, Scan};
use mod_api::client::widgets::Outcome;
use waypoints::{dist2, Waypoints};
use std::collections::HashMap;
use std::sync::Mutex;

/// Sampled grid side in cells; the drawn size and blocks per cell are settings.
const SIDE: u32 = 128;
/// Columns read per tick (spec A6).
const BUDGET: u32 = 4096;
/// Dirty tiles are written this often (5 s).
const FLUSH_TICKS: u64 = 100;
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
    /// The waypoint screen while it is open; None builds the list when the screen draws.
    view: Option<View>,
    /// The list asked to open the edit screen (needs the screen width, known when drawing).
    open: Option<Option<usize>>,
    /// Where a new waypoint opened from the map goes (a right click); None is the player.
    open_here: Option<[i32; 3]>,
    /// The map while the edit screen opened from it is up; Back returns to it.
    back_map: Option<fullmap::MapView>,
    /// A button asked to close the screen; the next client tick does it (input hooks only read).
    want_close: bool,
    /// A teleport asked by a screen button; the next client tick sends it (input hooks only read).
    tp: Option<[i32; 3]>,
    /// Explored terrain of the world and dimension in `scope` (ADR 0003), kept on disk.
    store: tiles::Store,
    /// Overview of every tile seen in `scope` (ADR 0004).
    mip: tiles::Mip,
    scope: (Option<String>, String),
    /// The tile being scanned.
    job: Option<tiles::Job>,
    /// The key asked for the fullscreen map; the screen draw opens it (it needs the screen size).
    open_map: bool,
    /// Client ticks since the mod started; the clock of the rescan rule.
    tick: u64,
}

/// The two views of the waypoint screen.
enum View {
    List(list::List),
    Edit(screen::Screen),
    Map(fullmap::MapView),
}

impl View {
    fn widgets(&mut self) -> &mut mod_api::client::widgets::Widgets {
        match self {
            View::List(l) => &mut l.widgets,
            View::Edit(e) => &mut e.widgets,
            View::Map(m) => &mut m.widgets,
        }
    }
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
            view: None,
            open: None,
            open_here: None,
            back_map: None,
            want_close: false,
            tp: None,
            store: tiles::Store::new(tiles::CAPACITY),
            mip: tiles::Mip::new(),
            scope: (None, String::new()),
            job: None,
            open_map: false,
            tick: 0,
        }))
    }
}

fn lock(m: &Mutex<State>) -> std::sync::MutexGuard<'_, State> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// This mod's id: names its screen and its key mapping.
const ID: &str = "worldmap";

/// Block id to map colour id, asked from the host once per block.
fn map_color(host: &dyn ClientHost, colors: &mut HashMap<String, u8>, block: String) -> u8 {
    if let Some(&id) = colors.get(&block) {
        return id;
    }
    let id = host.block_state_id(&block).and_then(|b| host.block_states().map_color(b)).unwrap_or(0);
    colors.insert(block, id);
    id
}

/// Disk path of a tile of `scope`; None when the world is unknown (nothing is kept then).
fn tile_path(scope: &(Option<String>, String), key: tiles::Key) -> Option<ConfigPath> {
    let world = scope.0.as_deref()?;
    ConfigPath::new(&format!("{}/{}", tiles::folder(world, &scope.1), tiles::file_name(key))).ok()
}

fn write_tile(host: &dyn ClientHost, scope: &(Option<String>, String), key: tiles::Key, tile: &tiles::Tile) -> bool {
    let Some(path) = tile_path(scope, key) else { return true };
    let ok = host.data_files().write(&path, &tile.serialize());
    if let Err(e) = &ok {
        eprintln!("[worldmap] tile {key:?} not saved: {e:?}");
    }
    ok.is_ok()
}

/// Writes every dirty tile; one that fails stays dirty for the next round.
fn flush(host: &dyn ClientHost, s: &mut State) {
    for key in s.store.dirty() {
        if let Some(t) = s.store.get_mut(key) {
            if write_tile(host, &s.scope, key, t) {
                t.dirty = false;
            }
        }
    }
}

/// Puts a tile in memory and writes the dirty ones this pushes out.
fn put(host: &dyn ClientHost, s: &mut State, key: tiles::Key, tile: tiles::Tile) {
    for (k, t) in s.store.insert(key, tile) {
        write_tile(host, &s.scope, k, &t);
    }
}

/// Scans part of one tile into the store (spec A17): a new tile when none is running, else the next
/// `tiles::BUDGET` columns of the running one. A column the host cannot read keeps what is stored.
/// Tiles come from disk first (spec A18) and dirty ones are written every `FLUSH_TICKS`.
fn explore(host: &dyn ClientHost, s: &mut State, player: (i32, i32)) {
    s.tick += 1;
    let scope = (s.world.clone(), host.dimension().to_string());
    if s.scope != scope {
        flush(host, s);
        s.scope = scope;
        s.store = tiles::Store::new(tiles::CAPACITY);
        s.mip.clear();
        s.job = None;
    }
    // Tiles on disk count as known: load the picked one, and pick again unless it must be rescanned (few reads a tick).
    for _ in 0..4 {
        if s.job.is_some() {
            break;
        }
        let Some(key) = tiles::pick(&s.store, player, s.tick) else { break };
        let loaded = tile_path(&s.scope, key).and_then(|p| host.data_files().read(&p).ok()).and_then(|b| tiles::Tile::parse(&b));
        match loaded {
            Some(tile) => {
                let (px, pz) = tiles::tile_of(player.0, player.1);
                let near = (key.0 - px).abs().max((key.1 - pz).abs()) <= tiles::NEAR;
                let rescan = tile.needs_rescan(s.tick, near);
                s.mip.insert(key, tile.mip());
                put(host, s, key, tile);
                if rescan {
                    s.job = Some(tiles::Job::new(key));
                }
            }
            None => s.job = Some(tiles::Job::new(key)),
        }
    }
    if s.tick % FLUSH_TICKS == 0 {
        flush(host, s);
    }
    let Some(job) = s.job.as_mut() else { return };
    let key = job.key;
    let range = job.batch();
    let cols: Vec<(i32, i32)> = range.map(|i| job.column(i)).collect();
    let done = job.done();
    let Ok(found) = host.client_far_surface(&cols) else { return };
    if s.store.peek(key).is_none() {
        put(host, s, key, tiles::Tile::default());
    }
    let Some(tile) = s.store.get_mut(key) else { return };
    for (&(x, z), col) in cols.iter().zip(found) {
        let Some(c) = col else { continue };
        let id = map_color(host, &mut s.colors, c.block);
        let (id, brightness, level) = shade(id, c.height, c.water, tile.north_height(x, z));
        tile.set(x, z, tiles::Column { color: id, shade: tiles::shade_of(brightness), height: level.clamp(i16::MIN.into(), i16::MAX.into()) as i16 });
    }
    if done {
        tile.scanned_at = Some(s.tick);
        s.mip.insert(key, tile.mip());
        s.job = None;
    }
}

/// Surface height stored for world column `(x, z)`; None when the tile or the column is unknown.
fn surface(store: &tiles::Store, x: i32, z: i32) -> Option<i32> {
    store.peek(tiles::tile_of(x, z)).map(|t| t.get(x, z)).filter(|c| c.color != 0).map(|c| i32::from(c.height))
}

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
    let Some(view) = s.view.as_mut() else { return Flow::Continue };
    let outcome = input(view.widgets());
    let act = match view {
        View::List(l) => l.act(outcome, &mut s.waypoints),
        View::Edit(e) => {
            let act = e.act(outcome, &mut s.waypoints);
            e.refresh();
            act
        }
        View::Map(m) => {
            let here = host.camera().map_or((0, 0), |c| (c.position[0].floor() as i32, c.position[2].floor() as i32));
            m.act(outcome, here)
        }
    };
    if act.save {
        save_waypoints(host, s);
    }
    if act.back {
        s.view = s.back_map.take().map(|mut m| {
            m.invalidate();
            View::Map(m)
        });
    }
    if act.open.is_some() {
        s.open = act.open;
    }
    s.want_close |= act.close;
    if act.teleport.is_some() {
        s.tp = act.teleport;
        s.want_close = true;
    }
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

/// Reads the visible tiles that are new to the map from disk, a few a frame (spec A18, ADR 0004): at 1 block a pixel
/// the full tile goes into memory, at larger scales only its overview is kept. The image is made again after each batch.
fn load_view(host: &dyn ClientHost, s: &mut State) {
    let Some(View::Map(m)) = &mut s.view else { return };
    let full = m.scale == 1;
    let missing: Vec<tiles::Key> = fullmap::visible_tiles(m.centre, m.scale, m.image.0, m.image.1)
        .into_iter()
        .filter(|k| !m.tried.contains(&(*k, full)) && if full { s.store.peek(*k).is_none() } else { !s.mip.contains_key(k) })
        .take(fullmap::LOADS_PER_FRAME)
        .collect();
    if missing.is_empty() {
        return;
    }
    m.tried.extend(missing.iter().map(|k| (*k, full)));
    m.invalidate();
    for key in missing {
        let loaded = tile_path(&s.scope, key).and_then(|p| host.data_files().read(&p).ok()).and_then(|b| tiles::Tile::parse(&b));
        if let Some(tile) = loaded {
            s.mip.insert(key, tile.mip());
            if full {
                put(host, s, key, tile);
            }
        }
    }
}

fn draw_map(host: &dyn ClientHost, event: &mut ScreenRenderPostEvent<'_>, m: &mut fullmap::MapView, wps: &Waypoints, store: &tiles::Store) {
    let size = event.draw.size();
    if m.gui != size {
        // The window was resized: lay out again, keep the view.
        let mut fresh = fullmap::MapView::new(size, m.centre);
        fresh.scale = m.scale;
        fresh.tried = std::mem::take(&mut m.tried);
        fresh.set_target(m.target);
        *m = fresh;
    }
    event.draw.fill([0.0, 0.0, size[0], size[1]], 0xFF101014);
    if m.uploaded {
        event.draw.sprite(SPRITE, [0.0, 0.0, m.image.0 as f32, m.image.1 as f32], [0.0, 0.0, size[0], size[1]], None);
    }
    let dim = host.dimension().to_string();
    for w in wps.list().iter().filter(|w| w.visible && w.dimension == dim) {
        let [wx, wy] = m.to_gui(f64::from(w.pos[0]) + 0.5, f64::from(w.pos[2]) + 0.5);
        if wx >= 0.0 && wy >= 0.0 && wx < size[0] && wy < size[1] {
            icons::draw(&mut |r, argb| event.draw.fill(r, argb), &w.icon, [wx, wy], w.color);
        }
    }
    if let Some(cam) = host.camera() {
        let [mx, my] = m.to_gui(cam.position[0], cam.position[2]);
        let yaw = cam.yaw.to_radians();
        let (tx, ty) = (mx - yaw.sin() * 7.0, my + yaw.cos() * 7.0);
        event.draw.fill([mx - 3.0, my - 3.0, 6.0, 6.0], 0xFF000000);
        event.draw.fill([mx - 2.0, my - 2.0, 4.0, 4.0], 0xFFFFFFFF);
        event.draw.fill([tx - 1.5, ty - 1.5, 3.0, 3.0], 0xFFFF2020);
    }
    if let Some([tx, _, tz]) = m.target {
        let [x, y] = m.to_gui(f64::from(tx) + 0.5, f64::from(tz) + 0.5);
        event.draw.fill([x - 5.0, y - 1.0, 10.0, 2.0], 0xFFFFFFFF);
        event.draw.fill([x - 1.0, y - 5.0, 2.0, 10.0], 0xFFFFFFFF);
    }
    // Under the cursor: X Z, the stored surface Y, and the biome where the host can read it (loaded chunks only).
    let mouse = event.screen.mouse;
    if mouse[1] < size[1] - 30.0 {
        let (x, z) = m.to_world(mouse);
        let y = surface(store, x, z);
        let mut line = format!("X {x}  Z {z}");
        if let Some(y) = y {
            line.push_str(&format!("  Y {y}"));
        }
        if let Some(y) = y {
            let base: &dyn Host = host;
            if let Some(b) = host.level(host.dimension()).and_then(|l| l.client_biome(base, [x, y, z])) {
                line.push_str("  ");
                line.push_str(&host.translate(&format!("biome.{}", b.replacen(':', ".", 1))).unwrap_or_else(|| pretty_id(&b)));
            }
        }
        let w = event.draw.text_width(&line);
        event.draw.fill([2.0, 2.0, w + 6.0, 12.0], 0x80000000);
        event.draw.text(&line, [5.0, 4.0], 0xFFFFFFFF, true);
    }
    // A refused label is not drawn; the screen stays usable.
    let _ = m.widgets.draw(&mut *event.draw, host, event.screen.mouse);
}

fn draw_edit(host: &dyn ClientHost, event: &mut ScreenRenderPostEvent<'_>, screen: &mut screen::Screen, width: f32) {
    let x = (width / 2.0 - 100.0).floor();
    event.draw.fill([x - 6.0, 20.0, 212.0, 280.0], 0x80000000);
    let tr = |key: &str, fallback: &str| host.translate(key).unwrap_or_else(|| fallback.to_owned());
    let title = tr(if screen.is_new() { "worldmap.wp.fresh" } else { "worldmap.wp.edit" }, "Waypoint");
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

/// `text` cut with `..` so it fits `max` GUI pixels.
fn fit(event: &mut ScreenRenderPostEvent<'_>, text: &str, max: f32) -> String {
    if event.draw.text_width(text) <= max {
        return text.to_owned();
    }
    let mut out: String = text.to_owned();
    while !out.is_empty() && event.draw.text_width(&format!("{out}..")) > max {
        out.pop();
    }
    format!("{out}..")
}

fn draw_list(host: &dyn ClientHost, event: &mut ScreenRenderPostEvent<'_>, l: &mut list::List, wps: &Waypoints) {
    let x = l.left();
    event.draw.fill([x - 6.0, 20.0, list::WIDTH + 12.0, list::PAGER_Y + 52.0 - 20.0], 0x80000000);
    let title = host.translate("worldmap.wp.list").unwrap_or_else(|| "Waypoints".to_owned());
    let tw = event.draw.text_width(&title);
    event.draw.text(&title, [x + (list::WIDTH - tw) / 2.0, 30.0], 0xFFFFFFFF, true);
    // A refused label is not drawn; the screen stays usable.
    let _ = l.widgets.draw(&mut *event.draw, host, event.screen.mouse);
    for row in l.rows() {
        let Some(w) = wps.list().get(row.index) else { continue };
        let [bx, by, bw, _] = row.rect;
        icons::draw(&mut |r, argb| event.draw.fill(r, argb), &w.icon, [bx + 10.0, by + 10.0], w.color);
        let far = if w.dimension == l.dimension { format!("{:.0} m", (dist2(w.pos, l.here) as f64).sqrt()) } else { pretty_id(&w.dimension) };
        let fw = event.draw.text_width(&far);
        let name = fit(event, &w.name, bw - 24.0 - fw - 6.0);
        let tint = if w.visible { 0xFFFFFFFF } else { 0xFF808080 };
        event.draw.text(&name, [bx + 20.0, by + 6.0], tint, true);
        event.draw.text(&far, [bx + bw - fw - 4.0, by + 6.0], 0xFFA0A0A0, true);
    }
    let (page, pages) = l.page();
    let label = format!("{page} / {pages}");
    let lw = event.draw.text_width(&label);
    event.draw.text(&label, [x + (list::WIDTH - lw) / 2.0, list::PAGER_Y + 6.0], 0xFFFFFFFF, true);
}

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn client_tick_pre(&self, host: &mut dyn ClientHost, _: &mut ClientTickPreEvent) {
        let mut s = lock(&self.0);
        if std::mem::take(&mut s.want_close) {
            host.mod_screen_close(ID);
        }
        if let Some([x, y, z]) = s.tp.take() {
            // Block centre; the server answers a missing permission in chat.
            let cmd = format!("/tp @s {:.1} {y} {:.1}", x as f64 + 0.5, z as f64 + 0.5);
            if host.send_chat(&cmd).is_err() {
                let _ = host.chat_line(Text::translate("worldmap.wp.tp.fail", Vec::new()));
            }
        }
        // The key counts only while no screen is open.
        if host.consume_click(ID, "waypoints") {
            s.view = None;
            s.back_map = None;
            host.mod_screen_open(ID);
        }
        if host.consume_click(ID, "map") {
            s.view = None;
            s.back_map = None;
            s.open_map = true;
            host.mod_screen_open(ID);
        }
    }

    fn client_tick_post(&self, host: &mut dyn ClientHost, _: &mut ClientTickPostEvent) {
        if host.setting_bool("enabled") == Some(false) {
            return;
        }
        let mut s = lock(&self.0);
        let Some(pos) = host.camera().map(|c| c.position) else {
            // Out of a world (menu, disconnect): write what is left.
            flush(host, &mut s);
            return;
        };
        load_waypoints(host, &mut s);
        explore(host, &mut s, (pos[0].floor() as i32, pos[2].floor() as i32));
        let s = &mut *s;
        let open = host.mod_screen() == Some(ID);
        if let Some(View::Map(m)) = s.view.as_mut().filter(|_| open) {
            // The one shared image slot belongs to the map while it is open.
            if m.stale(s.tick) {
                let rgba = fullmap::render(&s.store, &s.mip, m.centre, m.scale, m.image.0, m.image.1);
                if host.mod_image(m.image.0 as u32, m.image.1 as u32, &rgba) {
                    m.made(s.tick);
                }
            }
            return;
        }
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
                    let id = map_color(host, &mut s.colors, c.block);
                    // Row 0 and an unread north neighbour stay flat.
                    let north = i.checked_sub(SIDE).and_then(|n| s.heights[n as usize]);
                    let (id, brightness, level) = shade(id, c.height, c.water, north);
                    s.heights[i as usize] = Some(level);
                    map_rgba(id, brightness)
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
        registrar.register_key_mapping(KeyRegistration::new("map", "key.keyboard.m").label("worldmap.key.map"));
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
        let [width, height] = event.draw.size();
        let mut guard = lock(&self.0);
        let s = &mut *guard;
        let here = host.camera().map_or([0; 3], |c| c.position.map(|v| v.floor() as i32));
        let dimension = host.dimension().to_string();
        if std::mem::take(&mut s.open_map) {
            s.view = Some(View::Map(fullmap::MapView::new([width, height], (here[0], here[2]))));
            // The minimap's image is replaced by the map's; hide it until its next pass.
            s.ready = false;
        }
        load_view(host, s);
        if let Some(i) = s.open.take() {
            if let Some(View::Map(m)) = s.view.take() {
                s.back_map = Some(m);
            }
            let at = s.open_here.take().unwrap_or(here);
            s.view = Some(View::Edit(screen::Screen::new(width, at, dimension.clone(), i, &s.waypoints)));
        }
        let view = s.view.get_or_insert_with(|| View::List(list::List::new(width, here, dimension, &s.waypoints)));
        match view {
            View::Edit(screen) => draw_edit(host, event, screen, width),
            View::Map(m) => draw_map(host, event, m, &s.waypoints, &s.store),
            View::List(l) => draw_list(host, event, l, &s.waypoints),
        }
    }

    fn screen_mouse_button_pressed_pre(&self, host: &dyn ClientHost, event: &mut ScreenMouseButtonPressedPreEvent<'_>) -> Flow {
        let (at, button, modifiers) = (event.screen.mouse, event.button, event.modifiers);
        let flow = screen_input(&self.0, host, |w| w.mouse_pressed(host, at, button, modifiers));
        if matches!(flow, Flow::Cancel) || host.mod_screen() != Some(ID) {
            return flow;
        }
        let mut guard = lock(&self.0);
        let s = &mut *guard;
        let Some(View::Map(m)) = &mut s.view else { return flow };
        // The bottom strip holds the buttons.
        if at[1] >= m.gui[1] - 30.0 {
            return flow;
        }
        if button == 0 {
            if let Some(i) = m.waypoint_at(s.waypoints.list(), &host.dimension().to_string(), at) {
                s.open = Some(Some(i));
                return Flow::Cancel;
            }
            (m.press, m.moved) = (Some(at), false);
        } else if button == 1 {
            // A right click starts a new waypoint here, at the stored surface (Y = height + 1), else at the player's height.
            let (x, z) = m.to_world(at);
            let y = surface(&s.store, x, z).map_or_else(|| host.camera().map_or(64, |c| c.position[1].floor() as i32), |h| h + 1);
            s.open_here = Some([x, y, z]);
            s.open = Some(None);
            return Flow::Cancel;
        }
        flow
    }

    fn screen_mouse_button_released_pre(&self, host: &dyn ClientHost, event: &mut ScreenMouseButtonReleasedPreEvent<'_>) -> Flow {
        let button = event.button;
        let flow = screen_input(&self.0, host, |w| w.mouse_released(button));
        if button != 0 || host.mod_screen() != Some(ID) {
            return flow;
        }
        let mut guard = lock(&self.0);
        let s = &mut *guard;
        let Some(View::Map(m)) = &mut s.view else { return flow };
        // A press and release with no drag between is a click: choose that point for Teleport (Y: stored surface + 1, else the player's).
        if let Some(at) = m.press.take().filter(|_| !m.moved) {
            let (x, z) = m.to_world(at);
            let y = surface(&s.store, x, z).map_or_else(|| host.camera().map_or(64, |c| c.position[1].floor() as i32), |h| h + 1);
            m.set_target(Some([x, y, z]));
        }
        flow
    }

    fn screen_mouse_dragged_pre(&self, host: &dyn ClientHost, event: &mut ScreenMouseDraggedPreEvent<'_>) -> Flow {
        let (at, button, delta) = (event.screen.mouse, event.button, event.delta);
        let flow = screen_input(&self.0, host, |w| w.mouse_dragged(host, at, button));
        if matches!(flow, Flow::Cancel) || host.mod_screen() != Some(ID) {
            return flow;
        }
        match &mut lock(&self.0).view {
            Some(View::Map(m)) => {
                m.moved = true;
                m.pan(delta);
                Flow::Cancel
            }
            _ => flow,
        }
    }

    fn screen_mouse_scrolled_pre(&self, host: &dyn ClientHost, event: &mut ScreenMouseScrolledPreEvent<'_>) -> Flow {
        if host.mod_screen() != Some(ID) {
            return Flow::Continue;
        }
        match &mut lock(&self.0).view {
            Some(View::Map(m)) => {
                m.zoom(event.scroll[1] as f32);
                Flow::Cancel
            }
            _ => Flow::Continue,
        }
    }

    fn screen_key_pressed_pre(&self, host: &dyn ClientHost, event: &mut ScreenKeyPressedPreEvent<'_>) -> Flow {
        let (key, modifiers) = (event.key, event.modifiers);
        // shortcut: the map closes on the key's default M, not on a rebound key; Esc and the button always close it.
        if key == "key.keyboard.m" && host.mod_screen() == Some(ID) {
            let mut s = lock(&self.0);
            if matches!(s.view, Some(View::Map(_))) {
                s.want_close = true;
                return Flow::Cancel;
            }
        }
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
