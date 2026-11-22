//! mythicupgrades: Mythic Upgrades 5.1.1 (TriQue; MIT), from the decompiled jar (tools/dl/mythicupgrades/named) and the
//! registry oracle (tools/ns_oracle). data/ and assets/ are the content mod mods/mythicupgrades
//! (tools/mythicupgrades_port.py), which also writes `classes.rs`. All of it is the mod API: no game module, no mixin.
//! The numbers are MythicStats' defaults, which the live server's config/mythicupgrades.json keeps.
//!
//! Blocks (`Rules` on every block):
//! - the gem ores drop DropExperienceBlock's experience (6-14, the Nether gems 4-10), necoium none;
//! - the budding crystals (MythicBuddingCrystalBlock) grow their own buds: one random tick in five, a random side,
//!   small -> medium -> large -> cluster facing that side, or a new small bud in air or a water source; the
//!   budding amethyst template's own growth is cancelled.
//!
//! Gear (LivingEntityMixin, ServerPlayerGameModeMixin, LightningBoltMixin), on every living entity but armour stands:
//! - **armour sets**: each worn piece adds levels (helmet 2, chestplate 3, leggings 3, boots 2) to its gem's effect,
//!   kept as an infinite effect of amplifier levels - 1 while worn: sapphire Damage Deflection, ametrine Arcane Aura,
//!   topaz Topaz Reaction, ruby Blood Thirst, peridot Miasma, aquamarine Ice Shield, citrine Static Field, jade Jade
//!   Aura;
//! - **the effects** (from armour, food or the tools): Damage Deflection hurts the attacker back (5% of the damage
//!   per level, plus 0.5 per level up to 5); Arcane Aura levitates everything within level blocks when hurt; Topaz
//!   Reaction shocks everything within level blocks when hurt (0.25 damage and knockback per level) and takes 5% per
//!   level off fall damage, shocking again; Blood Thirst heals 3% per level of any damage dealt within 3 blocks per
//!   level; Miasma poisons everything within level blocks every 200 ticks (not the owner's pets or team) and cures
//!   its holder's poison; Ice Shield freezes a melee attacker; Static Field hurts everything within 0.4 blocks per
//!   level each second (0.2 per level, 10% more per hit in a row) and pushes lightning 7 blocks away; Jade Aura
//!   gives 10% speed and a 0.05 jump boost per level and leaves a trail that gives Jade Aura, Speed and Jump Boost;
//!   Freeze keeps its holder frozen; Lethal Incubation, Ice Bomb and Charged burst when their holder dies (poison
//!   shock, freezing blast, lightning); Necoium Share links holders within 30 blocks and shares their gem effects;
//! - **the tools** (on a melee hit): sapphire +5% of the target's max health (up to 15), ametrine levitation and a
//!   5% teleport, topaz a shock every third hit, ruby 30% life steal, peridot Lethal Incubation, aquamarine Freeze,
//!   Slowness and Ice Bomb, citrine Charged, Freeze and a chain hit on three more targets, jade Jade Aura on the
//!   wielder and a 5% teleport; breaking a block with jade gives Jade Aura, with ruby heals 1, with topaz shocks
//!   every third block.
//!
//! `loot_table_load`: the end city treasure chest gets the Mythic Upgrade template at 25% (MythicUpgrades'
//! LootTableEvents.MODIFY).
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use mod_api::serde_json::{json, Value};
use mod_api::{
    ActiveEffect, AttributeModifier, AttributeOperation, BlockPart, BlockUpdateFlags, DataLoadContext, EntityRef,
    Hooks, LevelHost, LevelRef, LevelWriteHost, LivingRef, LootTableLoadEvent, ModifierLifetime, RandomTickEvent,
    Registrar, RegistrationContext, ServerHost, ServerTickEvent, ServerTickPhase, SoundSource,
};

mod classes;

const NS: &str = "mythicupgrades:";

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn debug() -> bool {
    std::env::var_os("MYTHIC_DEBUG").is_some()
}

// ---------------------------------------------------------------------------------------------
// MythicStats (defaults; the live server's config holds the same values)
// ---------------------------------------------------------------------------------------------
const PIECE_LEVELS: [i32; 4] = [2, 3, 3, 2]; // helmet, chestplate, leggings, boots (every gem)
const DEFLECT_PER_LEVEL: f32 = 0.05;
const DEFLECT_BONUS_DAMAGE_PER_LEVEL: f32 = 0.5;
const DEFLECT_BONUS_DAMAGE_MAX: f32 = 5.0;
const SAPPHIRE_TOOL_PERCENT_DAMAGE: f32 = 0.05;
const SAPPHIRE_TOOL_PERCENT_DAMAGE_CAP: f32 = 15.0;
const ARCANE_AURA_RADIUS_PER_LEVEL: f32 = 1.0;
const ARCANE_AURA_LEVITATION_AMP_PER_LEVEL: i32 = 1;
const ARCANE_AURA_LEVITATION_DURATION_TICKS: i32 = 30;
const AMETRINE_TOOL_LEVITATION_AMPLIFIER: i32 = 2;
const AMETRINE_TOOL_LEVITATION_DURATION_TICKS: i32 = 20;
const AMETRINE_TOOL_TELEPORT_CHANCE: f32 = 0.05;
const TOPAZ_SHOCK_DAMAGE_PER_LEVEL: f32 = 0.25;
const TOPAZ_SHOCK_KNOCKBACK_PER_LEVEL: f32 = 0.25;
const TOPAZ_FALL_REDUCTION_PER_LEVEL: f32 = 0.05;
const TOPAZ_ARMOR_SHOCK_RADIUS_PER_LEVEL: f32 = 1.0;
const TOPAZ_TOOL_SHOCK_INTERVAL: i32 = 3;
const TOPAZ_TOOL_EFFECTIVE_LEVEL: i32 = 5;
const RUBY_TOOL_LIFESTEAL_FRACTION: f32 = 0.3;
const RUBY_TOOL_INTERACTION_HEAL: f32 = 1.0;
const BLOOD_THIRST_RADIUS_PER_LEVEL: f32 = 3.0;
const BLOOD_THIRST_HEAL_FRACTION_PER_LEVEL: f32 = 0.03;
const LETHAL_INCUBATION_TOOL_DURATION_TICKS: i32 = 100;
const LETHAL_INCUBATION_SHOCK_DAMAGE: f32 = 1.5;
const LETHAL_INCUBATION_SHOCK_RADIUS: f32 = 5.0;
const LETHAL_INCUBATION_POISON_DURATION_TICKS: i32 = 60;
const MIASMA_CLOUD_RADIUS_PER_LEVEL: f32 = 1.0;
const MIASMA_POISON_DURATION_TICKS: i32 = 60;
const MIASMA_INTERVAL_TICKS: i32 = 200;
const AQUAMARINE_TOOL_FREEZE_TICKS: i32 = 18;
const AQUAMARINE_TOOL_SLOWNESS_DURATION_TICKS: i32 = 60;
const AQUAMARINE_TOOL_SLOWNESS_AMPLIFIER: i32 = 0;
const ICE_SHIELD_FREEZE_TICKS: i32 = 10;
const ICE_BOMB_TOOL_DURATION_TICKS: i32 = 60;
const ICE_BOMB_BURST_RADIUS: f32 = 5.0;
const ICE_BOMB_BURST_DAMAGE: f32 = 4.0;
const ICE_BOMB_BURST_FREEZE_TICKS: i32 = 18;
const CITRINE_TOOL_CHARGED_DURATION_TICKS: i32 = 60;
const CITRINE_TOOL_FREEZE_TICKS: i32 = 5;
const CITRINE_TOOL_CHAIN_FRACTION: f32 = 0.333;
const CITRINE_TOOL_CHAIN_TARGETS: usize = 3;
const CITRINE_TOOL_CHAIN_RANGE: f32 = 10.0;
const CITRINE_CHARGED_LIGHTNING_MIN: i32 = 1;
const CITRINE_CHARGED_LIGHTNING_MAX: i32 = 3;
const CITRINE_CHARGED_LIGHTNING_SPREAD: f64 = 1.5;
const STATIC_FIELD_RADIUS_PER_LEVEL: f32 = 0.4;
const STATIC_FIELD_DAMAGE_PER_LEVEL_PER_SECOND: f32 = 0.2;
const CITRINE_LIGHTNING_REDIRECT_DISTANCE: f64 = 7.0;
const STATIC_FIELD_STACK_DAMAGE_INCREASE: f32 = 0.1;
const STATIC_FIELD_STACK_RESET_TICKS: i64 = 60;
const JADE_TOOL_TELEPORT_CHANCE: f32 = 0.05;
const JADE_TOOL_AURA_DURATION_TICKS: i32 = 20;
const JADE_AURA_SPEED_PER_LEVEL: f64 = 0.1;
const JADE_AURA_JUMP_PER_LEVEL: f32 = 0.05;
const JADE_TRAIL_INTERVAL_TICKS: i64 = 10;
const JADE_TRAIL_RADIUS: f32 = 1.0;
const JADE_TRAIL_DURATION_TICKS: i64 = 60;
const JADE_TRAIL_EFFECT_DURATION_TICKS: i32 = 80;
const JADE_TRAIL_LINGER_CONTACT_RADIUS: f64 = 1.0;
const JADE_TRAIL_CONTACT_EFFECT_DURATION_TICKS: i32 = 300;
const NECOIUM_RANGE: f64 = 30.0;
const NECOIUM_SHARE_DURATION: i32 = 100;
const TICKS_REQUIRED_TO_FREEZE: i32 = 140;

/// The eight gems with the effect their armour gives.
const SETS: [(&str, &str); 8] = [
    ("sapphire", "damage_deflection"),
    ("ametrine", "arcane_aura"),
    ("topaz", "topaz_reaction"),
    ("ruby", "blood_thirst"),
    ("peridot", "miasma"),
    ("aquamarine", "ice_shield"),
    ("citrine", "static_field"),
    ("jade", "jade_aura"),
];
const PIECES: [&str; 4] = ["helmet", "chestplate", "leggings", "boots"];
const TOOLS: [&str; 5] = ["sword", "pickaxe", "axe", "shovel", "hoe"];
/// NecoiumShareHandler.SHAREABLE_EFFECTS.
const SHAREABLE: [&str; 8] = [
    "damage_deflection",
    "arcane_aura",
    "topaz_reaction",
    "blood_thirst",
    "miasma",
    "static_field",
    "jade_aura",
    "ice_shield",
];
/// The jar's damage types: a hit of one of these sets off no reaction (LivingEntityMixin.onHurt).
const OWN_DAMAGE: [&str; 7] = [
    "deflecting_damage_type",
    "percentage_damage_type",
    "topaz_shock_damage_type",
    "peridot_incubation_damage_type",
    "citrine_chain_damage_type",
    "static_field_damage_type",
    "ice_bomb_burst_damage_type",
];

fn dmg(name: &str) -> String {
    format!("{NS}{name}_damage_type")
}

/// True while one of our own hits is being dealt, so living_damage_post doesn't queue it as a new hit
/// (with a vanilla stand-in type the id can't tell it apart).
static OWN_HURT: AtomicBool = AtomicBool::new(false);

/// The vanilla type that stands in for one of ours: magic for the armour-bypassing ones, generic for the rest.
fn stand_in(name: &str) -> &'static str {
    match name {
        "static_field" | "ice_bomb_burst" => "minecraft:generic",
        _ => "minecraft:magic",
    }
}

/// Deal one of the mod's damage types. Horizon dd67b76 refuses a content mod's damage type in
/// `hurt_target` (world_cmds.rs:205 checks the base catalog, `self.motion.health.damage`, not the
/// world's `health_resources`), so a refused hit is dealt again with the vanilla stand-in.
fn hurt_own(host: &mut dyn ServerHost, target: EntityRef, name: &str, amount: f32, direct: Option<EntityRef>, causing: Option<EntityRef>) -> bool {
    OWN_HURT.store(true, Ordering::Relaxed);
    let mut hurt = target.hurt(host, &dmg(name), amount, direct, causing) == Some(true);
    if !hurt {
        hurt = target.hurt(host, stand_in(name), amount, direct, causing) == Some(true);
    }
    OWN_HURT.store(false, Ordering::Relaxed);
    hurt
}

/// The gem of a Mythic tool, if the stack is one.
fn tool_gem(id: &str) -> Option<&'static str> {
    let rest = id.strip_prefix(NS)?;
    let (gem, piece) = rest.rsplit_once('_')?;
    if !TOOLS.contains(&piece) {
        return None;
    }
    SETS.iter().map(|s| s.0).find(|g| *g == gem)
}

// ---------------------------------------------------------------------------------------------
// A small random source (the hooks that roll here get no level random)
// ---------------------------------------------------------------------------------------------
static SEED: AtomicU64 = AtomicU64::new(0x9E37_79B9_7F4A_7C15);

fn next_u64() -> u64 {
    let mut x = SEED.load(Ordering::Relaxed);
    if x == 0x9E37_79B9_7F4A_7C15 {
        x ^= std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64);
    }
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    SEED.store(x, Ordering::Relaxed);
    x
}

fn rand_f64() -> f64 {
    (next_u64() >> 11) as f64 / (1u64 << 53) as f64
}

fn rand_int(bound: i32) -> i32 {
    if bound <= 0 { 0 } else { (next_u64() % bound as u64) as i32 }
}

// ---------------------------------------------------------------------------------------------
// Blocks
// ---------------------------------------------------------------------------------------------
fn block_name(level: &dyn LevelHost, state: mod_api::BlockState) -> Option<String> {
    let t = level.state_string(state)?;
    let id = t.split_once('[').map_or(t.as_str(), |x| x.0);
    id.strip_prefix(NS).map(str::to_string)
}

fn state_prop(text: &str, key: &str) -> Option<String> {
    let rest = text.split_once('[')?.1.trim_end_matches(']');
    rest.split(',').filter_map(|kv| kv.split_once('=')).find(|(k, _)| *k == key).map(|(_, v)| v.to_string())
}

struct Rules;

const DIRS: [(&str, [i32; 3]); 6] = [
    ("down", [0, -1, 0]),
    ("up", [0, 1, 0]),
    ("north", [0, 0, -1]),
    ("south", [0, 0, 1]),
    ("west", [-1, 0, 0]),
    ("east", [1, 0, 0]),
];

fn lrand(level: &mut dyn LevelWriteHost, bound: i32) -> i32 {
    level.with_random(|r| mod_api::RandomSource::next_int_bound(r, bound).unwrap_or(0)).unwrap_or(0)
}

#[mod_api::horizon_part]
impl BlockPart for Rules {
    /// DropExperienceBlock's UniformInt; necoium ore (a plain Block) drops none.
    fn exp_drop(&self, level: &dyn LevelHost, event: &mut mod_api::ExpDropEvent<'_>) -> Option<i32> {
        let name = block_name(level, event.state)?;
        if !name.ends_with("_ore") {
            return None;
        }
        Some(match classes::ORE_XP.iter().find(|o| o.0 == name) {
            Some(&(_, lo, hi)) => lo + rand_int(hi - lo + 1),
            None => 0,
        })
    }

    /// MythicBuddingCrystalBlock.randomTick.
    fn random_tick(&self, level: &mut dyn LevelWriteHost, event: &mut RandomTickEvent) -> mod_api::Flow {
        let Some(name) = block_name(&*level, event.state) else { return mod_api::Flow::Continue };
        let Some(gem) = name.strip_prefix("budding_").and_then(|n| n.strip_suffix("_crystal")) else {
            return mod_api::Flow::Continue;
        };
        if lrand(level, 5) == 0 {
            let (face, v) = DIRS[lrand(level, 6) as usize];
            let p = [event.pos.x + v[0], event.pos.y + v[1], event.pos.z + v[2]];
            let here = level.block_state(p).and_then(|s| level.state_string(s)).unwrap_or_default();
            let id = here.split_once('[').map_or(here.as_str(), |x| x.0).to_string();
            let facing_ok = state_prop(&here, "facing").as_deref() == Some(face);
            let stages = [
                format!("{NS}small_{gem}_crystal_bud"),
                format!("{NS}medium_{gem}_crystal_bud"),
                format!("{NS}large_{gem}_crystal_bud"),
                format!("{NS}{gem}_crystal_cluster"),
            ];
            let water = id == "minecraft:water" && state_prop(&here, "level").as_deref() == Some("0");
            let waterlogged = water || state_prop(&here, "waterlogged").as_deref() == Some("true");
            let next = match stages.iter().position(|s| *s == id) {
                Some(i) if i < 3 && facing_ok => Some(stages[i + 1].clone()),
                Some(_) => None,
                None if id.is_empty() || matches!(id.as_str(), "minecraft:air" | "minecraft:cave_air") || water => {
                    Some(stages[0].clone())
                }
                None => None,
            };
            if let Some(n) = next {
                let s = format!("{n}[facing={face},waterlogged={waterlogged}]");
                if let Some(st) = level.block_state_id(&s) {
                    level.set_block(p, st, BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS);
                }
            }
        }
        mod_api::Flow::Cancel
    }
}

// ---------------------------------------------------------------------------------------------
// Living state
// ---------------------------------------------------------------------------------------------
#[derive(Default)]
struct Living {
    /// The set level last applied per gem (SETS order).
    levels: [i32; 8],
    miasma_tick: i32,
    /// Static Field: target -> (hits in a row, last hit tick).
    static_stacks: HashMap<u32, (i32, i64)>,
    /// Jade trail points: (tick, position, level).
    jade_trail: Vec<(i64, [f64; 3], i32)>,
    /// The Jade Aura speed modifier's amplifier on the entity, if any.
    jade_speed: Option<i32>,
    seen: i64,
}

/// A hit the hooks saw, acted on at the end of the tick (the damage hooks only listen).
struct Hit {
    victim: u32,
    damage_type: String,
    causing: i32,
    direct: i32,
    damage: f32,
    fall_blocked: f32,
}

static LIVING: Mutex<Option<HashMap<u32, Living>>> = Mutex::new(None);
static HITS: Mutex<Vec<Hit>> = Mutex::new(Vec::new());
/// Deaths with a burst effect: (dimension, position, lethal incubation level or 0, ice bomb, charged).
static DEATHS: Mutex<Vec<(String, [f64; 3], i32, bool, bool, u32)>> = Mutex::new(Vec::new());
/// Blocks broken with a Mythic tool: (player entity, gem, block position).
static BROKEN: Mutex<Vec<(u32, &'static str, [i32; 3])>> = Mutex::new(Vec::new());
/// Topaz tool hits per wielder (MythicState.TOPAZ_TOOL_HIT_COUNTS).
static TOPAZ_COUNTS: Mutex<Option<HashMap<u32, i32>>> = Mutex::new(None);
/// Effects the hooks without a host need, refreshed each tick: entity -> amplifier.
/// Each entry carries the tick it was seen; entries two ticks old are dropped.
static TOPAZ_HOLDERS: Mutex<Option<HashMap<u32, (i32, i64)>>> = Mutex::new(None);
static JADE_HOLDERS: Mutex<Option<HashMap<u32, (i32, i64)>>> = Mutex::new(None);
/// Static Field holders: entity -> (dimension, position) for the lightning redirect.
static STATIC_HOLDERS: Mutex<Option<HashMap<u32, (String, [f64; 3], i64)>>> = Mutex::new(None);
/// Blood Thirst holders: entity -> (dimension, position, level).
static BLOOD_HOLDERS: Mutex<Option<HashMap<u32, (String, [f64; 3], i32, i64)>>> = Mutex::new(None);
/// Jade jump fall credit (LivingEntityMixin.mu_jadeJumpFallCredit).
static FALL_CREDIT: Mutex<Option<HashMap<u32, f32>>> = Mutex::new(None);
/// Lightning moved away from a Static Field holder: (dimension, position).
static REDIRECT: Mutex<Vec<(String, [f64; 3])>> = Mutex::new(Vec::new());
static STRIKING: AtomicBool = AtomicBool::new(false);
static TICK: AtomicU64 = AtomicU64::new(0);
static TICK_SEEN: Mutex<Option<HashSet<u32>>> = Mutex::new(None);

fn now() -> i64 {
    TICK.load(Ordering::Relaxed) as i64
}

fn map<K, V>(m: &mut Option<HashMap<K, V>>) -> &mut HashMap<K, V> {
    m.get_or_insert_with(HashMap::new)
}

// ---------------------------------------------------------------------------------------------
// Host helpers
// ---------------------------------------------------------------------------------------------
fn effect(host: &dyn ServerHost, living: LivingRef, name: &str) -> Option<ActiveEffect> {
    let key = host.effect_key(name)?;
    living.effect(host, key)
}

fn add_effect(host: &mut dyn ServerHost, living: LivingRef, name: &str, duration: i32, amplifier: i32, visible: bool) {
    // MobEffectInstance(effect, duration, amplifier): not ambient, particles and icon shown; the armour's infinite
    // ones (false, false, true) show only the icon.
    if let Some(key) = host.effect_key(name) {
        let e = ActiveEffect::new(key, duration, amplifier, false, visible, true);
        living.add_effect(host, &e);
    }
}

fn add_effect_raw(host: &mut dyn ServerHost, living: LivingRef, name: &str, duration: i32, amplifier: i32, ambient: bool, visible: bool) {
    if let Some(key) = host.effect_key(name) {
        let e = ActiveEffect::new(key, duration, amplifier, ambient, visible, visible);
        living.add_effect(host, &e);
    }
}

fn remove_effect(host: &mut dyn ServerHost, living: LivingRef, name: &str) {
    if let Some(key) = host.effect_key(name) {
        living.remove_effect(host, key);
    }
}

fn pos_of(host: &dyn ServerHost, e: EntityRef) -> Option<[f64; 3]> {
    e.position(host)
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn is_living(host: &dyn ServerHost, e: EntityRef) -> Option<LivingRef> {
    let living = host.living(e)?;
    let kind = e.kind(host)?;
    if kind.id(host) == "minecraft:armor_stand" {
        return None;
    }
    Some(living)
}

/// Living entities (not armour stands) whose box meets the cube of `r` around `c`.
fn living_near(host: &dyn ServerHost, level: LevelRef, c: [f64; 3], r: f64) -> Vec<(LivingRef, [f64; 3])> {
    let Some(list) = level.entities_in(host, [c[0] - r, c[1] - r, c[2] - r], [c[0] + r, c[1] + r, c[2] + r]) else {
        return Vec::new();
    };
    list.into_iter()
        .filter_map(|e| Some((is_living(host, e)?, pos_of(host, e)?)))
        .collect()
}

/// LivingEntity.knockback(strength, dx, dz): pushed away from (dx, dz)'s direction, less knockback resistance.
fn knockback(host: &mut dyn ServerHost, living: LivingRef, strength: f64, dx: f64, dz: f64) {
    let e = living.entity();
    let res = living.attribute_value(&*host, "minecraft:knockback_resistance").unwrap_or(0.0);
    let s = strength * (1.0 - res);
    if s <= 0.0 {
        return;
    }
    let len = (dx * dx + dz * dz).sqrt();
    if len < 1.0e-5 {
        return;
    }
    let v = e.velocity(&*host).unwrap_or([0.0; 3]);
    let ground = e.on_ground(&*host).unwrap_or(true);
    let nx = v[0] / 2.0 - dx / len * s;
    let nz = v[2] / 2.0 - dz / len * s;
    let ny = if ground { (v[1] / 2.0 + s).min(0.4) } else { v[1] };
    e.set_velocity(host, [nx, ny, nz]);
}

fn sound(host: &mut dyn ServerHost, level: LevelRef, at: [f64; 3], name: &str, volume: f32, pitch: f32) {
    level.play_sound(host, name, at, SoundSource::Players, volume, pitch);
}

/// A ring of dust at `c` (the jar's wave rings, drawn once at their full radius).
fn ring(host: &mut dyn ServerHost, dim: &str, c: [f64; 3], radius: f32, colour: u32) {
    let (r, g, b) = (((colour >> 16) & 255) as f32 / 255.0, ((colour >> 8) & 255) as f32 / 255.0, (colour & 255) as f32 / 255.0);
    let n = ((radius * 6.0).ceil() as i32).clamp(6, 48);
    for i in 0..n {
        let a = i as f64 / n as f64 * std::f64::consts::TAU;
        let (x, z) = (c[0] + radius as f64 * a.cos(), c[2] + radius as f64 * a.sin());
        host.commands().run_command(&format!(
            "execute in {dim} run particle minecraft:dust{{color:[{r:.3},{g:.3},{b:.3}],scale:1.25}} {x:.2} {:.2} {z:.2} 0 0 0 0 1 force",
            c[1]
        ));
    }
}

fn puff(host: &mut dyn ServerHost, dim: &str, c: [f64; 3], colour: u32, count: i32) {
    let (r, g, b) = (((colour >> 16) & 255) as f32 / 255.0, ((colour >> 8) & 255) as f32 / 255.0, (colour & 255) as f32 / 255.0);
    host.commands().run_command(&format!(
        "execute in {dim} run particle minecraft:dust{{color:[{r:.3},{g:.3},{b:.3}],scale:1.0}} {:.2} {:.2} {:.2} 0.3 0.4 0.3 0 {count} force",
        c[0], c[1], c[2]
    ));
}

/// LivingEntityMixin.randomTeleportNear: 16 tries within 8 blocks.
fn random_teleport(host: &mut dyn ServerHost, living: LivingRef) {
    let Some(p) = pos_of(&*host, living.entity()) else { return };
    let (lo, hi) = living
        .entity()
        .level(&*host)
        .and_then(|l| l.height(&*host))
        .map_or((-64.0, 319.0), |(min, h)| (min as f64, (min + h - 1) as f64));
    for _ in 0..16 {
        let x = p[0] + (rand_f64() - 0.5) * 16.0;
        let y = (p[1] + (rand_int(16) - 8) as f64).clamp(lo, hi);
        let z = p[2] + (rand_f64() - 0.5) * 16.0;
        if living.random_teleport(host, [x, y, z], None) {
            break;
        }
    }
}

/// applyShockInRange: Topaz shock of `level` around `c`, hurting as `source`.
fn shock(host: &mut dyn ServerHost, lvl: LevelRef, dim: &str, c: [f64; 3], center: Option<u32>, radius: f32, level: i32, source: EntityRef) {
    let damage = level as f32 * TOPAZ_SHOCK_DAMAGE_PER_LEVEL;
    let kb = (level as f32 * TOPAZ_SHOCK_KNOCKBACK_PER_LEVEL) as f64;
    for (l, p) in living_near(&*host, lvl, c, radius as f64) {
        if Some(l.id()) == center || dist(p, c) > radius as f64 {
            continue;
        }
        hurt_own(host, l.entity(), "topaz_shock", damage, Some(source), Some(source));
        knockback(host, l, kb, c[0] - p[0], c[2] - p[2]);
    }
    sound(host, lvl, c, "minecraft:block.amethyst_block.break", 1.0, 1.1);
    sound(host, lvl, c, "minecraft:block.anvil.place", 1.0, 0.7);
    ring(host, dim, [c[0], c[1] + 1.0, c[2]], radius, 14960924);
}

// ---------------------------------------------------------------------------------------------
// Each living entity, each tick (LivingEntityMixin.checkArmorEffects)
// ---------------------------------------------------------------------------------------------
fn tick_living(host: &mut dyn ServerHost, dim: &str, living: LivingRef) {
    let id = living.id();
    let t = now();
    {
        let mut seen = lock(&TICK_SEEN);
        if !seen.get_or_insert_with(HashSet::new).insert(id) {
            return;
        }
    }
    let e = living.entity();
    let Some(p) = pos_of(&*host, e) else { return };
    let Some(lvl) = host.level(dim) else { return };
    // Armour set levels.
    let mut levels = [0i32; 8];
    for (slot, piece, add) in [
        (mod_api::EquipmentSlot::Head, PIECES[0], PIECE_LEVELS[0]),
        (mod_api::EquipmentSlot::Chest, PIECES[1], PIECE_LEVELS[1]),
        (mod_api::EquipmentSlot::Legs, PIECES[2], PIECE_LEVELS[2]),
        (mod_api::EquipmentSlot::Feet, PIECES[3], PIECE_LEVELS[3]),
    ] {
        let Some(stack) = living.equipment(&*host, slot) else { continue };
        let item = stack.item().id(&*host).to_string();
        for (i, (gem, _)) in SETS.iter().enumerate() {
            if item.strip_prefix(NS).is_some_and(|r| r == format!("{gem}_{piece}")) {
                levels[i] += add;
            }
        }
    }
    let effects: HashMap<String, ActiveEffect> = living
        .effects(&*host)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|a| Some((a.id.id(&*host).to_string(), a)))
        .collect();
    let has = |n: &str| effects.get(&format!("{NS}{n}"));
    let mut st = {
        let mut g = lock(&LIVING);
        map(&mut g).remove(&id).unwrap_or_default()
    };
    st.seen = t;
    for (i, (_, eff)) in SETS.iter().enumerate() {
        let l = levels[i];
        if l != st.levels[i] || l > 0 && has(eff).is_none() {
            st.levels[i] = l;
            if eff == &"static_field" {
                st.static_stacks.clear();
            }
            remove_effect(host, living, &format!("{NS}{eff}"));
            if l > 0 {
                add_effect(host, living, &format!("{NS}{eff}"), -1, l - 1, false);
            }
        }
    }
    // Re-read what the set step may have changed.
    let effects: HashMap<String, ActiveEffect> = living
        .effects(&*host)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|a| Some((a.id.id(&*host).to_string(), a)))
        .collect();
    let amp = |n: &str| effects.get(&format!("{NS}{n}")).map(|a| a.amplifier);
    // Caches for the hooks that have no host.
    match amp("topaz_reaction") {
        Some(a) => map(&mut lock(&TOPAZ_HOLDERS)).insert(id, (a, t)),
        None => map(&mut lock(&TOPAZ_HOLDERS)).remove(&id),
    };
    match amp("jade_aura") {
        Some(a) => map(&mut lock(&JADE_HOLDERS)).insert(id, (a, t)),
        None => map(&mut lock(&JADE_HOLDERS)).remove(&id),
    };
    match amp("static_field") {
        Some(_) => map(&mut lock(&STATIC_HOLDERS)).insert(id, (dim.to_string(), p, t)),
        None => map(&mut lock(&STATIC_HOLDERS)).remove(&id),
    };
    match amp("blood_thirst") {
        Some(a) => map(&mut lock(&BLOOD_HOLDERS)).insert(id, (dim.to_string(), p, a + 1, t)),
        None => map(&mut lock(&BLOOD_HOLDERS)).remove(&id),
    };
    // Miasma: poison around every 200 ticks; MiasmaEffect.applyEffectTick cures poison.
    if let Some(a) = amp("miasma") {
        if effects.contains_key("minecraft:poison") {
            remove_effect(host, living, "minecraft:poison");
        }
        st.miasma_tick += 1;
        if st.miasma_tick >= MIASMA_INTERVAL_TICKS {
            st.miasma_tick = 0;
            miasma_poison(host, lvl, dim, living, p, a + 1);
        }
    } else {
        st.miasma_tick = 0;
    }
    // Static Field: every second, 0.2 per level, 10% more per hit in a row.
    if let Some(a) = amp("static_field") {
        let level = a + 1;
        let radius = level as f32 * STATIC_FIELD_RADIUS_PER_LEVEL;
        let tick_count = e.tick_count(&*host).unwrap_or(t as i32);
        if tick_count % 20 == 0 {
            let damage = level as f32 * STATIC_FIELD_DAMAGE_PER_LEVEL_PER_SECOND;
            st.static_stacks.retain(|_, v| t - v.1 <= STATIC_FIELD_STACK_RESET_TICKS);
            for (l, q) in living_near(&*host, lvl, p, radius as f64) {
                if l.id() == id || dist(q, p) > radius as f64 {
                    continue;
                }
                let entry = st.static_stacks.entry(l.id()).or_insert((0, t));
                let mult = (1.0 + STATIC_FIELD_STACK_DAMAGE_INCREASE).powi(entry.0);
                hurt_own(host, l.entity(), "static_field", damage * mult, Some(e), Some(e));
                entry.0 += 1;
                entry.1 = t;
            }
        }
        if tick_count % 3 == 0 && !e.is_crouching(&*host).unwrap_or(false) {
            puff(host, dim, [p[0], p[1] + 1.0, p[2]], 15241499, 2 * level);
        }
    } else {
        st.static_stacks.clear();
    }
    // Jade Aura: the speed modifier while it lasts, the trail every 10 ticks, the lingering points' contact.
    let jade = amp("jade_aura");
    if jade != st.jade_speed {
        let mid = mod_api::Identifier::parse("mythicupgrades:jade_aura_speed").ok();
        if let Some(mid) = mid {
            if st.jade_speed.is_some() {
                living.remove_attribute_modifier(host, "minecraft:movement_speed", &mid);
            }
            if let Some(a) = jade {
                let m = AttributeModifier::new(mid, (a + 1) as f64 * JADE_AURA_SPEED_PER_LEVEL, AttributeOperation::AddMultipliedBase);
                living.add_attribute_modifier(host, "minecraft:movement_speed", m, ModifierLifetime::Transient);
            }
        }
        st.jade_speed = jade;
    }
    if !st.jade_trail.is_empty() {
        st.jade_trail.retain(|tr| t - tr.0 < JADE_TRAIL_DURATION_TICKS);
        let trail = st.jade_trail.clone();
        for (_, q, alvl) in trail {
            if t % 4 == 0 {
                puff(host, dim, [q[0], q[1] + 0.2, q[2]], 2145087, 2);
            }
            let cr = JADE_TRAIL_LINGER_CONTACT_RADIUS;
            for (l, lp) in living_near(&*host, lvl, q, cr) {
                if l.id() == id || ((lp[0] - q[0]).powi(2) + (lp[2] - q[2]).powi(2)).sqrt() > cr {
                    continue;
                }
                let dur = JADE_TRAIL_CONTACT_EFFECT_DURATION_TICKS;
                add_effect_raw(host, l, "minecraft:speed", dur, (alvl - 1).min(9), false, true);
                add_effect_raw(host, l, "minecraft:jump_boost", dur, (alvl / 2).min(9), false, true);
            }
        }
    }
    if let Some(a) = jade {
        if t % JADE_TRAIL_INTERVAL_TICKS == 0 {
            let level = a + 1;
            let trail_amp = (level + 1) / 2 - 1;
            for (l, q) in living_near(&*host, lvl, p, JADE_TRAIL_RADIUS as f64) {
                if l.id() == id || dist(q, p) > JADE_TRAIL_RADIUS as f64 {
                    continue;
                }
                let cur = effect(&*host, l, &format!("{NS}jade_aura"));
                if cur.is_none_or(|c| c.amplifier < trail_amp) {
                    add_effect(host, l, &format!("{NS}jade_aura"), JADE_TRAIL_EFFECT_DURATION_TICKS, trail_amp, true);
                }
            }
            st.jade_trail.push((t, p, level));
            puff(host, dim, [p[0], p[1] + 0.4, p[2]], 2145087, 6);
        }
    }
    // FreezeEffect.applyEffectTick.
    if amp("freeze").is_some() {
        e.set_ticks_frozen(host, TICKS_REQUIRED_TO_FREEZE + 2);
    }
    // NecoiumShareEffect: every 10 ticks of its duration.
    if let Some(n) = effects.get(&format!("{NS}necoium_share")) {
        if n.duration == -1 && t % 10 == 0 || n.duration % 10 == 0 {
            necoium_share(host, lvl, dim, living, p);
        }
    }
    map(&mut lock(&LIVING)).insert(id, st);
}

/// LivingEntityMixin.applyMiasmaPoison.
fn miasma_poison(host: &mut dyn ServerHost, lvl: LevelRef, dim: &str, owner: LivingRef, c: [f64; 3], level: i32) {
    let radius = level as f32 * MIASMA_CLOUD_RADIUS_PER_LEVEL;
    let owner_uuid = owner.entity().uuid(host);
    let owner_tags = owner.entity().tags(&*host).unwrap_or_default();
    let _ = owner_tags;
    for (l, q) in living_near(&*host, lvl, c, radius as f64) {
        if l.id() == owner.id() || dist(q, c) > radius as f64 {
            continue;
        }
        if owner_uuid.is_some() && l.owner(&*host) == owner_uuid {
            continue;
        }
        add_effect(host, l, "minecraft:poison", MIASMA_POISON_DURATION_TICKS, level - 1, true);
    }
    sound(host, lvl, c, "minecraft:block.amethyst_block.break", 0.8, 0.6);
    ring(host, dim, [c[0], c[1] + 1.0, c[2]], radius, 9162775);
}

/// NecoiumShareHandler.onTick: the network of holders within 30 blocks of each other shares the strongest of each
/// gem effect (100 ticks, or infinite if a source is), shown as ambient copies; alone, a holder loses the copies.
fn necoium_share(host: &mut dyn ServerHost, lvl: LevelRef, dim: &str, source: LivingRef, p: [f64; 3]) {
    let mut net: Vec<(LivingRef, [f64; 3])> = vec![(source, p)];
    let mut seen: HashSet<u32> = [source.id()].into();
    let mut i = 0;
    while i < net.len() {
        let (_, c) = net[i];
        for (l, q) in living_near(&*host, lvl, c, NECOIUM_RANGE) {
            if seen.contains(&l.id()) || effect(&*host, l, &format!("{NS}necoium_share")).is_none() {
                continue;
            }
            seen.insert(l.id());
            net.push((l, q));
        }
        i += 1;
    }
    if net.len() < 2 {
        for eff in SHAREABLE {
            if let Some(a) = effect(&*host, source, &format!("{NS}{eff}")) {
                if a.ambient && a.visible && a.duration == -1 {
                    remove_effect(host, source, &format!("{NS}{eff}"));
                }
            }
        }
        return;
    }
    for eff in SHAREABLE {
        let name = format!("{NS}{eff}");
        let mut max_amp = -1;
        let mut infinite = false;
        for (l, _) in &net {
            if let Some(a) = effect(&*host, *l, &name) {
                if !a.ambient || !a.visible {
                    max_amp = max_amp.max(a.amplifier);
                    infinite |= a.duration == -1;
                }
            }
        }
        if max_amp >= 0 {
            let dur = if infinite { -1 } else { NECOIUM_SHARE_DURATION };
            for (l, _) in &net {
                let cur = effect(&*host, *l, &name);
                let update = match &cur {
                    None => true,
                    Some(c) if c.ambient || c.amplifier < max_amp => {
                        c.amplifier < max_amp || dur == -1 && c.duration != -1 || dur != -1 && c.duration != -1 && c.duration < 100
                    }
                    Some(_) => false,
                };
                if update {
                    add_effect_raw(host, *l, &name, dur, max_amp, true, true);
                }
            }
        } else {
            for (l, _) in &net {
                if let Some(c) = effect(&*host, *l, &name) {
                    if c.ambient && c.visible {
                        remove_effect(host, *l, &name);
                    }
                }
            }
        }
    }
    let t = now();
    if t % 4 == 0 {
        for w in net.windows(2) {
            let (a, b) = (w[0].1, w[1].1);
            let f = (t % 40) as f64 / 40.0;
            let m = [a[0] + (b[0] - a[0]) * f, a[1] + 1.0 + (b[1] - a[1]) * f, a[2] + (b[2] - a[2]) * f];
            puff(host, dim, m, 15080079, 3);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Hits (LivingEntityMixin.onHurt), handled at the end of the tick
// ---------------------------------------------------------------------------------------------
fn handle_hit(host: &mut dyn ServerHost, h: Hit) {
    let Some(ve) = host.entity(h.victim) else { return };
    let Some(victim) = host.living(ve) else { return };
    let Some(lvl) = ve.level(&*host) else { return };
    let dim = lvl.dimension_id().to_string();
    let Some(vp) = pos_of(&*host, ve) else { return };
    let actual = h.damage;
    let alive = victim.alive(&*host).unwrap_or(false) && victim.health(&*host).unwrap_or(0.0) > 0.0;
    let causing = if h.causing > 0 { host.entity(h.causing as u32) } else { None };
    let attacker = causing.and_then(|c| host.living(c));
    // Damage Deflection.
    if actual > 0.0 {
        if let (Some(d), Some(att)) = (effect(&*host, victim, &format!("{NS}damage_deflection")), attacker) {
            let lv = (d.amplifier + 1) as f32;
            hurt_own(host, att.entity(), "deflecting", lv * DEFLECT_PER_LEVEL * actual, Some(ve), Some(ve));
            let bonus = (lv * DEFLECT_BONUS_DAMAGE_PER_LEVEL).min(DEFLECT_BONUS_DAMAGE_MAX);
            hurt_own(host, att.entity(), "deflecting", bonus, Some(ve), Some(ve));
        }
    }
    // Topaz Reaction on a fall: the blocked part was taken off in living_damage_pre; the shock.
    if h.fall_blocked > 0.0 && alive {
        if let Some(t) = effect(&*host, victim, &format!("{NS}topaz_reaction")) {
            let l = t.amplifier + 1;
            shock(host, lvl, &dim, vp, Some(h.victim), l as f32 * TOPAZ_ARMOR_SHOCK_RADIUS_PER_LEVEL, l, ve);
        }
    }
    // Blood Thirst holders near the victim.
    if actual > 0.0 {
        let holders: Vec<(u32, String, [f64; 3], i32)> =
            map(&mut lock(&BLOOD_HOLDERS)).iter().map(|(k, v)| (*k, v.0.clone(), v.1, v.2)).collect();
        for (id, d, p, l) in holders {
            if id == h.victim || d != dim || dist(p, vp) > (l as f32 * BLOOD_THIRST_RADIUS_PER_LEVEL) as f64 {
                continue;
            }
            if let Some(hl) = host.entity(id).and_then(|e| host.living(e)) {
                hl.heal(host, actual * l as f32 * BLOOD_THIRST_HEAL_FRACTION_PER_LEVEL);
                puff(host, &dim, [p[0], p[1] + 1.0, p[2]], 14424897, 3 * l);
            }
        }
    }
    // Arcane Aura: levitation around.
    if let Some(a) = effect(&*host, victim, &format!("{NS}arcane_aura")) {
        let l = a.amplifier + 1;
        let r = l as f32 * ARCANE_AURA_RADIUS_PER_LEVEL;
        for (o, q) in living_near(&*host, lvl, vp, r as f64) {
            if o.id() != h.victim && dist(q, vp) <= r as f64 {
                add_effect(host, o, "minecraft:levitation", ARCANE_AURA_LEVITATION_DURATION_TICKS, l * ARCANE_AURA_LEVITATION_AMP_PER_LEVEL - 1, true);
            }
        }
        sound(host, lvl, vp, "minecraft:block.amethyst_block.break", 1.0, 0.8);
        sound(host, lvl, vp, "minecraft:block.enchantment_table.use", 1.0, 1.2);
        ring(host, &dim, [vp[0], vp[1] + 1.0, vp[2]], r, 10760642);
    }
    // Topaz Reaction: a shock around.
    if let Some(t) = effect(&*host, victim, &format!("{NS}topaz_reaction")) {
        let l = t.amplifier + 1;
        shock(host, lvl, &dim, vp, Some(h.victim), l as f32 * TOPAZ_ARMOR_SHOCK_RADIUS_PER_LEVEL, l, ve);
    }
    // Melee: the attacker hit directly.
    if h.direct > 0 && h.direct == h.causing {
        if let Some(att) = attacker {
            let ae = att.entity();
            let weapon = att.hand_item(&*host, mod_api::Hand::Main);
            let gem = weapon.as_ref().and_then(|s| tool_gem(&s.item().id(&*host).to_string()));
            match gem {
                Some("sapphire") => {
                    let max = victim.max_health(&*host).unwrap_or(20.0);
                    let bonus = (max * SAPPHIRE_TOOL_PERCENT_DAMAGE).min(SAPPHIRE_TOOL_PERCENT_DAMAGE_CAP);
                    hurt_own(host, ve, "percentage", bonus, Some(ae), Some(ae));
                }
                Some("ametrine") => {
                    add_effect(host, victim, "minecraft:levitation", AMETRINE_TOOL_LEVITATION_DURATION_TICKS, AMETRINE_TOOL_LEVITATION_AMPLIFIER, true);
                    if (rand_f64() as f32) < AMETRINE_TOOL_TELEPORT_CHANCE {
                        random_teleport(host, victim);
                    }
                }
                Some("topaz") => {
                    let mut counts = lock(&TOPAZ_COUNTS);
                    let c = map(&mut counts).entry(ae.id()).or_insert(0);
                    *c += 1;
                    let fire = *c >= TOPAZ_TOOL_SHOCK_INTERVAL;
                    if fire {
                        *c = 0;
                    }
                    drop(counts);
                    if fire {
                        let l = TOPAZ_TOOL_EFFECTIVE_LEVEL;
                        shock(host, lvl, &dim, vp, None, l as f32 * TOPAZ_ARMOR_SHOCK_RADIUS_PER_LEVEL, l, ae);
                    }
                }
                Some("ruby") if actual > 0.0 => {
                    att.heal(host, actual * RUBY_TOOL_LIFESTEAL_FRACTION + 1.0);
                    puff(host, &dim, [vp[0], vp[1] + 1.0, vp[2]], 14424897, 10);
                }
                Some("peridot") => {
                    add_effect(host, victim, &format!("{NS}lethal_incubation"), LETHAL_INCUBATION_TOOL_DURATION_TICKS, 2, true);
                }
                Some("aquamarine") => {
                    add_effect(host, victim, &format!("{NS}freeze"), AQUAMARINE_TOOL_FREEZE_TICKS, 0, true);
                    add_effect(host, victim, "minecraft:slowness", AQUAMARINE_TOOL_SLOWNESS_DURATION_TICKS, AQUAMARINE_TOOL_SLOWNESS_AMPLIFIER, true);
                    add_effect(host, victim, &format!("{NS}ice_bomb"), ICE_BOMB_TOOL_DURATION_TICKS, 0, true);
                    puff(host, &dim, [vp[0], vp[1] + 1.0, vp[2]], 1024711, 12);
                }
                Some("citrine") => {
                    add_effect(host, victim, &format!("{NS}charged"), CITRINE_TOOL_CHARGED_DURATION_TICKS, 0, true);
                    add_effect(host, victim, &format!("{NS}freeze"), CITRINE_TOOL_FREEZE_TICKS, 0, true);
                    if actual > 0.0 {
                        chain_lightning(host, lvl, ve, vp, att, actual);
                    }
                }
                Some("jade") => {
                    add_effect(host, att, &format!("{NS}jade_aura"), JADE_TOOL_AURA_DURATION_TICKS, 4, true);
                    if (rand_f64() as f32) < JADE_TOOL_TELEPORT_CHANCE {
                        random_teleport(host, victim);
                    }
                }
                _ => {}
            }
            // The attacker's own Blood Thirst.
            if actual > 0.0 {
                if let Some(b) = effect(&*host, att, &format!("{NS}blood_thirst")) {
                    let l = (b.amplifier + 1) as f32;
                    att.heal(host, actual * l * BLOOD_THIRST_HEAL_FRACTION_PER_LEVEL);
                }
            }
            // Ice Shield freezes the attacker.
            if effect(&*host, victim, &format!("{NS}ice_shield")).is_some() {
                add_effect(host, att, &format!("{NS}freeze"), ICE_SHIELD_FREEZE_TICKS, 0, true);
            }
        }
    }
}

/// applyChainLightning: a third of the damage to the three nearest others within 10 blocks of the attacker.
fn chain_lightning(host: &mut dyn ServerHost, lvl: LevelRef, victim: EntityRef, vp: [f64; 3], att: LivingRef, damage: f32) {
    let ae = att.entity();
    let c = pos_of(&*host, ae).unwrap_or(vp);
    let mut near = living_near(&*host, lvl, c, CITRINE_TOOL_CHAIN_RANGE as f64);
    near.sort_by(|a, b| dist(a.1, vp).total_cmp(&dist(b.1, vp)));
    let mut n = 0;
    for (l, _) in near {
        if n >= CITRINE_TOOL_CHAIN_TARGETS {
            break;
        }
        if l.id() == victim.id() || l.id() == ae.id() {
            continue;
        }
        hurt_own(host, l.entity(), "citrine_chain", damage * CITRINE_TOOL_CHAIN_FRACTION, Some(ae), Some(ae));
        n += 1;
    }
}

/// triggerDeathEffects.
fn handle_death(host: &mut dyn ServerHost, dim: &str, c: [f64; 3], incubation: i32, ice_bomb: bool, charged: bool, who: u32) {
    let Some(lvl) = host.level(dim) else { return };
    let src = host.entity(who);
    if incubation > 0 {
        let r = LETHAL_INCUBATION_SHOCK_RADIUS;
        for (l, q) in living_near(&*host, lvl, c, r as f64) {
            if l.id() == who || dist(q, c) > r as f64 {
                continue;
            }
            hurt_own(host, l.entity(), "peridot_incubation", LETHAL_INCUBATION_SHOCK_DAMAGE, src, src);
            add_effect(host, l, "minecraft:poison", LETHAL_INCUBATION_POISON_DURATION_TICKS, (incubation - 1).max(0), true);
            knockback(host, l, 1.0, c[0] - q[0], c[2] - q[2]);
        }
        let _ = lvl.explode(host, None, c, 2.0, false, mod_api::ExplosionInteraction::None, Default::default());
        ring(host, dim, [c[0], c[1] + 1.0, c[2]], r, 5148691);
    }
    if ice_bomb {
        let r = ICE_BOMB_BURST_RADIUS;
        for (l, q) in living_near(&*host, lvl, c, r as f64) {
            if l.id() == who || dist(q, c) > r as f64 {
                continue;
            }
            hurt_own(host, l.entity(), "ice_bomb_burst", ICE_BOMB_BURST_DAMAGE, src, src);
            add_effect(host, l, &format!("{NS}freeze"), ICE_BOMB_BURST_FREEZE_TICKS, 0, true);
            knockback(host, l, 1.0, c[0] - q[0], c[2] - q[2]);
        }
        let _ = lvl.explode(host, None, c, 2.0, false, mod_api::ExplosionInteraction::None, Default::default());
        ring(host, dim, [c[0], c[1] + 1.0, c[2]], r, 3927031);
    }
    if charged {
        let n = CITRINE_CHARGED_LIGHTNING_MIN + rand_int(CITRINE_CHARGED_LIGHTNING_MAX - CITRINE_CHARGED_LIGHTNING_MIN + 1);
        for _ in 0..n {
            let ox = (rand_f64() - 0.5) * 2.0 * CITRINE_CHARGED_LIGHTNING_SPREAD;
            let oz = (rand_f64() - 0.5) * 2.0 * CITRINE_CHARGED_LIGHTNING_SPREAD;
            lvl.strike_lightning(host, [c[0] + ox, c[1], c[2] + oz], false);
        }
    }
}

/// ServerPlayerGameModeMixin.onDestroyBlock.
fn handle_broken(host: &mut dyn ServerHost, player: u32, gem: &str, pos: [i32; 3]) {
    let Some(pe) = host.entity(player) else { return };
    let Some(pl) = host.living(pe) else { return };
    match gem {
        "jade" => add_effect(host, pl, &format!("{NS}jade_aura"), JADE_TOOL_AURA_DURATION_TICKS, 4, true),
        "ruby" => {
            pl.heal(host, RUBY_TOOL_INTERACTION_HEAL);
        }
        "topaz" => {
            let fire = {
                let mut counts = lock(&TOPAZ_COUNTS);
                let c = map(&mut counts).entry(player).or_insert(0);
                *c += 1;
                let f = *c >= TOPAZ_TOOL_SHOCK_INTERVAL;
                if f {
                    *c = 0;
                }
                f
            };
            if !fire {
                return;
            }
            let Some(lvl) = pe.level(&*host) else { return };
            let dim = lvl.dimension_id().to_string();
            let c = [pos[0] as f64 + 0.5, pos[1] as f64 + 0.5, pos[2] as f64 + 0.5];
            let l = TOPAZ_TOOL_EFFECTIVE_LEVEL;
            let radius = l as f32 * TOPAZ_ARMOR_SHOCK_RADIUS_PER_LEVEL;
            let pp = pos_of(&*host, pe).unwrap_or(c);
            let damage = l as f32 * TOPAZ_SHOCK_DAMAGE_PER_LEVEL;
            let kb = (l as f32 * TOPAZ_SHOCK_KNOCKBACK_PER_LEVEL) as f64;
            for (o, q) in living_near(&*host, lvl, c, radius as f64) {
                // The jar measures from the player, not the block.
                if o.id() == player || dist(q, pp) > radius as f64 {
                    continue;
                }
                hurt_own(host, o.entity(), "topaz_shock", damage, Some(pe), Some(pe));
                knockback(host, o, kb, c[0] - q[0], c[2] - q[2]);
            }
            sound(host, lvl, c, "minecraft:block.amethyst_block.break", 1.0, 1.1);
            sound(host, lvl, c, "minecraft:block.anvil.place", 1.0, 0.7);
            ring(host, &dim, c, radius, 14960924);
        }
        _ => {}
    }
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

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        for (name, ..) in classes::BLOCKS {
            let key = r.block_key(&format!("{NS}{name}"));
            r.block_part(key, Rules);
        }
    }

    /// MythicUpgrades' LootTableEvents.MODIFY: the template in the end city treasure at 25%.
    fn loot_table_load(&self, _context: &dyn DataLoadContext, event: &mut LootTableLoadEvent<'_>) {
        if event.id == "minecraft:chests/end_city_treasure" {
            push_pool(event.table, json!({"rolls": 1.0, "bonus_rolls": 0.0, "entries": [
                {"type": "minecraft:loot_table", "value": format!("{NS}injects/end_city_treasure")}]}));
            if std::env::var_os("MU_DEBUG").is_some() {
                eprintln!("[mythicupgrades] {} gets the template pool", event.id);
            }
        }
    }

    fn entity_tick_post(&self, host: &mut dyn ServerHost, event: &mut mod_api::EntityTickPostEvent<'_>) {
        let Some(e) = host.entity(event.entity) else { return };
        let Some(living) = is_living(&*host, e) else { return };
        let dim = event.dimension.to_string();
        tick_living(host, &dim, living);
    }

    /// Topaz Reaction takes 5% per level off fall damage (captureHealth's mu_topazFallBlocked, healed back in Java).
    fn living_damage_pre(&self, _level: &mut dyn mod_api::LevelRandomHost, event: &mut mod_api::LivingDamagePreEvent<'_>) {
        if event.source.damage_type != "minecraft:fall" {
            return;
        }
        let Some((a, _)) = map(&mut lock(&TOPAZ_HOLDERS)).get(&(event.entity as u32)).copied() else { return };
        let blocked = event.container.original() * (a + 1) as f32 * TOPAZ_FALL_REDUCTION_PER_LEVEL;
        let now_d = event.container.new_damage();
        event.container.set_new_damage((now_d - blocked).max(0.0));
        lock(&HITS).push(Hit {
            victim: event.entity as u32,
            damage_type: "fall_blocked".into(),
            causing: 0,
            direct: 0,
            damage: 0.0,
            fall_blocked: blocked,
        });
    }

    fn living_damage_post(&self, _level: &mut dyn mod_api::LevelRandomHost, event: &mut mod_api::LivingDamagePostEvent<'_>) {
        let t = event.source.damage_type;
        if OWN_HURT.load(Ordering::Relaxed) || OWN_DAMAGE.iter().any(|o| t.strip_prefix(NS) == Some(*o)) {
            return;
        }
        lock(&HITS).push(Hit {
            victim: event.entity as u32,
            damage_type: t.to_string(),
            causing: event.source.causing,
            direct: event.source.direct,
            damage: event.container.health_damage(),
            fall_blocked: 0.0,
        });
    }

    /// The death bursts (triggerDeathEffects): read while the dying entity still has its effects.
    fn living_death(&self, host: &mut dyn ServerHost, event: &mut mod_api::LivingDeathEvent<'_>) -> mod_api::Flow {
        let Some(e) = host.entity(event.entity) else { return mod_api::Flow::Continue };
        let Some(l) = host.living(e) else { return mod_api::Flow::Continue };
        let inc = effect(&*host, l, &format!("{NS}lethal_incubation")).map_or(0, |a| a.amplifier + 1);
        let bomb = effect(&*host, l, &format!("{NS}ice_bomb")).is_some();
        let charged = effect(&*host, l, &format!("{NS}charged")).is_some();
        if inc > 0 || bomb || charged {
            if let (Some(p), Some(lvl)) = (pos_of(&*host, e), e.level(&*host)) {
                let dim = lvl.dimension_id().to_string();
                lock(&DEATHS).push((dim, p, inc, bomb, charged, event.entity));
            }
        }
        mod_api::Flow::Continue
    }

    /// Jade Aura's jump boost (applyJadeJumpBoost), and its fall credit.
    fn living_jump_power(&self, _context: &dyn mod_api::LoopContext, event: &mut mod_api::LivingJumpPowerEvent<'_>) {
        let Some((a, _)) = map(&mut lock(&JADE_HOLDERS)).get(&event.entity).copied() else { return };
        let extra = (a + 1) as f32 * JADE_AURA_JUMP_PER_LEVEL;
        event.power += extra;
        map(&mut lock(&FALL_CREDIT)).insert(event.entity, extra * 7.0);
    }

    /// reduceJadeJumpFallDamage.
    fn living_fall(&self, _context: &dyn mod_api::LoopContext, event: &mut mod_api::LivingFallEvent<'_>) -> mod_api::Flow {
        if let Some(c) = map(&mut lock(&FALL_CREDIT)).remove(&event.entity) {
            event.distance = (event.distance - c as f64).max(0.0);
        }
        mod_api::Flow::Continue
    }

    /// LightningBoltMixin: a bolt within 3 blocks of a Static Field holder lands 7 blocks away.
    fn lightning_strike(&self, _host: &mut dyn ServerHost, event: &mut mod_api::LightningStrikeEvent<'_>) -> mod_api::Flow {
        if STRIKING.load(Ordering::Relaxed) {
            return mod_api::Flow::Continue;
        }
        let p = [event.pos[0] as f64 + 0.5, event.pos[1] as f64, event.pos[2] as f64 + 0.5];
        let holders: Vec<(String, [f64; 3])> = map(&mut lock(&STATIC_HOLDERS)).values().map(|v| (v.0.clone(), v.1)).collect();
        if let Some((dim, _)) = holders.iter().find(|(_, q)| (q[0] - p[0]).abs() <= 3.5 && (q[1] - p[1]).abs() <= 3.5 && (q[2] - p[2]).abs() <= 3.5) {
            let a = rand_f64() * std::f64::consts::TAU;
            let d = CITRINE_LIGHTNING_REDIRECT_DISTANCE;
            lock(&REDIRECT).push((dim.clone(), [p[0] + d * a.cos(), p[1], p[2] + d * a.sin()]));
            event.cancelled = true;
            return mod_api::Flow::Cancel;
        }
        mod_api::Flow::Continue
    }

    fn player_destroyed_block(&self, host: &mut dyn ServerHost, event: &mut mod_api::PlayerDestroyedBlockEvent) {
        let Some(player) = host.players().acting_player() else { return };
        let Some(living) = host.living(player.entity()) else { return };
        let Some(stack) = living.hand_item(&*host, mod_api::Hand::Main) else { return };
        if let Some(gem) = tool_gem(&stack.item().id(&*host).to_string()) {
            if matches!(gem, "jade" | "ruby" | "topaz") {
                lock(&BROKEN).push((player.entity().id(), gem, event.pos));
            }
        }
    }

    fn server_tick(&self, host: &mut dyn ServerHost, event: &mut ServerTickEvent<'_>) {
        match event.phase {
            ServerTickPhase::ServerTickStart { .. } => {
                TICK.fetch_add(1, Ordering::Relaxed);
                if let Some(s) = lock(&TICK_SEEN).as_mut() {
                    s.clear();
                }
                // Holders not seen in the last two ticks are gone (dead, unloaded).
                let t = now();
                map(&mut lock(&STATIC_HOLDERS)).retain(|_, v| t - v.2 <= 2);
                map(&mut lock(&BLOOD_HOLDERS)).retain(|_, v| t - v.3 <= 2);
                map(&mut lock(&TOPAZ_HOLDERS)).retain(|_, v| t - v.1 <= 2);
                map(&mut lock(&JADE_HOLDERS)).retain(|_, v| t - v.1 <= 2);
            }
            ServerTickPhase::ServerTickEnd { .. } => {
                // Players, in case the entity ticks skip them.
                let dims: Vec<String> = ["minecraft:overworld", "minecraft:the_nether", "minecraft:the_end"].iter().map(|s| s.to_string()).collect();
                for d in dims {
                    let Some(lvl) = host.level(&d) else { continue };
                    for p in lvl.online_players(&*host).unwrap_or_default() {
                        if let Some(l) = host.living(p.entity()) {
                            tick_living(host, &d, l);
                        }
                    }
                }
                let hits = std::mem::take(&mut *lock(&HITS));
                // Fold the fall reductions into their hit.
                let mut blocked: HashMap<u32, f32> = HashMap::new();
                let mut rest = Vec::new();
                for h in hits {
                    if h.damage_type == "fall_blocked" {
                        *blocked.entry(h.victim).or_insert(0.0) += h.fall_blocked;
                    } else {
                        rest.push(h);
                    }
                }
                for mut h in rest {
                    if h.damage_type == "minecraft:fall" {
                        h.fall_blocked = blocked.remove(&h.victim).unwrap_or(0.0);
                    }
                    if debug() {
                        eprintln!("[mythicupgrades] hit {} by {} ({}) {}", h.victim, h.causing, h.damage_type, h.damage);
                    }
                    handle_hit(host, h);
                }
                let deaths = std::mem::take(&mut *lock(&DEATHS));
                for (dim, p, inc, bomb, charged, who) in deaths {
                    handle_death(host, &dim, p, inc, bomb, charged, who);
                }
                let broken = std::mem::take(&mut *lock(&BROKEN));
                for (player, gem, pos) in broken {
                    handle_broken(host, player, gem, pos);
                }
                let redirect = std::mem::take(&mut *lock(&REDIRECT));
                STRIKING.store(true, Ordering::Relaxed);
                for (dim, p) in redirect {
                    if let Some(lvl) = host.level(&dim) {
                        lvl.strike_lightning(host, p, false);
                    }
                }
                STRIKING.store(false, Ordering::Relaxed);
                // Forget entities not seen for a while.
                let t = now();
                if t % 200 == 0 {
                    map(&mut lock(&LIVING)).retain(|_, s| t - s.seen < 200);
                    map(&mut lock(&TOPAZ_COUNTS)).retain(|k, _| host.entity(*k).is_some());
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools() {
        assert_eq!(tool_gem("mythicupgrades:sapphire_sword"), Some("sapphire"));
        assert_eq!(tool_gem("mythicupgrades:jade_hoe"), Some("jade"));
        assert_eq!(tool_gem("mythicupgrades:jade_helmet"), None);
        assert_eq!(tool_gem("minecraft:diamond_sword"), None);
    }

    #[test]
    fn props() {
        assert_eq!(state_prop("a:b[facing=up,waterlogged=false]", "facing").as_deref(), Some("up"));
    }
}
