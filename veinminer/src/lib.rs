//! Veinminer for Horizon: breaking one ore of a vein with a pickaxe breaks the whole vein
//! (Veinminer 2.x by MiraculixxT, AGPL-3.0-only, with its default "Ores" group).
//!
//! The vein, as `NormalStrategy` finds it: a breadth-first fill from the broken block where every
//! block within `search_radius` (a cube) of the one before that belongs to the origin's group
//! counts, nearest layer first, until `max_chain` blocks are collected.
//!
//! `break_block` finds the vein and lets the broken block break as vanilla does; the next server
//! tick drops every other block's loot, mined with the held tool, where the block was (or at the
//! mined block, merged), clears the blocks, and wears the tool one point per extra block.
//!
//! The options are Horizon settings, read as text with `host.setting` because the typed reads are
//! client-only and this runs on the server.
//!
//! shortcut: the only group is "Ores" (`#c:ores` by name), upgrade if editable groups are wanted.
//! Not ported: see the README.
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use mod_api::{
    BreakBlockEvent, Flow, GameMode, Hand, Hooks, Host, LevelRef, LivingRef, Registrar, RegistrationContext,
    ServerHost, ServerTickEvent, ServerTickPhase, SettingSpec,
};

const ID: &str = "veinminer";

/// The original's defaults.
pub const MAX_CHAIN: usize = 100;
pub const SEARCH_RADIUS: i32 = 1;
pub const COOLDOWN: u64 = 20;

/// `#c:ores` by name (the original's "Ores" group).
pub fn is_ore(block: &str) -> bool {
    block == "minecraft:ancient_debris" || block.ends_with("_ore")
}

/// `#minecraft:pickaxes` by name.
pub fn is_pickaxe(item: &str) -> bool {
    item.ends_with("_pickaxe")
}

/// The options, as read from the settings.
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    pub must_sneak: bool,
    pub need_correct_tool: bool,
    pub merge_item_drops: bool,
    pub decrease_durability: bool,
    pub max_chain: usize,
    pub search_radius: i32,
    pub cooldown: u64,
}

impl Options {
    fn read(host: &dyn Host) -> Self {
        let text = |key: &str| host.setting(ID, key);
        let flag = |key: &str, default: bool| text(key).map_or(default, |v| v == "true");
        let number = |key: &str, default: f64| text(key).and_then(|v| v.parse::<f64>().ok()).unwrap_or(default);
        Options {
            must_sneak: flag("must_sneak", false),
            need_correct_tool: flag("need_correct_tool", true),
            merge_item_drops: flag("merge_item_drops", false),
            decrease_durability: flag("decrease_durability", true),
            max_chain: number("max_chain", MAX_CHAIN as f64).clamp(1.0, 1000.0) as usize,
            // `saveSearchRadius`: 1 to 5.
            search_radius: number("search_radius", f64::from(SEARCH_RADIUS)).clamp(1.0, 5.0) as i32,
            cooldown: number("cooldown", COOLDOWN as f64).max(0.0) as u64,
        }
    }
}

/// The vein through `origin`: the origin first, then the rest nearest layer first (`Veinmining.veinmine`
/// over `NormalStrategy`). `accept` says whether a block belongs to it.
pub fn find_vein(
    origin: [i32; 3],
    radius: i32,
    max_chain: usize,
    accept: &dyn Fn([i32; 3]) -> bool,
) -> Vec<[i32; 3]> {
    let mut vein = Vec::new();
    let mut seen = HashSet::from([origin]);
    let mut queue = VecDeque::from([origin]);
    while let Some(pos) = queue.pop_front() {
        if vein.len() >= max_chain {
            break;
        }
        vein.push(pos);
        for dx in -radius..=radius {
            for dy in -radius..=radius {
                for dz in -radius..=radius {
                    let next = [pos[0] + dx, pos[1] + dy, pos[2] + dz];
                    if accept(next) && seen.insert(next) {
                        queue.push_back(next);
                    }
                }
            }
        }
    }
    vein
}

/// One vein, waiting for the server tick.
#[derive(Debug)]
pub struct Veinmining {
    pub dimension: String,
    pub player: String,
    pub origin: [i32; 3],
    /// The blocks to mine, without the origin (vanilla breaks that).
    pub blocks: Vec<[i32; 3]>,
    pub merge: bool,
    /// The tool's durability, to wear it, if wearing is on and the tool has one.
    pub wear: Option<u32>,
}

impl Veinmining {
    fn loot_line(&self, [bx, by, bz]: [i32; 3]) -> String {
        let [x, y, z] = if self.merge { self.origin } else { [bx, by, bz] };
        format!(
            "execute in {} as {} run loot spawn {:.1} {:.3} {:.1} mine {bx} {by} {bz} mainhand",
            self.dimension,
            self.player,
            f64::from(x) + 0.5,
            f64::from(y) + 0.5,
            f64::from(z) + 0.5
        )
    }
    fn clear_line(&self, [bx, by, bz]: [i32; 3]) -> String {
        format!("execute in {} run setblock {bx} {by} {bz} minecraft:air", self.dimension)
    }
    fn wear_line(&self, mined: usize) -> Option<String> {
        let max = self.wear?;
        if mined == 0 {
            return None;
        }
        Some(format!(
            "execute as {} run item modify entity @s weapon.mainhand {{\"type\":\"minecraft:set_damage\",\"damage\":{:.6},\"add\":true}}",
            self.player,
            // `SetItemDamageFunction` floors a float: half a point keeps it from landing one short.
            -(mined as f64 + 0.5) / f64::from(max)
        ))
    }

    /// Mines the vein with `run`: each block's drops spawn, then the block is cleared. A block
    /// whose loot command fails stays, so nothing vanishes without its drops. Returns how many
    /// blocks were mined.
    pub fn run(&self, run: &mut dyn FnMut(&str) -> bool) -> usize {
        let mut mined = 0;
        for &b in &self.blocks {
            let loot = self.loot_line(b);
            if !run(&loot) {
                eprintln!("[veinminer] command failed, block left standing: {loot}");
                continue;
            }
            let clear = self.clear_line(b);
            if !run(&clear) {
                eprintln!("[veinminer] command failed: {clear}");
                continue;
            }
            mined += 1;
        }
        if let Some(line) = self.wear_line(mined)
            && !run(&line)
        {
            eprintln!("[veinminer] command failed: {line}");
        }
        mined
    }
}

#[derive(Default)]
struct Mod {
    queue: Mutex<Vec<Veinmining>>,
    /// The server tick count, and the tick each player may veinmine again.
    tick: AtomicU64,
    ready: Mutex<HashMap<String, u64>>,
}

fn block_name(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> String {
    level.block(host, pos).map(|k| k.id(host)).unwrap_or_default()
}

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn register(&self, registrar: &mut dyn Registrar, _: &RegistrationContext) {
        let settings = [
            SettingSpec::toggle("must_sneak", "veinminer.setting.must_sneak", false),
            SettingSpec::toggle("need_correct_tool", "veinminer.setting.need_correct_tool", true),
            SettingSpec::toggle("merge_item_drops", "veinminer.setting.merge_item_drops", false),
            SettingSpec::toggle("decrease_durability", "veinminer.setting.decrease_durability", true),
            SettingSpec::range("max_chain", "veinminer.setting.max_chain", 1.0, 1000.0, 1.0, MAX_CHAIN as f64),
            SettingSpec::range("search_radius", "veinminer.setting.search_radius", 1.0, 5.0, 1.0, f64::from(SEARCH_RADIUS)),
            SettingSpec::range("cooldown", "veinminer.setting.cooldown", 0.0, 200.0, 1.0, COOLDOWN as f64),
        ];
        for s in settings {
            if let Err(e) = registrar.register_setting(s) {
                eprintln!("[veinminer] setting refused: {e:?}");
            }
        }
    }

    fn break_block(&self, host: &mut dyn ServerHost, event: &mut BreakBlockEvent<'_>) -> Flow {
        if !host.on(ID) {
            return Flow::Continue;
        }
        let Some(player) = host.players().acting_player() else { return Flow::Continue };
        let host: &dyn Host = &*host;
        if player.game_mode(host) != Some(GameMode::Survival) {
            return Flow::Continue;
        }
        let opts = Options::read(host);
        if opts.must_sneak && player.entity().is_shift_key_down(host) != Some(true) {
            return Flow::Continue;
        }
        let Some(held) = LivingRef::new(player.entity()).hand_item(host, Hand::Main) else { return Flow::Continue };
        let item = held.item();
        if !is_pickaxe(&item.id(host)) {
            return Flow::Continue;
        }
        let (Some(level), Some(dim), Some(name)) = (player.level(host), player.dimension(host), player.name(host)) else {
            return Flow::Continue;
        };
        let origin = block_name(host, level, event.pos);
        if !is_ore(&origin) {
            return Flow::Continue;
        }
        let now = self.tick.load(Ordering::Relaxed);
        if self.ready.lock().unwrap_or_else(|e| e.into_inner()).get(&name).is_some_and(|&t| now < t) {
            return Flow::Continue;
        }
        // The tool's wear: known durability, and at least one point left after the breaking itself.
        let max = host.max_damage(item).filter(|&m| m > 0);
        let wear = if opts.decrease_durability { max } else { None };
        let spare = wear.map(|m| m.saturating_sub(held.damage()).saturating_sub(2) as usize);
        let accept = |p: [i32; 3]| {
            level.block(host, p).is_some_and(|k| is_ore(&k.id(host)))
                && (!opts.need_correct_tool
                    || level
                        .block_state(host, p)
                        .and_then(|s| player.has_correct_tool_for_drops(host, s))
                        .unwrap_or(false))
        };
        let mut vein = find_vein(event.pos, opts.search_radius, opts.max_chain, &accept);
        vein.remove(0);
        vein.truncate(spare.unwrap_or(usize::MAX));
        if vein.is_empty() {
            return Flow::Continue;
        }
        self.ready.lock().unwrap_or_else(|e| e.into_inner()).insert(name.clone(), now + opts.cooldown);
        let mut queue = self.queue.lock().unwrap_or_else(|e| e.into_inner());
        queue.push(Veinmining {
            dimension: dim.to_string(),
            player: name,
            origin: event.pos,
            blocks: vein,
            merge: opts.merge_item_drops,
            wear,
        });
        Flow::Continue
    }

    fn server_tick(&self, host: &mut dyn ServerHost, event: &mut ServerTickEvent<'_>) {
        if !matches!(event.phase, ServerTickPhase::ServerTickEnd { .. }) {
            return;
        }
        let now = self.tick.fetch_add(1, Ordering::Relaxed) + 1;
        self.ready.lock().unwrap_or_else(|e| e.into_inner()).retain(|_, &mut t| t > now);
        let pending = std::mem::take(&mut *self.queue.lock().unwrap_or_else(|e| e.into_inner()));
        for vein in pending {
            vein.run(&mut |line| host.commands().run_command(line));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world(blocks: &[[i32; 3]]) -> HashSet<[i32; 3]> {
        blocks.iter().copied().collect()
    }

    #[test]
    fn names() {
        assert!(is_ore("minecraft:diamond_ore") && is_ore("minecraft:deepslate_coal_ore"));
        assert!(is_ore("minecraft:ancient_debris") && !is_ore("minecraft:stone"));
        assert!(is_pickaxe("minecraft:iron_pickaxe") && !is_pickaxe("minecraft:iron_axe"));
    }

    #[test]
    fn the_vein_is_the_connected_ores_nearest_first() {
        let w = world(&[[0, 0, 0], [1, 0, 0], [2, 1, 0], [5, 0, 0]]);
        let v = find_vein([0, 0, 0], 1, 100, &|p| w.contains(&p));
        assert_eq!(v, vec![[0, 0, 0], [1, 0, 0], [2, 1, 0]]);
        // A bigger radius bridges the gap, still not 5 away.
        let v = find_vein([0, 0, 0], 3, 100, &|p| w.contains(&p));
        assert_eq!(v.len(), 4);
    }

    #[test]
    fn the_chain_limit_counts_the_origin() {
        let w = world(&[[0, 0, 0], [1, 0, 0], [2, 0, 0], [3, 0, 0]]);
        assert_eq!(find_vein([0, 0, 0], 1, 3, &|p| w.contains(&p)).len(), 3);
        assert_eq!(find_vein([0, 0, 0], 1, 1, &|p| w.contains(&p)), vec![[0, 0, 0]]);
    }

    fn mining(merge: bool) -> Veinmining {
        Veinmining {
            dimension: "minecraft:overworld".into(),
            player: "Steve".into(),
            origin: [-3, 12, 2],
            blocks: vec![[-3, 13, 2], [-4, 13, 2]],
            merge,
            wear: Some(250),
        }
    }

    #[test]
    fn each_block_drops_then_clears_then_the_tool_wears() {
        let mut c = Vec::new();
        let n = mining(false).run(&mut |l| {
            c.push(l.to_string());
            true
        });
        assert_eq!(n, 2);
        assert_eq!(c.len(), 5);
        assert_eq!(c[0], "execute in minecraft:overworld as Steve run loot spawn -2.5 13.500 2.5 mine -3 13 2 mainhand");
        assert_eq!(c[1], "execute in minecraft:overworld run setblock -3 13 2 minecraft:air");
        assert!(c[4].ends_with("{\"type\":\"minecraft:set_damage\",\"damage\":-0.010000,\"add\":true}"), "{}", c[4]);
    }

    #[test]
    fn merged_drops_land_at_the_origin() {
        let mut c = Vec::new();
        mining(true).run(&mut |l| {
            c.push(l.to_string());
            true
        });
        assert!(c[0].contains("loot spawn -2.5 12.500 2.5 mine -3 13 2"), "{}", c[0]);
    }

    #[test]
    fn a_block_whose_loot_fails_stays() {
        let mut c = Vec::new();
        let n = mining(false).run(&mut |l| {
            c.push(l.to_string());
            !l.contains("mine -4 13 2")
        });
        assert_eq!(n, 1);
        assert!(!c.iter().any(|l| l.contains("setblock -4 13 2")));
        assert!(c.last().unwrap().contains("-0.006000"), "one block worn");
    }
}
