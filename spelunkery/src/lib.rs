//! spelunkery: Spelunkery 1.20.1-0.3.16 (Ordana, Keybounce; LGPL-3.0), from the decompiled jar
//! (tools/dl/spelunkery/named) and the registry oracle's facts (tools/ns_oracle). data/ and assets/ are the content mod
//! mods/spelunkery (tools/spelunkery_port.py), which also writes `classes.rs` (every block's Java class and template)
//! and `vanilla_loot.rs` (the vanilla loot tables the jar's dynamic pack replaced). All of it is the mod API: no game
//! module, no mixin.
//!
//! One `BlockPart` (`Rules`) on every block:
//! - `exp_drop`: the ores whose Java DropExperienceBlock has no XP range drop none (their vanilla templates do);
//! - mushrooms (ModMushroomBlock on the brown mushroom template, which already has Java's survival and the inkcaps'
//!   spreading): button mushrooms grow into crimini and crimini into portabellas one tick in 25, portabellas spread
//!   button mushrooms; bone meal (0.4) grows the inkcaps and portabellas into the jar's huge mushrooms
//!   (GrowableMushroomBlock), button and crimini take none;
//! - floor-and-wall fungi (phosphor fungus, mushgloom, conk): placed on the floor or on a wall by the clicked face,
//!   they need the face they sit on (FloorAndSidesMushroomBlock); bone meal on a floor one over mushroom ground or below
//!   y 0 grows the huge one (0.4);
//! - milly bubcaps: survive on any floor; another bubcap clicked on one adds a cap (up to 8); bone meal spreads
//!   bubcaps on mushroom ground around and fills it to 8 caps, and at 8 grows the huge bubcap patch;
//! - sporophytes: bone meal makes a tall sporophyte (SporophyteBlock);
//! - tangle roots (GrowingPlantHeadBlock down, on the weeping vines templates): grow down one time in ten into air or
//!   water up to age 25, bone meal grows 1-4; a root whose support goes breaks, a body piece with nothing below becomes
//!   the tip; `top` follows the block above (TangleRootsBodyBlock);
//! - rope ladders (on the ladder template): hang from a wall or the ladder above, `top`/`end` follow the column, a
//!   rope ladder clicked on one goes to the bottom of the column, a sneaking click on the top takes the bottom rung
//!   back (RopeLadderBlock, RopeLadderBlockItem);
//! - salt (SaltBlock): connects to salt beside it, and up or down a block like redstone dust;
//! - salt lamps switch on and off when clicked (SaltLampBlock);
//! - the directional blocks on the stone template face the clicked face (or the player for the fountain);
//! - potted plants give their plant back; a `PotPlant` part on `minecraft:flower_pot` pots the jar's nine plants.
//! - grindstones polish (ModEvents.useGrindstone, GrindstoneMixin): an item with one of the jar's polishing tables
//!   clicked on the diamond grindstone or a `Grindstone` part on `minecraft:grindstone` gives the table's items and
//!   experience; the diamond grindstone wears with use and a diamond shard mends it.
//!
//! Items: glowsticks thrown in the air fly as snowballs carrying the glowstick and stick where they land as the block
//! (GlowstickItem, ThrownGlowstickEntity); the depth gauge shows the Y level over the hotbar (DepthGaugeItem.use); the
//! item magnet switches on and off (its glint) and pulls items within 8 blocks while on (MagnetItem); a parachute in
//! the inventory opens after half a second of falling faster than 1 block a tick and gives slow falling until the
//! player lands, then takes one damage (ParachuteItem). Pebbles copy the snowball and need no code.
//!
//! `loot_table_load`: the vanilla ore, sculk and piglin bartering tables the jar's dynamic pack replaced (live server
//! config: rough gems and raw nuggets on), and the chest injects (mineshaft, ruined portal, stronghold library,
//! ancient city).
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use mod_api::serde_json::{json, Value};
use mod_api::{
    BlockPart, BlockUpdateFlags, Compound, DataLoadContext, Direction, Hand, Hooks, Identifier, InteractionResult,
    ItemPart, ItemStack, LevelHost, LevelRef, LevelTicksHost, LevelWriteHost, LootTableLoadEvent, PerformBonemealEvent,
    RandomTickEvent, Registrar, RegistrationContext, ServerHost, ServerTickEvent, ServerTickPhase, SoundSource,
    StateForPlacementEvent, Tag, UpdateShapeEvent, UseItemOnEvent, UseWithoutItemEvent,
};

mod classes;
mod vanilla_loot;

const NS: &str = "spelunkery:";

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn debug() -> bool {
    std::env::var_os("SPELUNKERY_DEBUG").is_some()
}

fn class_map() -> &'static HashMap<&'static str, (&'static str, &'static str)> {
    static M: OnceLock<HashMap<&'static str, (&'static str, &'static str)>> = OnceLock::new();
    M.get_or_init(|| classes::BLOCKS.iter().map(|&(n, c, t)| (n, (c, t))).collect())
}

// ---------------------------------------------------------------------------------------------
// States as text
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
struct St {
    id: String,
    props: Vec<(String, String)>,
}

impl St {
    fn parse(s: &str) -> St {
        let (id, rest) = s.split_once('[').unwrap_or((s, ""));
        let props = rest
            .trim_end_matches(']')
            .split(',')
            .filter_map(|kv| kv.split_once('='))
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        St { id: id.to_string(), props }
    }
    fn get(&self, k: &str) -> Option<&str> {
        self.props.iter().find(|(a, _)| a == k).map(|(_, v)| v.as_str())
    }
    fn is(&self, k: &str, v: &str) -> bool {
        self.get(k) == Some(v)
    }
    fn set(&mut self, k: &str, v: &str) {
        if let Some(p) = self.props.iter_mut().find(|(a, _)| a == k) {
            p.1 = v.to_string();
        }
    }
    fn with(&self, k: &str, v: &str) -> St {
        let mut s = self.clone();
        s.set(k, v);
        s
    }
    fn text(&self) -> String {
        if self.props.is_empty() {
            return self.id.clone();
        }
        let p: Vec<String> = self.props.iter().map(|(k, v)| format!("{k}={v}")).collect();
        format!("{}[{}]", self.id, p.join(","))
    }
    fn name(&self) -> &str {
        self.id.strip_prefix(NS).unwrap_or("")
    }
    fn class(&self) -> &'static str {
        class_map().get(self.name()).map_or("", |c| c.0)
    }
    fn int(&self, k: &str) -> i32 {
        self.get(k).and_then(|v| v.parse().ok()).unwrap_or(0)
    }
}

fn b(v: bool) -> &'static str {
    if v { "true" } else { "false" }
}

fn bp(p: mod_api::BlockPos) -> [i32; 3] {
    [p.x, p.y, p.z]
}

fn add(a: [i32; 3], b: [i32; 3]) -> [i32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn vec_of(d: &str) -> [i32; 3] {
    match d {
        "north" => [0, 0, -1],
        "south" => [0, 0, 1],
        "west" => [-1, 0, 0],
        "east" => [1, 0, 0],
        "up" => [0, 1, 0],
        _ => [0, -1, 0],
    }
}

fn at(p: [i32; 3], d: &str) -> [i32; 3] {
    add(p, vec_of(d))
}

fn opposite(d: &str) -> &'static str {
    match d {
        "north" => "south",
        "south" => "north",
        "west" => "east",
        "east" => "west",
        "up" => "down",
        _ => "up",
    }
}

fn dir_name(d: Direction) -> &'static str {
    match d {
        Direction::Down => "down",
        Direction::Up => "up",
        Direction::North => "north",
        Direction::South => "south",
        Direction::West => "west",
        Direction::East => "east",
    }
}

#[cfg(test)]
fn face_name(v: [i32; 3]) -> &'static str {
    match v {
        [0, 1, 0] => "up",
        [0, -1, 0] => "down",
        [0, 0, -1] => "north",
        [0, 0, 1] => "south",
        [-1, 0, 0] => "west",
        _ => "east",
    }
}

/// `Direction.from2DDataValue(floor(yaw / 90 + 0.5) & 3)`: the way the player looks.
fn horizontal(yaw: f32) -> &'static str {
    ["south", "west", "north", "east"][((yaw / 90.0 + 0.5).floor() as i32 & 3) as usize]
}

const HORIZONTALS: [&str; 4] = ["north", "east", "south", "west"];

fn lst(level: &dyn LevelHost, p: [i32; 3]) -> Option<St> {
    level.block_state(p).and_then(|s| level.state_string(s)).map(|t| St::parse(&t))
}

fn lset(level: &mut dyn LevelWriteHost, p: [i32; 3], st: &St) -> bool {
    let Some(state) = level.block_state_id(&st.text()) else { return false };
    level.set_block(p, state, BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS)
}

fn ldefault(level: &dyn LevelHost, id: &str) -> Option<St> {
    level.block_state_id(id).and_then(|s| level.state_string(s)).map(|t| St::parse(&t))
}

fn lrand(level: &mut dyn LevelWriteHost, bound: i32) -> i32 {
    level.with_random(|r| mod_api::RandomSource::next_int_bound(r, bound).unwrap_or(0)).unwrap_or(0)
}

fn lfloat(level: &mut dyn mod_api::LevelRandomHost) -> f32 {
    level.with_random(|r| mod_api::RandomSource::next_float(r)).unwrap_or(1.0)
}

fn host_look<'a>(host: &'a dyn ServerHost, level: LevelRef) -> impl Fn([i32; 3]) -> Option<St> + 'a {
    move |p| level.block_state(host, p).and_then(|s| host.block_state_string(s)).map(|t| St::parse(&t))
}

fn is_air(st: &Option<St>) -> bool {
    st.as_ref().is_none_or(|s| matches!(s.id.as_str(), "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"))
}

fn is_water(st: &Option<St>) -> bool {
    st.as_ref().is_some_and(|s| s.id == "minecraft:water")
}

/// Blocks with no full face to stand on or hang from (an approximation of `isFaceSturdy`, which the level hooks can't
/// ask): air, liquids, plants, and the thin blocks.
fn sturdy(st: &Option<St>) -> bool {
    let Some(s) = st else { return false };
    if is_air(st) {
        return false;
    }
    let id = s.id.as_str();
    let n = id.split_once(':').map_or(id, |x| x.1);
    let thin = [
        "water", "lava", "short_grass", "tall_grass", "fern", "large_fern", "snow", "torch", "wall_torch", "ladder",
        "vine", "glow_lichen", "moss_carpet", "redstone_wire", "rail", "lever", "flower_pot", "seagrass",
        "tall_seagrass", "kelp", "kelp_plant", "hanging_roots", "cobweb", "fire", "soul_fire", "lantern",
        "soul_lantern", "chain", "iron_chain", "end_rod", "lightning_rod", "candle", "bubble_column",
    ];
    if thin.contains(&n) || n.ends_with("_carpet") || n.ends_with("_sapling") || n.ends_with("_button")
        || n.ends_with("_pressure_plate") || n.ends_with("_sign") || n.ends_with("_banner") || n.ends_with("_rail")
        || n.ends_with("_mushroom") || n.ends_with("_fungus") || n.ends_with("_roots") || n.ends_with("_candle")
        || n.ends_with("_trapdoor") || n.ends_with("_door") || n.ends_with("_fence") || n.ends_with("_wall")
        || n.ends_with("_pane") || n.ends_with("_bars") || n.ends_with("_slab") && !s.is("type", "double")
    {
        return false;
    }
    if let Some(ours) = id.strip_prefix(NS) {
        let t = class_map().get(ours).map_or("", |c| c.1);
        return matches!(t, "stone" | "oak_planks" | "sand" | "redstone_ore" | "redstone_block" | "raw_copper_block"
            | "quartz_pillar" | "brown_mushroom_block" | "mushroom_stem" | "shroomlight" | "coal_ore" | "iron_ore"
            | "copper_ore" | "gold_ore" | "lapis_ore" | "emerald_ore" | "diamond_ore")
            && !matches!(class_map().get(ours).map_or("", |c| c.0), "FloorAndSidesMushroomBlock"
                | "ConkFungusBlock" | "SaltBlock" | "ChannelBlock" | "ChannelSluiceBlock" | "MineomiteBlock"
                | "NephriteFountainBlock" | "NephriteSiphonBlock");
    }
    true
}

/// `#minecraft:mushroom_grow_block` (26.3).
fn mushroom_ground(st: &Option<St>) -> bool {
    st.as_ref().is_some_and(|s| {
        matches!(s.id.as_str(), "minecraft:mycelium" | "minecraft:podzol" | "minecraft:crimson_nylium" | "minecraft:warped_nylium")
    })
}

// ---------------------------------------------------------------------------------------------
// Growth that places a feature (from server_tick: the level hooks place no feature)
// ---------------------------------------------------------------------------------------------

/// (dimension, position, the block before, the configured feature).
static GROW: Mutex<Vec<(String, [i32; 3], String, String)>> = Mutex::new(Vec::new());

fn queue_feature(dim: &str, pos: [i32; 3], st: &St, feature: &str) {
    lock(&GROW).push((dim.to_string(), pos, st.text(), format!("{NS}{feature}")));
}

/// GrowableMushroomBlock/FloorAndSidesMushroomBlock.growMushroom: the block goes, the feature is placed, the block
/// comes back when it can't be.
fn grow(host: &mut dyn ServerHost, dim: &str, pos: [i32; 3], before: &str, feature: &str) {
    let Some(level) = host.level(dim) else { return };
    let Some(air) = host.block_state_id("minecraft:air") else { return };
    let Some(old) = host.block_state_id(before) else { return };
    let _ = level.set_block(host, pos, air, BlockUpdateFlags::CLIENTS);
    let cmd = format!("execute in {dim} run place feature {feature} {} {} {}", pos[0], pos[1], pos[2]);
    host.commands().run_command(&cmd);
    let placed = !is_air(&host_look(&*host, level)(pos));
    if debug() {
        eprintln!("[spelunkery] {cmd}: {}", if placed { "grew" } else { "no room" });
    }
    if !placed {
        let _ = level.set_block(host, pos, old, BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS);
    }
}

/// The huge mushroom bone meal grows (the jar's `*_bonemeal` configured features).
fn huge_of(name: &str) -> Option<&'static str> {
    Some(match name {
        "inkcap_mushroom" => "huge_inkcap_mushroom_bonemeal",
        "white_inkcap_mushroom" => "huge_white_inkcap_mushroom_bonemeal",
        "portabella" => "huge_portabella_bonemeal",
        "phosphor_fungus" => "huge_phosphor_fungus_bonemeal",
        "mushgloom" => "huge_mushgloom_bonemeal",
        _ => return None,
    })
}

// ---------------------------------------------------------------------------------------------
// Survival and shapes
// ---------------------------------------------------------------------------------------------

/// FloorAndSidesMushroomBlock.canSurvive: the face it sits on is sturdy.
fn fungus_survives(level: &dyn LevelHost, pos: [i32; 3], st: &St) -> bool {
    let toward = if st.is("floor", "true") { "down" } else { opposite(st.get("facing").unwrap_or("north")) };
    sturdy(&lst(level, at(pos, toward)))
}

/// RopeLadderBlock.canSurvive: a sturdy wall behind or a rope ladder above.
fn ladder_survives(level: &dyn LevelHost, pos: [i32; 3], st: &St) -> bool {
    let wall = at(pos, opposite(st.get("facing").unwrap_or("north")));
    sturdy(&lst(level, wall)) || lst(level, at(pos, "up")).is_some_and(|a| a.id == st.id)
}

/// RopeLadderBlock.isTop / isEnd.
fn ladder_shape(level: &dyn LevelHost, pos: [i32; 3], st: &St) -> St {
    let above = lst(level, at(pos, "up")).is_some_and(|a| a.id == st.id);
    let below = lst(level, at(pos, "down")).is_some_and(|a| a.id == st.id);
    st.with("top", b(!above)).with("end", b(!below && above))
}

const ROOTS: &str = "spelunkery:tangle_roots";
const ROOTS_PLANT: &str = "spelunkery:tangle_roots_plant";

fn is_roots(st: &Option<St>) -> bool {
    st.as_ref().is_some_and(|s| s.id == ROOTS || s.id == ROOTS_PLANT)
}

/// GrowingPlantBlock.canSurvive (downwards): the block above is of the plant or sturdy.
fn roots_survive(level: &dyn LevelHost, pos: [i32; 3]) -> bool {
    let above = lst(level, at(pos, "up"));
    is_roots(&above) || sturdy(&above)
}

/// TangleRootsBodyBlock.isTop: the block above isn't the plant and is sturdy.
fn roots_top(level: &dyn LevelHost, pos: [i32; 3]) -> bool {
    let above = lst(level, at(pos, "up"));
    !is_roots(&above) && sturdy(&above)
}

/// SaltBlock.getConnectingSide, as redstone dust: salt beside it (side), up the face of the block beside (up), or down
/// it (side).
fn salt_connections(level: &dyn LevelHost, pos: [i32; 3], st: &St) -> St {
    let salt = |s: &Option<St>| s.as_ref().is_some_and(|s| s.id == st.id);
    let above_open = !sturdy(&lst(level, at(pos, "up")));
    let mut o = st.clone();
    for d in HORIZONTALS {
        let side = at(pos, d);
        let here = lst(level, side);
        let v = if salt(&here) {
            "side"
        } else if above_open && sturdy(&here) && salt(&lst(level, at(side, "up"))) {
            "up"
        } else if !sturdy(&here) && salt(&lst(level, at(side, "down"))) {
            "side"
        } else {
            "none"
        };
        o.set(d, v);
    }
    o
}

// ---------------------------------------------------------------------------------------------
// The part
// ---------------------------------------------------------------------------------------------

struct Rules;

fn place(level: &dyn LevelHost, event: &StateForPlacementEvent<'_>) -> Option<Option<St>> {
    let st = St::parse(&level.state_string(event.state)?);
    let ctx = event.context;
    let pos = bp(ctx.pos);
    let face = dir_name(ctx.face);
    let wet = is_water(&lst(level, pos));
    let n = match st.class() {
        // FloorAndSidesMushroomBlock.getStateForPlacement: a side face hangs it on that wall, else on the floor.
        "FloorAndSidesMushroomBlock" | "ConkFungusBlock" => {
            let s = if face == "up" || face == "down" { st.with("floor", "true") } else { st.with("floor", "false").with("facing", face) };
            if !fungus_survives(level, pos, &s) {
                return Some(None);
            }
            s
        }
        // RopeLadderBlock.getStateForPlacement: against a wall, or hanging under a rope ladder with its facing.
        "RopeLadderBlock" => {
            let above = lst(level, at(pos, "up"));
            let mut s = st.with("waterlogged", b(wet));
            let mut found = None;
            let looking = opposite(horizontal(ctx.yaw));
            let mut order = vec![looking];
            order.extend(HORIZONTALS.iter().copied().filter(|d| *d != looking));
            for d in order {
                let t = s.with("facing", d);
                if sturdy(&lst(level, at(pos, opposite(d)))) {
                    found = Some(t);
                    break;
                }
            }
            if let Some(a) = above.as_ref().filter(|a| a.id == st.id) {
                found = Some(s.with("facing", a.get("facing").unwrap_or("north")));
            }
            let Some(t) = found else { return Some(None) };
            s = t;
            ladder_shape(level, pos, &s)
        }
        "TangleRootsBodyBlock" => st.with("top", b(roots_top(level, pos))).with("waterlogged", b(wet)),
        "TangleRootsHeadBlock" => st.with("waterlogged", b(wet)),
        "SaltBlock" => salt_connections(level, pos, &st),
        "GlowstickBlock" => st.with("facing", face).with("waterlogged", b(wet)),
        // DirectionalBlock children on the stone template: the clicked face.
        "SulfuricVentBlock" | "TangleRootsBlockBlock" | "NephriteDiodeBlock" | "CompressionBlastMiner"
        | "MineomiteBlock" => {
            let s = st.with("facing", face);
            if st.get("waterlogged").is_some() { s.with("waterlogged", b(wet)) } else { s }
        }
        "NephriteFountainBlock" => st.with("facing", opposite(horizontal(ctx.yaw))),
        _ => return None,
    };
    Some(Some(n))
}

#[mod_api::horizon_part]
impl BlockPart for Rules {
    /// DropExperienceBlock with no XP range: none, though the vanilla template would drop some.
    fn exp_drop(&self, level: &dyn LevelHost, event: &mut mod_api::ExpDropEvent<'_>) -> Option<i32> {
        let st = St::parse(&level.state_string(event.state)?);
        classes::NO_XP.contains(&st.name()).then_some(0)
    }

    fn random_tick(&self, level: &mut dyn LevelWriteHost, event: &mut RandomTickEvent) -> mod_api::Flow {
        let Some(st) = level.state_string(event.state).map(|t| St::parse(&t)) else { return mod_api::Flow::Continue };
        let pos = bp(event.pos);
        match st.name() {
            // ModMushroomBlock.randomTick for the three that grow into each other; the inkcaps spread as the
            // template does.
            "button_mushroom" | "crimini" | "portabella" => {
                if lrand(level, 25) == 0 {
                    mushroom_tick(level, pos, &st);
                }
                mod_api::Flow::Cancel
            }
            // MillyBubcapMushroomBlock has no random tick (the template's spreading).
            "milly_bubcap" => mod_api::Flow::Cancel,
            // GrowingPlantHeadBlock.randomTick: one time in ten, down into air or water, until age 25.
            "tangle_roots" => {
                if st.int("age") < 25 && lrand(level, 10) == 0 {
                    grow_roots(level, pos, &st, 1);
                }
                mod_api::Flow::Cancel
            }
            "tangle_roots_plant" => mod_api::Flow::Cancel,
            _ => mod_api::Flow::Continue,
        }
    }

    fn is_valid_bonemeal_target(&self, level: mod_api::LevelSide, event: &mut mod_api::IsValidBonemealTargetEvent) -> Option<bool> {
        let (mod_api::LevelSide::Server(level) | mod_api::LevelSide::Client(level)) = level;
        let st = St::parse(&level.state_string(event.state)?);
        let pos = bp(event.pos);
        match st.class() {
            "GrowableMushroomBlock" | "MillyBubcapMushroomBlock" | "SporophyteBlock" => Some(true),
            "ModMushroomBlock" | "ConkFungusBlock" | "TangleRootsBodyBlock" => Some(false),
            "FloorAndSidesMushroomBlock" => {
                Some(st.is("floor", "true") && (mushroom_ground(&lst(level, at(pos, "down"))) || pos[1] < 0))
            }
            "TangleRootsHeadBlock" => {
                let below = lst(level, at(pos, "down"));
                Some(is_air(&below) || is_water(&below))
            }
            _ => None,
        }
    }

    fn is_bonemeal_success(&self, level: &mut dyn mod_api::LevelRandomHost, event: &mut mod_api::IsBonemealSuccessEvent) {
        let Some(st) = level.state_string(event.state).map(|t| St::parse(&t)) else { return };
        match st.class() {
            "GrowableMushroomBlock" | "FloorAndSidesMushroomBlock" => event.success = lfloat(level) < 0.4,
            "MillyBubcapMushroomBlock" | "SporophyteBlock" | "TangleRootsHeadBlock" => event.success = true,
            _ => {}
        }
    }

    fn perform_bonemeal(&self, level: &mut dyn LevelWriteHost, event: &mut PerformBonemealEvent) {
        let Some(st) = level.state_string(event.state).map(|t| St::parse(&t)) else { return };
        let pos = bp(event.pos);
        let dim = event.dimension.key().id().to_string();
        match st.class() {
            "GrowableMushroomBlock" | "FloorAndSidesMushroomBlock" => {
                if let Some(f) = huge_of(st.name()) {
                    queue_feature(&dim, pos, &st, f);
                }
            }
            "MillyBubcapMushroomBlock" => bubcap_bonemeal(level, &dim, pos, &st),
            // SporophyteBlock.performBonemeal: the tall sporophyte, when the cell above is free.
            "SporophyteBlock" => {
                if !is_air(&lst(&*level, at(pos, "up"))) {
                    return;
                }
                let Some(tall) = ldefault(&*level, &format!("{NS}tall_sporophyte")) else { return };
                lset(level, pos, &tall.with("half", "lower"));
                lset(level, at(pos, "up"), &tall.with("half", "upper"));
            }
            // NetherVines.getBlocksToGrowWhenBonemealed: 1 + nextInt(4) in practice.
            "TangleRootsHeadBlock" => {
                let n = 1 + lrand(level, 4);
                grow_roots(level, pos, &st, n);
            }
            _ => {}
        }
    }

    fn can_survive(&self, level: &dyn LevelHost, event: &mut mod_api::CanSurviveEvent) -> Option<bool> {
        let st = St::parse(&level.state_string(event.state)?);
        let pos = bp(event.pos);
        match st.class() {
            "FloorAndSidesMushroomBlock" | "ConkFungusBlock" => Some(fungus_survives(level, pos, &st)),
            "RopeLadderBlock" => Some(ladder_survives(level, pos, &st)),
            "MillyBubcapMushroomBlock" => Some(sturdy(&lst(level, at(pos, "down")))),
            "TangleRootsHeadBlock" | "TangleRootsBodyBlock" => Some(roots_survive(level, pos)),
            _ => None,
        }
    }

    fn state_for_placement(&self, level: &dyn LevelHost, event: &mut StateForPlacementEvent<'_>) -> Option<Option<mod_api::BlockState>> {
        let n = place(level, event)?;
        if debug() {
            eprintln!("[spelunkery] place {:?}", n.as_ref().map(St::text));
        }
        Some(match n {
            Some(s) => Some(level.block_state_id(&s.text())?),
            None => None,
        })
    }

    fn update_shape(&self, level: &mut dyn LevelTicksHost, event: &mut UpdateShapeEvent) -> Option<mod_api::BlockState> {
        let reader: &dyn LevelHost = &*level;
        let st = St::parse(&reader.state_string(event.state)?);
        let pos = bp(event.pos);
        let air = || reader.block_state_id("minecraft:air");
        let n = match st.class() {
            "FloorAndSidesMushroomBlock" | "ConkFungusBlock" => {
                if !fungus_survives(reader, pos, &st) {
                    return air();
                }
                return None;
            }
            "MillyBubcapMushroomBlock" => {
                if !sturdy(&lst(reader, at(pos, "down"))) {
                    return air();
                }
                return None;
            }
            "RopeLadderBlock" => {
                if !ladder_survives(reader, pos, &st) {
                    return air();
                }
                ladder_shape(reader, pos, &st)
            }
            "TangleRootsHeadBlock" => {
                if !roots_survive(reader, pos) {
                    return air();
                }
                return None;
            }
            "TangleRootsBodyBlock" => {
                if !roots_survive(reader, pos) {
                    return air();
                }
                if !is_roots(&lst(reader, at(pos, "down"))) {
                    let mut head = ldefault(reader, ROOTS)?.with("age", "25");
                    head.set("waterlogged", st.get("waterlogged").unwrap_or("false"));
                    return reader.block_state_id(&head.text());
                }
                st.with("top", b(roots_top(reader, pos)))
            }
            "SaltBlock" => {
                if !sturdy(&lst(reader, at(pos, "down"))) {
                    return air();
                }
                salt_connections(reader, pos, &st)
            }
            _ => return None,
        };
        (n != st).then(|| reader.block_state_id(&n.text())).flatten()
    }

    fn use_without_item(&self, host: &mut dyn ServerHost, event: &mut UseWithoutItemEvent<'_>) -> Option<InteractionResult> {
        let click = event.click;
        let st = St::parse(&host.block_state_string(event.state)?);
        let dim = click.dimension.key().id().to_string();
        let level: LevelRef = host.level(&dim)?;
        let pos = click.pos;
        match st.class() {
            // FlowerPotBlock.use with an empty hand: the plant back, an empty pot left.
            "FlowerPotBlock" => {
                let content = format!("{NS}{}", st.name().strip_prefix("potted_")?);
                give(host, click.player, &level, pos, &content);
                let pot = host.block_state_id("minecraft:flower_pot")?;
                let _ = level.set_block(host, pos, pot, BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS);
                Some(InteractionResult::Success)
            }
            // SaltLampBlock.use: lit cycles.
            "SaltLampBlock" => {
                let n = st.with("lit", b(!st.is("lit", "true")));
                let s = host.block_state_id(&n.text())?;
                let _ = level.set_block(host, pos, s, BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS);
                Some(InteractionResult::Success)
            }
            "RopeLadderBlock" => retract_ladder(host, click.player, &level, pos, &st),
            _ => None,
        }
    }

    fn use_item_on(&self, host: &mut dyn ServerHost, event: &mut UseItemOnEvent<'_>) -> Option<InteractionResult> {
        let click = event.click;
        let stack = click.stack.filter(|s| s.count() > 0)?;
        let held = stack.item().id(&*host).to_string();
        let st = St::parse(&host.block_state_string(event.state)?);
        let dim = click.dimension.key().id().to_string();
        let level: LevelRef = host.level(&dim)?;
        let pos = click.pos;
        match st.class() {
            "RopeLadderBlock" if held == st.id => {
                if sneaking(&*host, click.player) {
                    return None;
                }
                extend_ladder(host, click, &level, pos, &st, stack)
            }
            "RopeLadderBlock" if sneaking(&*host, click.player) => retract_ladder(host, click.player, &level, pos, &st),
            // MillyBubcapMushroomBlock.getStateForPlacement on itself: one more cap.
            "MillyBubcapMushroomBlock" if held == st.id && !sneaking(&*host, click.player) && st.int("caps") < 8 => {
                let n = st.with("caps", &(st.int("caps") + 1).to_string());
                let s = host.block_state_id(&n.text())?;
                let _ = level.set_block(host, pos, s, BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS);
                use_up_hand(host, click.player, click.hand, stack, 1);
                level.play_sound(host, "minecraft:block.fungus.place", [pos[0] as f64 + 0.5, pos[1] as f64 + 0.5, pos[2] as f64 + 0.5],
                    SoundSource::Blocks, 1.0, 1.0);
                Some(InteractionResult::Success)
            }
            "DiamondGrindstoneBlock" => use_grindstone(host, click, Some(&st), stack, &held),
            _ => None,
        }
    }
}

/// ModMushroomBlock.randomTick (the one in 25 already drawn).
fn mushroom_tick(level: &mut dyn LevelWriteHost, pos: [i32; 3], st: &St) {
    match st.name() {
        "button_mushroom" => {
            if let Some(n) = ldefault(&*level, &format!("{NS}crimini")) {
                lset(level, pos, &n);
            }
            return;
        }
        "crimini" => {
            if let Some(n) = ldefault(&*level, &format!("{NS}portabella")) {
                lset(level, pos, &n);
            }
            return;
        }
        _ => {}
    }
    // Portabella: at most 5 of it in the 9x3x9 around, then a button mushroom at a free spot nearby it survives on.
    let mut left = 5;
    for dx in -4..=4 {
        for dy in -1..=1 {
            for dz in -4..=4 {
                if lst(&*level, add(pos, [dx, dy, dz])).is_some_and(|s| s.id == st.id) {
                    left -= 1;
                    if left <= 0 {
                        return;
                    }
                }
            }
        }
    }
    let mut p = pos;
    let step = |level: &mut dyn LevelWriteHost, from: [i32; 3]| {
        let dx = lrand(level, 3) - 1;
        let dy = lrand(level, 2) - lrand(level, 2);
        let dz = lrand(level, 3) - 1;
        add(from, [dx, dy, dz])
    };
    let mut q = step(level, p);
    for _ in 0..4 {
        if is_air(&lst(&*level, q)) && mushroom_survives(&*level, q) {
            p = q;
        }
        q = step(level, p);
    }
    if is_air(&lst(&*level, q)) && mushroom_survives(&*level, q) {
        if let Some(n) = ldefault(&*level, &format!("{NS}button_mushroom")) {
            lset(level, q, &n);
        }
    }
}

/// MushroomBlock.canSurvive: mushroom ground below, or dark and a solid block below.
fn mushroom_survives(level: &dyn LevelHost, pos: [i32; 3]) -> bool {
    let below = lst(level, at(pos, "down"));
    mushroom_ground(&below) || level.raw_brightness(pos, 0) < 13 && sturdy(&below)
}

/// GrowingPlantHeadBlock growth: `n` blocks down while there's air or water, the old tip becoming the body.
fn grow_roots(level: &mut dyn LevelWriteHost, pos: [i32; 3], st: &St, n: i32) {
    let mut tip = pos;
    let mut age = st.int("age");
    for _ in 0..n {
        let below = at(tip, "down");
        let bst = lst(&*level, below);
        if !(is_air(&bst) || is_water(&bst)) || age >= 25 {
            break;
        }
        age += 1;
        let Some(head) = ldefault(&*level, ROOTS) else { return };
        let head = head.with("age", &age.to_string()).with("waterlogged", b(is_water(&bst)));
        let Some(body) = ldefault(&*level, ROOTS_PLANT) else { return };
        let here_wet = lst(&*level, tip).is_some_and(|s| s.is("waterlogged", "true"));
        let top = roots_top(&*level, tip);
        lset(level, below, &head);
        lset(level, tip, &body.with("waterlogged", b(here_wet)).with("top", b(top)));
        tip = below;
    }
}

/// MillyBubcapMushroomBlock.performBonemeal: bubcaps with random caps on free mushroom ground in a diamond around
/// (one in six), this one at 8 caps; at 8 caps already, the huge bubcap patch grows.
fn bubcap_bonemeal(level: &mut dyn LevelWriteHost, dim: &str, pos: [i32; 3], st: &St) {
    if st.int("caps") == 8 {
        queue_feature(dim, pos, st, "huge_milly_bubcap_bonemeal");
    }
    let mut j = 1;
    let mut n = 0;
    let m = pos[0] - 2;
    for (l, o) in (0..5).enumerate() {
        for p in 0..j {
            let q = 2 + pos[1] - 1;
            for r in (q - 2)..q {
                let at_ = [m + o, r, pos[2] - n + p];
                if at_ != pos && lrand(level, 6) == 0 && is_air(&lst(&*level, at_)) && mushroom_ground(&lst(&*level, at(at_, "down"))) {
                    let caps = (lrand(level, 8) + 1).to_string();
                    lset(level, at_, &st.with("caps", &caps));
                }
            }
        }
        if l < 2 {
            j += 2;
            n += 1;
        } else {
            j -= 2;
            n -= 1;
        }
    }
    lset(level, pos, &st.with("caps", "8"));
}

// ---------------------------------------------------------------------------------------------
// Clicks
// ---------------------------------------------------------------------------------------------

fn sneaking(host: &dyn ServerHost, player: u128) -> bool {
    host.player(player).and_then(|p| p.entity().is_shift_key_down(host)) == Some(true)
}

fn creative(host: &dyn ServerHost, player: u128) -> bool {
    host.player(player).and_then(|p| p.is_creative(host)) == Some(true)
}

/// The stack in `hand` `n` smaller (not in creative).
fn use_up_hand(host: &mut dyn ServerHost, player: u128, hand: Hand, stack: &ItemStack, n: u32) {
    if creative(host, player) {
        return;
    }
    let Some(p) = host.player(player) else { return };
    let slot = match hand {
        Hand::Main => p.selected_slot(&*host).map(|s| s as u32),
        _ => Some(40),
    };
    if let Some(slot) = slot {
        let _ = p.set_inventory_slot(host, slot, stack.clone().shrink(n));
    }
}

fn give(host: &mut dyn ServerHost, player: u128, level: &LevelRef, pos: [i32; 3], id: &str) {
    let Some(key) = Identifier::parse(id).ok().and_then(|i| host.item_key(&i)) else { return };
    let Some(stack) = ItemStack::new(key, 1) else { return };
    let Some(p) = host.player(player) else { return };
    if p.give_stack(host, stack.clone()).is_err() {
        let _ = level.pop_resource(host, pos, stack);
    }
}

/// RopeLadderBlockItem.updatePlacementContext: a rope ladder clicked on a rope ladder goes under the column's bottom,
/// facing as the clicked one, if that cell is free.
fn extend_ladder(
    host: &mut dyn ServerHost, click: &mod_api::BlockClick<'_>, level: &LevelRef, pos: [i32; 3], st: &St, stack: &ItemStack,
) -> Option<InteractionResult> {
    let mut p = at(pos, "down");
    let look = |host: &dyn ServerHost, q: [i32; 3]| host_look(host, *level)(q);
    let mut n = 0;
    while look(&*host, p).is_some_and(|s| s.id == st.id) && n < 384 {
        p = at(p, "down");
        n += 1;
    }
    let target = look(&*host, p);
    if !(is_air(&target) || is_water(&target)) {
        return Some(InteractionResult::Fail);
    }
    let mut new = st.with("waterlogged", b(is_water(&target))).with("top", "false");
    let below = look(&*host, at(p, "down"));
    new.set("end", b(!below.is_some_and(|s| s.id == st.id)));
    let s = host.block_state_id(&new.text())?;
    let _ = level.set_block(host, p, s, BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS);
    use_up_hand(host, click.player, click.hand, stack, 1);
    level.play_sound(host, "minecraft:block.ladder.place", [p[0] as f64 + 0.5, p[1] as f64 + 0.5, p[2] as f64 + 0.5],
        SoundSource::Blocks, 1.0, 1.0);
    Some(InteractionResult::Success)
}

/// RopeLadderBlock.use, sneaking on the top rung: the bottom rung goes back to the player.
fn retract_ladder(host: &mut dyn ServerHost, player: u128, level: &LevelRef, pos: [i32; 3], st: &St) -> Option<InteractionResult> {
    if !sneaking(&*host, player) || !st.is("top", "true") {
        return None;
    }
    let look = |host: &dyn ServerHost, q: [i32; 3]| host_look(host, *level)(q);
    let mut p = pos;
    for _ in 0..32 {
        if !look(&*host, at(p, "down")).is_some_and(|s| s.id == st.id) {
            break;
        }
        p = at(p, "down");
    }
    let air = host.block_state_id("minecraft:air")?;
    let _ = level.set_block(host, p, air, BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS);
    if !creative(&*host, player) {
        give(host, player, level, p, &st.id);
    }
    Some(InteractionResult::Success)
}

/// FlowerPotBlock.use: an empty flower pot clicked with one of the jar's plants becomes its potted block (26.3's pot
/// only knows vanilla's plants), the stack one smaller outside creative.
struct PotPlant(HashMap<String, String>);

#[mod_api::horizon_part]
impl BlockPart for PotPlant {
    fn use_item_on(&self, host: &mut dyn ServerHost, event: &mut UseItemOnEvent<'_>) -> Option<InteractionResult> {
        let click = event.click;
        let stack = click.stack.filter(|s| s.count() > 0)?;
        let held = stack.item().id(&*host).to_string();
        let potted = self.0.get(&held)?;
        let dim = click.dimension.key().id().to_string();
        let level: LevelRef = host.level(&dim)?;
        let id = host.block_state_id(potted)?;
        let _ = level.set_block(host, click.pos, id, BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS);
        use_up_hand(host, click.player, click.hand, stack, 1);
        Some(InteractionResult::Success)
    }
}

// ---------------------------------------------------------------------------------------------
// Grindstones
// ---------------------------------------------------------------------------------------------

/// The jar's `gameplay/diamond_grindstone_polishing` tables (data/spelunkery/loot_table/gameplay): item path, then
/// (result, min, max) per pool. `experience_bottle` pools are experience points, `bedrock` pools wear the grindstone.
/// Item path, then (result, min, max) per pool.
type Polish = &'static [(&'static str, &'static [(&'static str, i32, i32)])];

const DIAMOND_POLISH: Polish = &[
    ("rough_cinnabar", &[("spelunkery:cinnabar", 1, 1), ("minecraft:redstone", 2, 4), ("minecraft:experience_bottle", 2, 3), ("minecraft:bedrock", 1, 1)]),
    ("rough_diamond", &[("minecraft:diamond", 1, 1), ("minecraft:experience_bottle", 2, 4), ("minecraft:bedrock", 1, 1)]),
    ("rough_emerald", &[("minecraft:emerald", 1, 1), ("minecraft:experience_bottle", 2, 4), ("minecraft:bedrock", 1, 1)]),
    ("rough_lazurite", &[("minecraft:lapis_lazuli", 1, 2), ("spelunkery:raw_gold_nugget", 1, 3), ("minecraft:experience_bottle", 3, 5), ("minecraft:bedrock", 1, 1)]),
];

/// The jar's `gameplay/grindstone_polishing` tables, the same shape.
const POLISH: Polish = &[
    ("andesite", &[("minecraft:polished_andesite", 1, 1)]),
    ("andesite_slab", &[("minecraft:polished_andesite_slab", 1, 1)]),
    ("andesite_stairs", &[("minecraft:polished_andesite_stairs", 1, 1)]),
    ("basalt", &[("minecraft:polished_basalt", 1, 1)]),
    ("blackstone", &[("minecraft:polished_blackstone", 1, 1)]),
    ("blackstone_slab", &[("minecraft:polished_blackstone_slab", 1, 1)]),
    ("blackstone_stairs", &[("minecraft:polished_blackstone_stairs", 1, 1)]),
    ("blackstone_wall", &[("minecraft:polished_blackstone_wall", 1, 1)]),
    ("cobbled_deepslate", &[("minecraft:polished_deepslate", 1, 1)]),
    ("cobbled_deepslate_slab", &[("minecraft:polished_deepslate_slab", 1, 1)]),
    ("cobbled_deepslate_stairs", &[("minecraft:polished_deepslate_stairs", 1, 1)]),
    ("cobbled_deepslate_wall", &[("minecraft:polished_deepslate_wall", 1, 1)]),
    ("deepslate", &[("minecraft:polished_deepslate", 1, 1)]),
    ("diorite", &[("minecraft:polished_diorite", 1, 1)]),
    ("diorite_slab", &[("minecraft:polished_diorite_slab", 1, 1)]),
    ("diorite_stairs", &[("minecraft:polished_diorite_stairs", 1, 1)]),
    ("granite", &[("minecraft:polished_granite", 1, 1)]),
    ("granite_slab", &[("minecraft:polished_granite_slab", 1, 1)]),
    ("granite_stairs", &[("minecraft:polished_granite_stairs", 1, 1)]),
    ("lapis_lazuli", &[("minecraft:blue_dye", 1, 3)]),
    ("nephrite", &[("spelunkery:polished_nephrite", 1, 1)]),
    ("nephrite_slab", &[("spelunkery:polished_nephrite_slab", 1, 1)]),
    ("nephrite_stairs", &[("spelunkery:polished_nephrite_stairs", 1, 1)]),
    ("nephrite_wall", &[("spelunkery:polished_nephrite_wall", 1, 1)]),
    ("rock_salt", &[("spelunkery:salt", 1, 3)]),
    ("rock_salt_block", &[("spelunkery:polished_rock_salt", 1, 1)]),
    ("rock_salt_slab", &[("spelunkery:polished_rock_salt_slab", 1, 1)]),
    ("rock_salt_stairs", &[("spelunkery:polished_rock_salt_stairs", 1, 1)]),
    ("rock_salt_wall", &[("spelunkery:polished_rock_salt_wall", 1, 1)]),
    ("rough_cinnabar", &[("spelunkery:cinnabar", 1, 1), ("minecraft:redstone", 1, 3), ("minecraft:experience_bottle", 1, 3)]),
    ("rough_lazurite", &[("minecraft:lapis_lazuli", 1, 2), ("spelunkery:raw_gold_nugget", 0, 2), ("minecraft:experience_bottle", 1, 5)]),
];

/// CommonConfigs.DIAMOND_GRINDSTONE_DEPLETE_CHANCE (default 64): each bedrock roll wears the diamond grindstone one
/// step with chance 1 in this.
const DEPLETE_CHANCE: i32 = 64;

/// ModEvents.useGrindstone (DiamondGrindstoneBlock.use, and GrindstoneMixin on the vanilla grindstone). An item with a
/// polishing table is used up and the table's items go to the player (or drop), each experience bottle in it is one
/// experience point, and on a diamond grindstone that isn't worn out (depletion < 3) the diamond table comes first,
/// with each bedrock roll a 1 in 64 chance to raise depletion. A worn-out diamond grindstone and the vanilla one use
/// the plain table. On the vanilla grindstone, items without a table fall through to its menu, as an empty hand does;
/// the diamond grindstone opens no menu (the mod API can't open a vanilla station's menu for a mod block).
/// ModEvents.disenchant's repair branch: a (rough) diamond shard clicked on a worn diamond grindstone sets depletion
/// back to 0 (the jar asks for sneaking there; here sneaking isn't needed, because 26.3 skips the block's item click
/// while sneaking with an item in hand). Disenchanting stays the grindstone menu's.
fn use_grindstone(
    host: &mut dyn ServerHost, click: &mod_api::BlockClick<'_>, st: Option<&St>, stack: &ItemStack, held: &str,
) -> Option<InteractionResult> {
    let dim = click.dimension.key().id().to_string();
    let level: LevelRef = host.level(&dim)?;
    let pos = click.pos;
    let depletion = st.map(|s| s.int("depletion"));
    if let (Some(s), Some(d)) = (st, depletion)
        && d > 0
        && matches!(held, "spelunkery:rough_diamond_shard" | "spelunkery:diamond_shard")
    {
        let n = s.with("depletion", "0");
        let id = host.block_state_id(&n.text())?;
        let _ = level.set_block(host, pos, id, BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS);
        use_up_hand(host, click.player, click.hand, stack, 1);
        return Some(InteractionResult::Success);
    }
    let name = held.strip_prefix("spelunkery:").or_else(|| held.strip_prefix("minecraft:"))?;
    let sharp = depletion.is_some_and(|d| d < 3);
    let diamond = if sharp { DIAMOND_POLISH.iter().find(|t| t.0 == name) } else { None };
    let (_, pools) = diamond.or_else(|| POLISH.iter().find(|t| t.0 == name))?;
    let rand = |host: &mut dyn ServerHost, bound: i32| {
        level.with_random(host, |r| mod_api::RandomSource::next_int_bound(r, bound).unwrap_or(0)).unwrap_or(0)
    };
    let centre = [pos[0] as f64 + 0.5, pos[1] as f64 + 0.5, pos[2] as f64 + 0.5];
    let mut wear = 0;
    for &(item, min, max) in pools.iter() {
        let n = min + rand(host, max - min + 1);
        match item {
            "minecraft:experience_bottle" => {
                for _ in 0..n {
                    let mut nbt = Compound::new();
                    nbt.insert("Value".into(), Tag::Short(1));
                    host.summon(&dim, "minecraft:experience_orb", centre, nbt);
                }
            }
            "minecraft:bedrock" => wear += n,
            _ if n > 0 => give_n(host, click.player, &level, pos, item, n as u32),
            _ => {}
        }
    }
    if diamond.is_some()
        && let (Some(s), Some(d)) = (st, depletion)
        && (0..wear).filter(|_| rand(host, DEPLETE_CHANCE) == 1).count() > 0
    {
        let n = s.with("depletion", &(d + 1).to_string());
        if let Some(id) = host.block_state_id(&n.text()) {
            let _ = level.set_block(host, pos, id, BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS);
        }
    }
    use_up_hand(host, click.player, click.hand, stack, 1);
    level.play_sound(host, "minecraft:block.grindstone.use", centre, SoundSource::Blocks, 0.5, 0.5);
    Some(InteractionResult::Success)
}

fn give_n(host: &mut dyn ServerHost, player: u128, level: &LevelRef, pos: [i32; 3], id: &str, n: u32) {
    let Some(key) = Identifier::parse(id).ok().and_then(|i| host.item_key(&i)) else { return };
    let Some(stack) = ItemStack::new(key, n) else { return };
    let Some(p) = host.player(player) else { return };
    if p.give_stack(host, stack.clone()).is_err() {
        let _ = level.pop_resource(host, pos, stack);
    }
}

/// GrindstoneMixin: the vanilla grindstone polishes with the plain tables.
struct Grindstone;

#[mod_api::horizon_part]
impl BlockPart for Grindstone {
    fn use_item_on(&self, host: &mut dyn ServerHost, event: &mut UseItemOnEvent<'_>) -> Option<InteractionResult> {
        let click = event.click;
        let stack = click.stack.filter(|s| s.count() > 0)?;
        let held = stack.item().id(&*host).to_string();
        use_grindstone(host, click, None, stack, &held)
    }
}

// ---------------------------------------------------------------------------------------------
// Items
// ---------------------------------------------------------------------------------------------

struct Thrown {
    dim: String,
    id: u32,
    last: [f64; 3],
    block: String,
}

static THROWN: Mutex<Vec<Thrown>> = Mutex::new(Vec::new());

/// GlowstickItem.use: thrown (velocity 1.5) with the snowball sound; ThrownGlowstickEntity places its glowstick
/// where it lands. Here a snowball carrying the glowstick flies, and `server_tick` sets the block where it broke.
struct GlowstickItem(String);

#[mod_api::horizon_part]
impl ItemPart for GlowstickItem {
    fn use_item(&self, host: mod_api::HostSide, event: &mut mod_api::UseItemEvent<'_>) -> Option<InteractionResult> {
        let mod_api::HostSide::Server(host) = host else { return Some(InteractionResult::Success) };
        let p = host.player(event.player)?;
        let dim = p.dimension(&*host)?.to_string();
        let ent = p.entity();
        let eye = ent.eye_position(&*host)?;
        let look = ent.view_vector(&*host)?;
        let mut item = Compound::new();
        item.insert("id".into(), Tag::String(self.0.as_str().into()));
        item.insert("count".into(), Tag::Int(1));
        let mut nbt = Compound::new();
        nbt.insert("Item".into(), Tag::Compound(item));
        nbt.insert("Motion".into(), Tag::List(look.iter().map(|c| Tag::Double((c * 1.5).to_bits())).collect()));
        let u = event.player;
        let owner = [(u >> 96) as u32 as i32, (u >> 64) as u32 as i32, (u >> 32) as u32 as i32, u as u32 as i32];
        nbt.insert("Owner".into(), Tag::IntArray(owner.to_vec()));
        let start = [eye[0], eye[1] - 0.1, eye[2]];
        let id = host.summon(&dim, "minecraft:snowball", start, nbt)?;
        lock(&THROWN).push(Thrown { dim: dim.clone(), id, last: start, block: self.0.clone() });
        if let Some(level) = host.level(&dim) {
            level.play_sound(host, "minecraft:entity.snowball.throw", start, SoundSource::Neutral, 0.5, 0.45);
        }
        let stack = event.stack.clone();
        use_up_hand(host, event.player, event.hand, &stack, 1);
        Some(InteractionResult::Success)
    }
}

/// DepthGaugeItem.use: "Depth: Y" over the hotbar.
struct DepthGauge;

#[mod_api::horizon_part]
impl ItemPart for DepthGauge {
    fn use_item(&self, host: mod_api::HostSide, event: &mut mod_api::UseItemEvent<'_>) -> Option<InteractionResult> {
        let mod_api::HostSide::Server(host) = host else { return Some(InteractionResult::Success) };
        let p = host.player(event.player)?;
        let y = p.entity().position(&*host)?[1].floor() as i32;
        let text = mod_api::Text::translate("tooltip.spelunkery.depth_gauge_depth", vec![mod_api::Text::literal(y.to_string())]);
        let _ = p.action_bar(host, text);
        Some(InteractionResult::Success)
    }
}

const GLINT: &str = "minecraft:enchantment_glint_override";

fn magnet_on(stack: &ItemStack) -> bool {
    stack.components().is_some_and(|c| c.as_nbt().contains_key(&GLINT.into()))
}

/// MagnetItem.use: on or off (shown by its glint), with the beacon sound.
struct Magnet;

#[mod_api::horizon_part]
impl ItemPart for Magnet {
    fn use_item(&self, host: mod_api::HostSide, event: &mut mod_api::UseItemEvent<'_>) -> Option<InteractionResult> {
        let mod_api::HostSide::Server(host) = host else { return Some(InteractionResult::Success) };
        let p = host.player(event.player)?;
        let mut stack = event.stack.clone();
        let on = magnet_on(&stack);
        let mut patch = Compound::new();
        if on {
            patch.insert(format!("!{GLINT}").as_str().into(), Tag::Compound(Compound::new()));
        } else {
            patch.insert(GLINT.into(), Tag::Byte(1));
        }
        stack.apply_components(&patch).ok()?;
        let slot = match event.hand {
            Hand::Main => p.selected_slot(&*host).map(|s| s as u32),
            _ => Some(40),
        }?;
        let _ = p.set_inventory_slot(host, slot, Some(stack));
        let dim = p.dimension(&*host)?.to_string();
        let at_ = p.entity().position(&*host)?;
        if let Some(level) = host.level(&dim) {
            let snd = if on { "minecraft:block.beacon.deactivate" } else { "minecraft:block.beacon.activate" };
            level.play_sound(host, snd, at_, SoundSource::Blocks, 1.0, 2.0);
        }
        Some(InteractionResult::Success)
    }
}

/// Per player: ticks spent falling fast, whether the parachute is open.
static PARACHUTES: Mutex<Vec<(u128, u32, bool)>> = Mutex::new(Vec::new());

fn tick_players(host: &mut dyn ServerHost) {
    for dim in ["minecraft:overworld", "minecraft:the_nether", "minecraft:the_end"] {
        let Some(level) = host.level(dim) else { continue };
        let Some(players) = level.online_players(&*host) else { continue };
        for p in players {
            let Some(inv) = p.inventory(&*host) else { continue };
            let id = |s: &ItemStack| s.item().id(&*host).to_string();
            let magnet = inv.iter().any(|s| s.count() > 0 && id(s) == "spelunkery:item_magnet" && magnet_on(s));
            let chute = inv.iter().position(|s| s.count() > 0 && id(s) == "spelunkery:parachute");
            if magnet {
                pull_items(host, level, p);
            }
            if let Some(slot) = chute {
                parachute(host, p, slot as u32, inv[slot].clone());
            }
        }
    }
}

/// MagnetItem.inventoryTick: item entities within 8 blocks get pulled toward the player.
fn pull_items(host: &mut dyn ServerHost, level: LevelRef, p: mod_api::PlayerRef) {
    let Some(pos) = p.entity().position(&*host) else { return };
    let r = 8.0;
    let Some(ents) = level.entities_in(&*host, [pos[0] - r, pos[1] - r, pos[2] - r], [pos[0] + r, pos[1] + r, pos[2] + r]) else { return };
    for e in ents {
        if e.kind(&*host).map(|k| k.id(&*host).to_string()).as_deref() != Some("minecraft:item") {
            continue;
        }
        let (Some(ep), Some(v)) = (e.position(&*host), e.velocity(&*host)) else { continue };
        let d = [pos[0] - ep[0], pos[1] - ep[1], pos[2] - ep[2]];
        let len2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
        if len2 >= 64.0 || len2 < 1e-6 {
            continue;
        }
        let len = len2.sqrt();
        let f = (1.0 - len / 8.0).powi(2) * 0.1;
        let nv = [v[0] + d[0] / len * f, v[1] + d[1] / len * f, v[2] + d[2] / len * f];
        let _ = e.set_velocity(host, nv);
    }
}

/// ParachuteItem.inventoryTick: after 10 ticks falling faster than 1 a tick it opens, giving slow falling until the
/// player stands on something, then it takes one damage (and breaks at its last).
fn parachute(host: &mut dyn ServerHost, p: mod_api::PlayerRef, slot: u32, stack: ItemStack) {
    let uuid = p.uuid();
    let ent = p.entity();
    let vy = ent.velocity(&*host).map_or(0.0, |v| v[1]);
    let ground = ent.on_ground(&*host).unwrap_or(true);
    let mut all = lock(&PARACHUTES);
    let i = match all.iter().position(|x| x.0 == uuid) {
        Some(i) => i,
        None => {
            all.push((uuid, 0, false));
            all.len() - 1
        }
    };
    let (_, ticks, open) = &mut all[i];
    if !*open {
        if vy < -1.0 {
            *ticks += 1;
            if *ticks >= 10 {
                *open = true;
                *ticks = 0;
                drop(all);
                if let (Some(dim), Some(at_)) = (p.dimension(&*host), ent.position(&*host)) {
                    if let Some(level) = host.level(&dim.to_string()) {
                        level.play_sound(host, "minecraft:item.armor.equip_leather", at_, SoundSource::Blocks, 1.0, 1.0);
                    }
                }
            }
        } else {
            *ticks = 0;
        }
        return;
    }
    if ground {
        *open = false;
        drop(all);
        if !creative(&*host, uuid) {
            let max = 16;
            let next = if stack.damage() + 1 >= max { None } else {
                let mut s = stack.clone();
                s.set_damage(stack.damage() + 1);
                Some(s)
            };
            let _ = p.set_inventory_slot(host, slot, next);
        }
        return;
    }
    drop(all);
    if let Some(key) = host.effect_key("minecraft:slow_falling") {
        let eff = mod_api::ActiveEffect::new(key, 5, 0, true, false, true);
        let _ = mod_api::LivingRef::new(ent).add_effect(host, &eff);
    }
}

// ---------------------------------------------------------------------------------------------
// Loot
// ---------------------------------------------------------------------------------------------

fn vanilla_loot() -> &'static HashMap<String, Value> {
    static M: OnceLock<HashMap<String, Value>> = OnceLock::new();
    M.get_or_init(|| mod_api::serde_json::from_str(vanilla_loot::JSON).unwrap_or_default())
}

fn push_pool(table: &mut Value, pool: Value) {
    match table.get_mut("pools").and_then(|p| p.as_array_mut()) {
        Some(pools) => pools.push(pool),
        None => {
            if let Some(t) = table.as_object_mut() {
                t.insert("pools".into(), json!([pool]));
            }
        }
    }
}

const CHEST_INJECTS: &[&str] = &["abandoned_mineshaft", "ruined_portal", "stronghold_library", "ancient_city"];

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        for (name, ..) in classes::BLOCKS {
            let key = r.block_key(&format!("{NS}{name}"));
            r.block_part(key, Rules);
        }
        let pots = classes::BLOCKS
            .iter()
            .filter(|b| b.1 == "FlowerPotBlock")
            .filter_map(|b| Some((format!("{NS}{}", b.0.strip_prefix("potted_")?), format!("{NS}{}", b.0))))
            .collect();
        let k = r.block_key("minecraft:flower_pot");
        r.block_part(k, PotPlant(pots));
        let k = r.block_key("minecraft:grindstone");
        r.block_part(k, Grindstone);
        for (name, cls, _) in classes::BLOCKS {
            if *cls == "GlowstickBlock" {
                let id = format!("{NS}{name}");
                let k = r.item_key(&id);
                r.item_part(k, GlowstickItem(id));
            }
        }
        let k = r.item_key("spelunkery:depth_gauge");
        r.item_part(k, DepthGauge);
        let k = r.item_key("spelunkery:item_magnet");
        r.item_part(k, Magnet);
    }

    fn loot_table_load(&self, _context: &dyn DataLoadContext, event: &mut LootTableLoadEvent<'_>) {
        let id = event.id;
        if let Some(t) = vanilla_loot().get(id) {
            *event.table = t.clone();
            if debug() {
                eprintln!("[spelunkery] {id} replaced");
            }
            return;
        }
        if let Some(chest) = id.strip_prefix("minecraft:chests/") {
            if CHEST_INJECTS.contains(&chest) {
                push_pool(event.table, json!({"rolls": 1.0, "bonus_rolls": 0.0, "entries": [
                    {"type": "minecraft:loot_table", "value": format!("{NS}injects/{chest}")}]}));
            }
        }
    }

    fn server_tick(&self, host: &mut dyn ServerHost, event: &mut ServerTickEvent<'_>) {
        if !matches!(event.phase, ServerTickPhase::ServerTickEnd { .. }) {
            return;
        }
        let grow_now = std::mem::take(&mut *lock(&GROW));
        for (dim, pos, before, feature) in grow_now {
            grow(host, &dim, pos, &before, &feature);
        }
        tick_players(host);
        let thrown = std::mem::take(&mut *lock(&THROWN));
        if thrown.is_empty() {
            return;
        }
        let mut keep = Vec::new();
        for mut t in thrown {
            if let Some(e) = host.entity(t.id) {
                if let Some(p) = e.position(&*host) {
                    t.last = p;
                    keep.push(t);
                    continue;
                }
            }
            // ThrownGlowstickEntity.onHit: the glowstick where it landed (or the item, if that cell is taken).
            let Some(level) = host.level(&t.dim) else { continue };
            let cell = [t.last[0].floor() as i32, t.last[1].floor() as i32, t.last[2].floor() as i32];
            let here = host_look(&*host, level)(cell);
            let placed = (is_air(&here) || is_water(&here))
                && host
                    .block_state_id(&format!("{}[facing=up,waterlogged={}]", t.block, is_water(&here)))
                    .is_some_and(|s| level.set_block(host, cell, s, BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS).is_ok_and(|b| b));
            if !placed {
                let mut item = Compound::new();
                item.insert("id".into(), Tag::String(t.block.as_str().into()));
                item.insert("count".into(), Tag::Int(1));
                let mut nbt = Compound::new();
                nbt.insert("Item".into(), Tag::Compound(item));
                host.summon(&t.dim, "minecraft:item", t.last, nbt);
            }
        }
        lock(&THROWN).extend(keep);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes_known() {
        assert_eq!(class_map().get("inkcap_mushroom").map(|c| c.0), Some("GrowableMushroomBlock"));
        assert_eq!(class_map().get("rope_ladder").map(|c| c.1), Some("ladder"));
    }

    #[test]
    fn loot_parses() {
        assert!(vanilla_loot().contains_key("minecraft:blocks/diamond_ore"));
        assert!(vanilla_loot().contains_key("minecraft:gameplay/piglin_bartering"));
    }

    #[test]
    fn faces() {
        assert_eq!(face_name([0, 1, 0]), "up");
        assert_eq!(horizontal(0.0), "south");
    }
}
