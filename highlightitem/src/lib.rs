//! HighLightItem for Horizon: over an item in a container screen, every slot holding a matching item
//! is lit (HighLightItem 3.1.0 by elmital, GPL-3.0-only).
//!
//! `screen_render_foreground` finds the hovered slot from the slot boxes and the pointer, and fills the
//! boxes of the slots whose item matches the comparator. The options are Horizon settings; the toggle
//! and comparator keys are mod key mappings, handled in `input_key`.
//!
//! shortcut: modifier keys on the two mappings are ignored (the key alone fires), upgrade if a clash shows up.
use mod_api::{
    ClientHost, Hooks, Host, InputKeyEvent, ItemStack, KeyAction, KeyRegistration, MenuSlotKind, Registrar,
    RegistrationContext, ScreenRenderForegroundEvent, SettingSpec, Tag,
};

const ID: &str = "highlightitem";
const UNBOUND: &str = "key.keyboard.unknown";
const COMPARATORS: [&str; 7] = [
    "item_only",
    "item_and_amount",
    "item_and_nbt",
    "item_and_nbt_and_amount",
    "name_only",
    "name_and_amount",
    "namespace",
];
const HOVERED: [&str; 3] = ["vanilla", "colored", "colored_not_empty"];
const SCREENS: [&str; 5] = [
    "everywhere",
    "exclude_creative",
    "player_inventory",
    "player_inventory_and_exclude_creative",
    "storage_only",
];

fn keys(prefix: &str, names: &[&str]) -> Vec<String> {
    names.iter().map(|n| format!("highlightitem.{prefix}.{n}")).collect()
}

/// `minecraft:custom_name` as text, the part of vanilla's hover name a stack carries itself.
fn custom_name(stack: &ItemStack) -> Option<String> {
    let tag = stack.components()?.as_nbt().get(&"minecraft:custom_name".into())?;
    Some(match tag {
        Tag::String(s) => s.to_string().unwrap_or_default(),
        other => format!("{other:?}"),
    })
}

fn namespace(host: &dyn Host, stack: &ItemStack) -> String {
    let id = stack.item().id(host);
    id.split(':').next().unwrap_or_default().to_owned()
}

/// `ItemComparator.Comparators`, by index into `COMPARATORS`.
fn matches(host: &dyn Host, mode: u32, a: &ItemStack, b: &ItemStack) -> bool {
    let item = a.item() == b.item();
    let amount = a.count() == b.count();
    let nbt = item && a.same_components(b);
    let name = match (custom_name(a), custom_name(b)) {
        (Some(x), Some(y)) => x == y,
        (None, None) => item,
        _ => false,
    };
    match mode {
        0 => item,
        1 => item && amount,
        2 => nbt,
        3 => nbt && amount,
        4 => name,
        5 => name && amount,
        _ => namespace(host, a).eq_ignore_ascii_case(&namespace(host, b)),
    }
}

/// `Configurator.ScreenContext`: whether a slot is left out. `kind` is None when the game can't tell.
fn skip(screen: u32, creative: bool, kind: Option<&MenuSlotKind>) -> bool {
    let player = kind.map(|k| matches!(k, MenuSlotKind::Player(_)));
    if creative {
        match screen {
            1 => return true,
            3 => return !matches!(kind, Some(MenuSlotKind::Player(i)) if *i < 9),
            _ => {}
        }
    }
    match (screen, player) {
        (2 | 3, Some(p)) => !p,
        (4, Some(p)) => p,
        _ => false,
    }
}

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn register(&self, registrar: &mut dyn Registrar, _: &RegistrationContext) {
        registrar.register_key_mapping(KeyRegistration::new("toggle", UNBOUND).label("key.highlightitem.toggle"));
        registrar.register_key_mapping(KeyRegistration::new("comparator", UNBOUND).label("key.highlightitem.comparator"));
        let settings = [
            SettingSpec::toggle("enabled", "highlightitem.setting.enabled", true),
            SettingSpec::choice("comparator", "highlightitem.setting.comparator", keys("comparator", &COMPARATORS), 0),
            SettingSpec::choice("hovered", "highlightitem.setting.hovered", keys("hovered", &HOVERED), 0),
            SettingSpec::choice("screen", "highlightitem.setting.screen", keys("screen", &SCREENS), 1),
            SettingSpec::color("color", "highlightitem.setting.color", 0xFF_FF_FF),
            SettingSpec::range("opacity", "highlightitem.setting.opacity", 0.0, 100.0, 1.0, 45.0),
        ];
        for s in settings {
            if let Err(e) = registrar.register_setting(s) {
                eprintln!("[highlightitem] setting refused: {e:?}");
            }
        }
    }

    fn input_key(&self, host: &mut dyn ClientHost, event: &mut InputKeyEvent<'_>) {
        if !matches!(event.action, KeyAction::Press) || event.key == UNBOUND {
            return;
        }
        let bound = |name: &str| host.mod_key(ID, name).is_some_and(|(key, _)| key == event.key);
        if bound("toggle") {
            let on = host.setting_bool("enabled").unwrap_or(true);
            host.set_setting(ID, "enabled", if on { "false" } else { "true" });
        } else if bound("comparator") {
            let next = (host.setting_choice("comparator").unwrap_or(0) + 1) % COMPARATORS.len() as u32;
            host.set_setting(ID, "comparator", &next.to_string());
        }
    }

    fn screen_render_foreground(&self, host: &dyn ClientHost, event: &mut ScreenRenderForegroundEvent<'_>) {
        if !host.on(ID) || host.setting_bool("enabled") == Some(false) || event.screen.slots == 0 {
            return;
        }
        let Some(menu) = host.local_player().and_then(|p| p.container_menu(host)) else { return };
        let Some(boxes) = menu.slot_boxes(host) else { return };
        let [mx, my] = event.screen.mouse;
        let Some(hovered) = boxes
            .iter()
            .position(|b| b.is_some_and(|[x, y, w, h]| mx >= x && mx < x + w && my >= y && my < y + h))
        else {
            return;
        };
        let rgb = host.setting_color("color").unwrap_or(0xFF_FF_FF);
        let alpha = (host.setting_f64("opacity").unwrap_or(45.0).clamp(0.0, 100.0) * 2.55).round() as u32;
        let argb = alpha << 24 | rgb;
        let held = menu.slot(host, hovered).flatten();

        // The game draws its own highlight on the hovered slot; "coloured" paints ours over it.
        let hovered_mode = host.setting_choice("hovered").unwrap_or(0);
        if hovered_mode == 1 || (hovered_mode == 2 && held.is_some()) {
            event.draw.fill(boxes[hovered].unwrap_or_default(), argb);
        }
        let Some(held) = held else { return };

        let mode = host.setting_choice("comparator").unwrap_or(0);
        let screen = host.setting_choice("screen").unwrap_or(1);
        let creative = event.screen.id == "creative";
        let kinds = if screen == 0 { None } else { menu.slot_kinds(host) };
        for (i, rect) in boxes.iter().enumerate() {
            let Some(rect) = rect else { continue };
            if i == hovered || skip(screen, creative, kinds.as_ref().and_then(|k| k.get(i))) {
                continue;
            }
            if let Some(other) = menu.slot(host, i).flatten()
                && matches(host, mode, &held, &other)
            {
                event.draw.fill(*rect, argb);
            }
        }
    }
}
