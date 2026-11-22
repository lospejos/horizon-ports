//! Trinkets 3.7.2 (Emi, MIT): the player's extra equipment slots, as Homestead's mods declare them
//! (tools/trinkets_port.py writes the table, `data.rs`).
//!
//! - The worn stacks are a player attachment, `trinkets:worn`: a compound of slot id -> item NBT
//!   (`{id, count, components}`), saved with the player, kept through death when nothing drops it,
//!   synced to the player's client (elytraslot glides on the cape's elytra there). Any crate reads
//!   it by declaring the same `AttachmentType<Compound>`: elytraslot, radiantgear, universalgraves.
//! - The menu (TrinketInventoryScreen's slots) is a server menu, opened by the Trinkets key
//!   (G, `key.trinkets.open` in Controls): the client sends `trinkets:open`, the server opens the
//!   menu on its next tick. A click on an inventory item wears it, a click on a worn one takes it
//!   off. Server menus move nothing on their own: `menu_clicked` does the moves.
//! - A right click with a trinket wears it in the first empty slot that takes it (TrinketItem.use).
//! - The tooltip names the slots an item fits.
//! - WebAssembly build: no death drop and no worn-totem save (both need engine mixins).
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub mod data;
pub use data::SLOTS;

use mod_api::{
    AttachmentCodec, AttachmentError, AttachmentType, ClientHost, Compound, Flow, Hand, Holder, HostSide, Hooks,
    Host, IPayloadContext, IPayloadHandler, Identifier, InputKeyEvent, InteractionResult, ItemStack,
    ItemTooltipEvent, KeyAction, KeyRegistration, MenuClickedEvent, MenuClosedEvent, MenuReplace, PayloadBytes,
    PayloadHandleError, PayloadId, PlayerRef, Registrar, RegistrationContext, RightClickItemEvent, ServerHost,
    ServerTickEvent, ServerTickPhase, SoundSource, Tag, Text,
};

fn same(c: &Compound) -> Result<Compound, AttachmentError> {
    Ok(c.clone())
}
fn empty() -> Compound {
    Compound::new()
}

/// The worn trinkets of a player: slot id ("chest/cape") -> the stack's NBT.
pub static WORN: AttachmentType<Compound> = AttachmentType {
    namespace: "trinkets",
    name: "worn",
    default: empty,
    codec: Some(AttachmentCodec { encode: same, decode: same }),
    copy_on_death: true,
    sync: true,
};

const OPEN: &str = "trinkets:open";

/// Whether slot `i` takes `item` (its trinkets tag, which `all.json` adds to).
pub fn accepts(i: usize, item: &str) -> bool {
    SLOTS.get(i).is_some_and(|s| s.3.contains(&item))
}

/// Whether some slot takes `item`.
pub fn is_trinket(item: &str) -> bool {
    SLOTS.iter().any(|s| s.3.contains(&item))
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// The custom_data key of a stand-in for a worn item the game can't make (a mod that isn't installed): the saved
/// entry, written back unchanged.
const UNKNOWN: &str = "trinkets:unknown";

/// The saved entry a stand-in carries.
fn unknown_entry(stack: &ItemStack) -> Option<Compound> {
    let comps = stack.components()?;
    let Some(Tag::Compound(data)) = comps.as_nbt().get(&"minecraft:custom_data".into()) else { return None };
    match data.get(&UNKNOWN.into())? {
        Tag::Compound(c) => Some(c.clone()),
        _ => None,
    }
}

fn is_unknown(stack: &ItemStack) -> bool {
    unknown_entry(stack).is_some()
}

/// A barrier standing in for the saved entry `raw` ("Not ported: vc_gliders:paraglider_wood"); it stays in its
/// slot and is saved back as it was.
fn stand_in(host: &dyn Host, raw: &Compound) -> Option<ItemStack> {
    let id = match raw.get(&"id".into())? {
        Tag::String(s) => s.to_string_lossy(),
        _ => return None,
    };
    if id.is_empty() || id == "minecraft:air" {
        return None;
    }
    let key = host.item_key(&Identifier::parse("minecraft:barrier").ok()?)?;
    let mut st = ItemStack::new(key, 1)?;
    let line = |t: &str, color: &str| Tag::compound([("text", Tag::String(t.into())), ("color", Tag::String(color.into())), ("italic", Tag::Byte(0))]);
    let mut patch = Compound::new();
    patch.insert("minecraft:custom_name".into(), line(&format!("Not ported: {id}"), "red"));
    patch.insert("minecraft:lore".into(), Tag::List(vec![line("Kept in this slot until its mod is installed", "gray")]));
    patch.insert("minecraft:custom_data".into(), Tag::compound([(UNKNOWN, Tag::Compound(raw.clone()))]));
    st.apply_components(&patch).ok()?;
    Some(st)
}

/// The player's worn stacks, one per slot of `SLOTS`. A worn item the game can't make comes back as a stand-in
/// (`stand_in`), so the slot stays taken and `set_worn` writes the entry back.
fn worn(host: &mut dyn Host, uuid: u128) -> Vec<Option<ItemStack>> {
    let c = WORN.get(host, Holder::Player(uuid)).unwrap_or_default();
    SLOTS
        .iter()
        .map(|s| match c.get(&s.0.into()) {
            Some(Tag::Compound(t)) => ItemStack::from_nbt(&*host, t).or_else(|| stand_in(&*host, t)),
            _ => None,
        })
        .collect()
}

fn set_worn(host: &mut dyn Host, uuid: u128, stacks: &[Option<ItemStack>]) -> bool {
    let mut c = WORN.get(host, Holder::Player(uuid)).unwrap_or_default();
    // Entries under ids that aren't slots of this table are kept as they are.
    for s in SLOTS {
        c.remove(&s.0.into());
    }
    for (s, stack) in SLOTS.iter().zip(stacks) {
        let Some(stack) = stack.as_ref().filter(|s| s.count() > 0) else { continue };
        if let Some(t) = unknown_entry(stack).or_else(|| stack.to_nbt(&*host)) {
            c.insert(s.0.into(), Tag::Compound(t));
        }
    }
    WORN.set(host, Holder::Player(uuid), c).is_ok()
}

fn item_id(host: &dyn Host, stack: &ItemStack) -> String {
    stack.item().id(host).to_string()
}

// ---- the menu ----

fn rows() -> u8 {
    SLOTS.len().div_ceil(9).clamp(1, 6) as u8
}

/// What the menu shows: each worn stack, or the slot's placeholder named for the slot.
fn menu_items(host: &dyn Host, worn: &[Option<ItemStack>]) -> Vec<Option<ItemStack>> {
    let mut out: Vec<Option<ItemStack>> = SLOTS
        .iter()
        .zip(worn)
        .map(|(s, w)| {
            w.clone().or_else(|| {
                let key = host.item_key(&Identifier::parse(s.2).ok()?)?;
                let mut p = ItemStack::new(key, 1)?;
                let mut patch = Compound::new();
                patch.insert(
                    "minecraft:custom_name".into(),
                    Tag::compound([("text", Tag::String(format!("{} (empty)", s.1).as_str().into())), ("italic", Tag::Byte(0))]),
                );
                patch.insert(
                    "minecraft:lore".into(),
                    Tag::List(vec![Tag::compound([
                        ("text", Tag::String("Click a trinket in your inventory to wear it.".into())),
                        ("color", Tag::String("gray".into())),
                        ("italic", Tag::Byte(0)),
                    ])]),
                );
                let _ = p.apply_components(&patch);
                Some(p)
            })
        })
        .collect();
    out.resize(rows() as usize * 9, None);
    out
}

/// A menu slot past the menu's own: the player's inventory slot (main 9..36 first, then the hotbar).
fn inventory_slot(size: usize, slot: i32) -> Option<u32> {
    let rel = usize::try_from(slot).ok()?.checked_sub(size)?;
    match rel {
        0..27 => Some(rel as u32 + 9),
        27..36 => Some(rel as u32 - 27),
        _ => None,
    }
}

/// The main inventory (slots 0..36), and whether every slot was read as it is.
///
/// `inventory_slots` answers None for the whole inventory when any slot holds an item the game can't hand to a mod
/// (an item of a mod that isn't installed, as live-world players carry). Reading nothing then made every slot look
/// empty: taking a trinket off wrote it over hotbar slot 0, and clicking an inventory item did nothing. Now each slot
/// is read on its own through the player's open menu (`by_menu`); an unknown item still reads as nothing there, so
/// with an incomplete read nothing is written into a slot that looks empty (the game's `give_stack` places it
/// instead).
fn inventory(host: &mut dyn ServerHost, player: PlayerRef) -> (Vec<Option<ItemStack>>, bool) {
    let mut inv = vec![None; 36];
    if let Some(slots) = player.inventory_slots(&*host) {
        for (slot, stack) in slots {
            if (slot as usize) < 36 {
                inv[slot as usize] = Some(stack);
            }
        }
        return (inv, true);
    }
    for (slot, stack) in by_menu(&*host, player) {
        if slot < 36 {
            inv[slot] = stack;
        }
    }
    (inv, false)
}

/// Drops `stack` where the player stands.
fn drop_at(host: &mut dyn ServerHost, player: PlayerRef, stack: ItemStack) {
    let Some(dim) = player.dimension(&*host).map(|d| d.to_string()) else { return };
    let Some(level) = host.level(&dim) else { return };
    let Some(pos) = player.entity().position(&*host) else { return };
    let _ = level.pop_resource(host, [pos[0].floor() as i32, pos[1].floor() as i32, pos[2].floor() as i32], stack);
}

/// The player's inventory slots (`ENTITY_ITEMS` numbers) that the player's open menu shows, each read on its own:
/// the menu's slots backed by the inventory (`menu_slot_backing`). A slot whose item the game can't hand over reads as
/// None. (The player's `ENTITY_ITEMS` handler reads nothing from a menu click here, so it isn't used.)
fn by_menu(host: &dyn Host, player: PlayerRef) -> Vec<(usize, Option<ItemStack>)> {
    let mut out = Vec::new();
    let Some(menu) = player.container_menu(host) else { return out };
    for i in 0..menu.slot_count(host).unwrap_or(0) {
        if let Some(mod_api::MenuSlotBacking::Player(p)) = menu.slot_backing(host, i) {
            out.push((p, menu.slot(host, i).flatten()));
        }
    }
    out
}

/// Puts `stack` in the player's inventory as the game's `Inventory.add` does (onto the same item, then the selected
/// slot, then the first empty one; what doesn't fit is thrown to the player). Whether it was given.
fn give(host: &mut dyn ServerHost, player: PlayerRef, stack: ItemStack) -> bool {
    match player.give_stack(host, stack) {
        Ok(()) => true,
        Err(e) => {
            eprintln!("[trinkets] give_stack: {e:?}");
            false
        }
    }
}

#[derive(Default)]
struct Mod {
    /// Players who pressed the key, to open the menu for on the next server tick.
    opens: Arc<Mutex<Vec<u128>>>,
    /// The open trinket menu's window of each player.
    menus: Mutex<HashMap<u128, i32>>,
}

struct OpenHandler(Arc<Mutex<Vec<u128>>>);

impl IPayloadHandler for OpenHandler {
    fn handle(&self, _payload: PayloadBytes, context: &mut dyn IPayloadContext) -> Result<(), PayloadHandleError> {
        // One pending open per player, however often the key packet comes.
        if let Ok(p) = context.player() {
            let uuid = p.uuid();
            let mut opens = lock(&self.0);
            if !opens.contains(&uuid) {
                opens.push(uuid);
            }
        }
        Ok(())
    }
}

impl Mod {
    fn open(&self, host: &mut dyn ServerHost, uuid: u128) {
        let worn = worn(host, uuid);
        let items = menu_items(&*host, &worn);
        match host.open_menu(uuid, rows(), Text::literal("Trinkets"), items, MenuReplace::CloseFirst) {
            Ok(Some(window)) => {
                lock(&self.menus).insert(uuid, window);
            }
            Ok(None) => {}
            Err(e) => eprintln!("[trinkets] open_menu: {e:?}"),
        }
    }

    /// A click in the player's trinket menu.
    fn click(&self, host: &mut dyn ServerHost, uuid: u128, window: i32, slot: i32) {
        let Some(player) = host.player(uuid) else { return };
        let size = rows() as usize * 9;
        let mut worn = worn(host, uuid);
        let (mut inv, complete) = inventory(host, player);
        let mut changed: Vec<u32> = Vec::new();
        // Stacks for the game to put in the inventory (never into a slot that only reads as empty).
        let mut gives: Vec<ItemStack> = Vec::new();
        let message;
        // Room for one more stack: a free slot, or (with an incomplete read) the game decides and throws what
        // doesn't fit.
        let room = !complete || inv.iter().any(Option::is_none);
        if let Ok(i) = usize::try_from(slot)
            && i < SLOTS.len()
        {
            // TrinketSlot: take a worn stack off, into the inventory.
            if worn[i].as_ref().is_some_and(is_unknown) {
                let _ = player.action_bar(host, Text::literal("This item's mod isn't installed: it stays in its slot"));
                return;
            }
            let Some(stack) = worn[i].take() else { return };
            if room {
                gives.push(stack);
                message = None;
            } else {
                worn[i] = Some(stack);
                message = Some("No room in your inventory");
            }
        } else if let Some(s) = inventory_slot(size, slot) {
            // A click on an inventory item: into the first empty slot that takes it, else swapped
            // with the first one that does.
            let s = s as usize;
            let Some(stack) = inv[s].clone() else { return };
            let id = item_id(&*host, &stack);
            let fits: Vec<usize> = (0..SLOTS.len()).filter(|&i| accepts(i, &id) && !worn[i].as_ref().is_some_and(is_unknown)).collect();
            match fits.iter().copied().find(|&i| worn[i].is_none()).or(fits.first().copied()) {
                Some(i) if stack.count() == 1 => {
                    inv[s] = worn[i].replace(stack);
                    changed.push(s as u32);
                    message = None;
                }
                Some(i) => {
                    if worn[i].is_some() && !room {
                        message = Some("No room in your inventory");
                    } else {
                        let split = stack.split(1);
                        inv[s] = split.remainder;
                        changed.push(s as u32);
                        if let Some(old) = std::mem::replace(&mut worn[i], split.taken) {
                            gives.push(old);
                        }
                        message = None;
                    }
                }
                None => message = Some("That doesn't go in a trinket slot"),
            }
        } else {
            return;
        }
        for s in changed {
            if let Err(e) = player.set_inventory_slot(host, s, inv[s as usize].clone()) {
                eprintln!("[trinkets] set_inventory_slot {s}: {e:?}");
            }
        }
        let mut message = message;
        for g in gives {
            if !give(host, player, g.clone()) {
                // Not given: it goes back on, or (when its slot is taken again) stays with the click undone.
                match (0..SLOTS.len()).find(|&i| worn[i].is_none() && accepts(i, &item_id(&*host, &g))) {
                    Some(i) => worn[i] = Some(g),
                    None => drop_at(host, player, g),
                }
                message = Some("No room in your inventory");
            }
        }
        set_worn(host, uuid, &worn);
        let items = menu_items(&*host, &worn);
        if let Err(e) = host.menu_set_items(uuid, window, items) {
            eprintln!("[trinkets] menu_set_items: {e:?}");
        }
        if let Some(m) = message {
            let _ = player.action_bar(host, Text::literal(m));
        }
    }
}

/// Items tagged into a trinket slot that aren't TrinketItems in Java: their own use comes first (a backpack opens,
/// a wireless terminal opens its terminal), so a right click doesn't wear them; the menu still does.
fn own_use(id: &str) -> bool {
    id.starts_with("sophisticatedbackpacks:") || id == "toms_storage:ts.adv_wireless_terminal"
}

/// TrinketItem.use: the slot a right click with `stack` wears it in, if any.
fn wear_slot(host: &mut dyn Host, uuid: u128, stack: &ItemStack) -> Option<(usize, Vec<Option<ItemStack>>)> {
    let id = item_id(&*host, stack);
    // An elytra (anything equippable) still goes to its armour slot, as in Java: ElytraItem isn't a
    // TrinketItem.
    if !is_trinket(&id) || own_use(&id) || mod_api::ItemResource::of(stack).equippable_slot(&*host).is_some() {
        return None;
    }
    let worn = worn(host, uuid);
    let i = (0..SLOTS.len()).find(|&i| worn[i].is_none() && accepts(i, &id))?;
    Some((i, worn))
}

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn register(&self, registrar: &mut dyn Registrar, _: &RegistrationContext) {
        registrar.register_key_mapping(KeyRegistration::new("open", "key.keyboard.g").label("key.trinkets.open"));
        let mut payloads = registrar.payload_handlers();
        payloads
            .registrar("1")
            .optional()
            .play_to_server(PayloadId(OPEN.into()), Arc::new(OpenHandler(self.opens.clone())));
    }

    /// The Trinkets key, while playing or in the player's own inventory: asks the server for the menu.
    fn input_key(&self, host: &mut dyn ClientHost, event: &mut InputKeyEvent<'_>) {
        if !matches!(event.action, KeyAction::Press) {
            return;
        }
        let Some((key, _)) = host.mod_key("trinkets", "open") else { return };
        if event.key != key {
            return;
        }
        let inventory = host.observed_screen_id().is_some_and(|s| s.id == "inventory") && host.screen_title().is_none();
        if !(host.playing() || inventory) {
            return;
        }
        if let Err(e) = host.send_payload_to_server(&PayloadId(OPEN.into()), PayloadBytes(Vec::new())) {
            eprintln!("[trinkets] open: {e:?}");
        }
    }

    fn server_tick(&self, host: &mut dyn ServerHost, event: &mut ServerTickEvent<'_>) {
        if !matches!(event.phase, ServerTickPhase::ServerTickEnd { .. }) {
            return;
        }
        let opens = std::mem::take(&mut *lock(&self.opens));
        for uuid in opens {
            // Already looking at it: don't rebuild the menu and resend the inventory.
            let window = lock(&self.menus).get(&uuid).copied();
            if window.is_some_and(|w| host.menu(uuid, w).is_some()) {
                continue;
            }
            self.open(host, uuid);
        }
    }

    fn menu_clicked(&self, host: &mut dyn ServerHost, event: &mut MenuClickedEvent) {
        if lock(&self.menus).get(&event.uuid) != Some(&event.window) {
            return;
        }
        // PICKUP and QUICK_MOVE; drags, swaps and throws do nothing.
        if !matches!(event.mode, 0 | 1) {
            return;
        }
        self.click(host, event.uuid, event.window, event.slot);
    }

    fn menu_closed(&self, _host: &mut dyn ServerHost, event: &mut MenuClosedEvent) {
        let mut menus = lock(&self.menus);
        if menus.get(&event.uuid) == Some(&event.window) {
            menus.remove(&event.uuid);
        }
    }

    /// TrinketItem.use: a right click with a trinket wears it in the first empty slot that takes it.
    /// The client decides from its synced copy, so it doesn't use the item itself.
    fn right_click_item(&self, host: HostSide<'_>, event: &mut RightClickItemEvent<'_>) -> Flow {
        let click = event.click;
        let worn_it = match host {
            HostSide::Server(host) => {
                let Some((i, mut worn)) = wear_slot(host, click.player, click.stack) else { return Flow::Continue };
                let Some(player) = host.player(click.player) else { return Flow::Continue };
                let hand_slot = match click.hand {
                    Hand::Main => player.selected_slot(&*host).map(|s| s as u32),
                    _ => Some(40),
                };
                let Some(hand_slot) = hand_slot else { return Flow::Continue };
                let split = click.stack.clone().split(1);
                if player.set_inventory_slot(host, hand_slot, split.remainder).is_err() {
                    return Flow::Continue;
                }
                worn[i] = split.taken;
                set_worn(host, click.player, &worn);
                let _ = player.play_sound(host, "minecraft:item.armor.equip_generic", SoundSource::Players, 1.0, 1.0);
                true
            }
            HostSide::Client(host) => wear_slot(host, click.player, click.stack).is_some(),
        };
        if !worn_it {
            return Flow::Continue;
        }
        *event.cancellation_result = InteractionResult::Success;
        event.cancelled = true;
        Flow::Cancel
    }

    /// TrinketsClient's tooltip: which trinket slots take the item (placeholders show nothing).
    fn item_tooltip(&self, host: &dyn ClientHost, event: &mut ItemTooltipEvent<'_>) {
        let id = item_id(host, event.stack);
        if id.starts_with("trinkets:slot_") {
            return;
        }
        let mut names: Vec<&str> = SLOTS.iter().filter(|s| s.3.contains(&id.as_str())).map(|s| s.1).collect();
        if names.is_empty() {
            return;
        }
        names.dedup();
        // The key as the player bound it; no hint when it's unbound.
        let hint = host
            .mod_key("trinkets", "open")
            .and_then(|(key, mods)| key_label(&key, mods.bits))
            .map(|k| format!(" ({k})"))
            .unwrap_or_default();
        let line = if names.len() == 1 {
            format!("§7Equippable in the §9{}§7 trinket slot{hint}", names[0])
        } else {
            format!("§7Equippable in the trinket slots: §9{}§7{hint}", names.join(", "))
        };
        event.lines.push((Text::literal(line), 0xFFAAAAAA));
    }
}

/// A key name for a tooltip from its saved form (`key.keyboard.g`, `key.keyboard.left.shift`, `key.mouse.4`) and
/// GLFW modifier bits: `G`, `Left Shift`, `Mouse 4`, `Ctrl + G`. None when unbound.
fn key_label(key: &str, mods: u32) -> Option<String> {
    let (kind, rest) = if let Some(r) = key.strip_prefix("key.keyboard.") {
        ("", r)
    } else if let Some(r) = key.strip_prefix("key.mouse.") {
        ("Mouse ", r)
    } else {
        return None;
    };
    if rest.is_empty() || rest == "unknown" {
        return None;
    }
    let words: Vec<String> = rest
        .split('.')
        .map(|w| match w {
            "keypad" => "Keypad".to_owned(),
            w => {
                let mut c = w.chars();
                match c.next() {
                    Some(f) => f.to_uppercase().chain(c).collect(),
                    None => String::new(),
                }
            }
        })
        .collect();
    let mut name = format!("{kind}{}", words.join(" "));
    for (bit, m) in [(8, "Super"), (4, "Alt"), (1, "Shift"), (2, "Ctrl")] {
        if mods & bit != 0 {
            name = format!("{m} + {name}");
        }
    }
    Some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_labels() {
        assert_eq!(key_label("key.keyboard.g", 0).as_deref(), Some("G"));
        assert_eq!(key_label("key.keyboard.left.shift", 0).as_deref(), Some("Left Shift"));
        assert_eq!(key_label("key.keyboard.keypad.1", 0).as_deref(), Some("Keypad 1"));
        assert_eq!(key_label("key.mouse.4", 0).as_deref(), Some("Mouse 4"));
        assert_eq!(key_label("key.keyboard.t", 2).as_deref(), Some("Ctrl + T"));
        assert_eq!(key_label("key.keyboard.unknown", 0), None);
    }

    #[test]
    fn inventory_slots_follow_the_menu() {
        let size = rows() as usize * 9;
        assert_eq!(inventory_slot(size, size as i32), Some(9));
        assert_eq!(inventory_slot(size, size as i32 + 26), Some(35));
        assert_eq!(inventory_slot(size, size as i32 + 27), Some(0));
        assert_eq!(inventory_slot(size, size as i32 + 35), Some(8));
        assert_eq!(inventory_slot(size, 3), None);
    }

    #[test]
    fn backpacks_open_rather_than_wear() {
        assert!(own_use("sophisticatedbackpacks:iron_backpack"));
        assert!(!own_use("things:socks"));
    }

    #[test]
    fn the_cape_takes_an_elytra() {
        let cape = SLOTS.iter().position(|s| s.0 == "chest/cape").unwrap();
        assert!(accepts(cape, "minecraft:elytra"));
        assert!(is_trinket("minecraft:totem_of_undying"));
    }
}
