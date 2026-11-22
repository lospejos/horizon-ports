//! The waypoint edit screen (T11): edit one waypoint or make a new one; the list screen (T14) opens it.
use crate::icons;
use crate::waypoints::{parse_color, Waypoint, Waypoints, PALETTE};
use mod_api::client::widgets::{Button, Outcome, TextField, WidgetId, Widgets};
use mod_api::Text;

/// Colour of a new waypoint.
const DEFAULT_COLOR: u32 = 0xFF8800;

pub struct Screen {
    pub widgets: Widgets,
    name: WidgetId,
    xyz: [WidgetId; 3],
    /// RRGGBB field.
    hex: WidgetId,
    /// Palette buttons: id, rect, colour.
    pub swatches: Vec<(WidgetId, [f32; 4], u32)>,
    /// Icon buttons: id, rect, icon id.
    pub icon_buttons: Vec<(WidgetId, [f32; 4], &'static str)>,
    /// Colour and icon shown on the screen; Save writes them.
    pub color: u32,
    pub icon: &'static str,
    save: WidgetId,
    delete: WidgetId,
    back: WidgetId,
    tp: WidgetId,
    /// Dimension of the waypoint shown (the current one for a new waypoint).
    wp_dim: String,
    /// The waypoint being edited; None is a new one.
    index: Option<usize>,
    /// Delete was pressed once and waits for a second press.
    confirm: bool,
    /// Where the player stands and its dimension, for a new waypoint.
    here: [i32; 3],
    dimension: String,
}

/// What the screen asks of the mod after an input.
#[derive(Default)]
pub struct Act {
    /// The list changed: write the file.
    pub save: bool,
    pub close: bool,
    /// Go from the list to the edit screen: Some(None) is a new waypoint.
    pub open: Option<Option<usize>>,
    /// Go from the edit screen back to the list.
    pub back: bool,
    /// Teleport the player here (same dimension; the mod sends the command).
    pub teleport: Option<[i32; 3]>,
}

fn label(key: &str) -> Text {
    Text::translate(key, Vec::new())
}

impl Screen {
    /// Widgets centred on a screen `width` GUI pixels wide, editing waypoint `index` (None: a new one at `here`).
    pub fn new(width: f32, here: [i32; 3], dimension: String, index: Option<usize>, wps: &Waypoints) -> Self {
        let x = (width / 2.0 - 100.0).floor();
        let mut w = Widgets::new();
        let name = w.add_text_field(TextField::new([x, 70.0, 200.0, 20.0], 32));
        let xyz = [0.0, 68.0, 136.0].map(|dx| w.add_text_field(TextField::new([x + dx, 105.0, 64.0, 20.0], 9)));
        let hex = w.add_text_field(TextField::new([x, 140.0, 64.0, 20.0], 6));
        let swatches = PALETTE
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let rect = [x + (i % 8) as f32 * 25.0, 165.0 + (i / 8) as f32 * 23.0, 21.0, 20.0];
                (w.add_button(Button::new(rect, Text::literal(""))), rect, *c)
            })
            .collect();
        let icon_buttons = icons::ids()
            .enumerate()
            .map(|(i, id)| {
                let rect = [x + i as f32 * 28.0, 218.0, 24.0, 24.0];
                (w.add_button(Button::new(rect, Text::literal(""))), rect, id)
            })
            .collect();
        let save = w.add_button(Button::new([x, 248.0, 98.0, 20.0], label("worldmap.wp.save")));
        let delete = w.add_button(Button::new([x + 102.0, 248.0, 98.0, 20.0], label("worldmap.wp.delete")));
        let tp = w.add_button(Button::new([x, 273.0, 98.0, 20.0], label("worldmap.wp.tp")));
        let back = w.add_button(Button::new([x + 102.0, 273.0, 98.0, 20.0], label("worldmap.wp.back")));
        let mut s = Self { widgets: w, name, xyz, hex, swatches, icon_buttons, color: DEFAULT_COLOR, icon: icons::canonical(""), save, delete, back, tp, wp_dim: String::new(), index: None, confirm: false, here, dimension };
        s.show(index, wps);
        s
    }

    /// Captions over the fields: (text, x offset from the left edge, y).
    pub const CAPTIONS: [(&'static str, f32, f32); 6] =
        [("", 0.0, 59.0), ("X", 0.0, 94.0), ("Y", 68.0, 94.0), ("Z", 136.0, 94.0), ("worldmap.wp.colour", 0.0, 129.0), ("worldmap.wp.icon", 0.0, 207.0)];

    /// True while the screen shows a new waypoint, not a saved one.
    pub fn is_new(&self) -> bool {
        self.index.is_none()
    }

    /// Fills the fields from waypoint `i`, or blank for a new one at the player.
    fn show(&mut self, i: Option<usize>, wps: &Waypoints) {
        self.index = i;
        self.confirm = false;
        self.wp_dim = self.dimension.clone();
        let (name, pos) = match i.and_then(|i| wps.list().get(i)) {
            Some(w) => {
                self.wp_dim = w.dimension.clone();
                self.color = w.color;
                self.icon = icons::canonical(&w.icon);
                (w.name.clone(), w.pos)
            }
            None => {
                self.color = DEFAULT_COLOR;
                self.icon = icons::canonical("");
                (String::new(), self.here)
            }
        };
        let set = |w: &mut Widgets, id, v: &str| {
            if let Some(f) = w.text_field_mut(id) {
                f.set_value(v);
            }
        };
        set(&mut self.widgets, self.name, &name);
        for (id, v) in self.xyz.into_iter().zip(pos) {
            set(&mut self.widgets, id, &v.to_string());
        }
        self.set_hex();
        self.set_delete_label();
    }

    fn set_hex(&mut self) {
        let v = format!("{:06X}", self.color);
        if let Some(f) = self.widgets.text_field_mut(self.hex) {
            f.set_value(&v);
        }
    }

    /// Writes the shown colour and icon into the waypoint being edited, so the map follows at once.
    /// A new waypoint has none yet; Save adds it.
    fn apply_style(&self, wps: &mut Waypoints) -> bool {
        let Some(mut w) = self.index.and_then(|i| wps.list().get(i)).cloned() else { return false };
        (w.color, w.icon) = (self.color, self.icon.to_owned());
        self.index.is_some_and(|i| wps.edit(i, w))
    }

    fn set_delete_label(&mut self) {
        let key = if self.confirm { "worldmap.wp.sure" } else { "worldmap.wp.delete" };
        let editing = self.index.is_some();
        if let Some(b) = self.widgets.button_mut(self.delete) {
            b.label = label(key);
            b.active = editing;
        }
    }

    /// The coordinates typed, None while one is not a whole number.
    fn coords(&self) -> Option<[i32; 3]> {
        let mut out = [0; 3];
        for (o, id) in out.iter_mut().zip(self.xyz) {
            *o = self.widgets.text_field(id)?.value().trim().parse().ok()?;
        }
        Some(out)
    }

    /// Acts on a widget's answer; `wps` is the world's list.
    pub fn act(&mut self, outcome: Outcome, wps: &mut Waypoints) -> Act {
        let mut act = Act::default();
        let Outcome::Pressed(id) = outcome else {
            if let Outcome::Changed(id) = outcome {
                // Editing the text cancels a pending delete.
                if self.confirm {
                    self.confirm = false;
                    self.set_delete_label();
                }
                if id == self.hex {
                    let typed = self.widgets.text_field(self.hex).and_then(|f| parse_color(f.value()));
                    if let Some(c) = typed {
                        self.color = c;
                        act.save = self.apply_style(wps);
                    }
                }
            }
            return act;
        };
        if let Some(&(_, _, c)) = self.swatches.iter().find(|b| b.0 == id) {
            self.color = c;
            self.set_hex();
            act.save = self.apply_style(wps);
        } else if let Some(&(_, _, i)) = self.icon_buttons.iter().find(|b| b.0 == id) {
            self.icon = i;
            act.save = self.apply_style(wps);
        } else if id == self.save {
            let Some(pos) = self.coords() else { return act };
            let typed = self.widgets.text_field(self.name).map(|f| f.value().trim().to_owned()).unwrap_or_default();
            let name = if typed.is_empty() { "Waypoint".to_owned() } else { typed };
            // An edit keeps what the screen does not show (dimension, visibility).
            let old = self.index.and_then(|i| wps.list().get(i)).cloned();
            let w = Waypoint {
                name,
                pos,
                dimension: old.as_ref().map_or_else(|| self.dimension.clone(), |o| o.dimension.clone()),
                color: self.color,
                icon: self.icon.to_owned(),
                visible: old.map_or(true, |o| o.visible),
            };
            let i = match self.index {
                Some(i) if wps.edit(i, w.clone()) => i,
                _ => wps.add(w),
            };
            self.index = Some(i);
            self.set_delete_label();
            act.save = true;
        } else if id == self.delete {
            let Some(i) = self.index else { return act };
            if !self.confirm {
                self.confirm = true;
                self.set_delete_label();
            } else {
                wps.remove(i);
                act.save = true;
                act.back = true;
            }
        } else if id == self.tp {
            if self.wp_dim == self.dimension {
                act.teleport = self.coords();
            }
        } else if id == self.back {
            act.back = true;
        }
        act
    }

    /// Save is greyed out while a coordinate is not a number.
    pub fn refresh(&mut self) {
        let ok = self.coords().is_some();
        let same = self.wp_dim == self.dimension;
        if let Some(b) = self.widgets.button_mut(self.save) {
            b.active = ok;
        }
        if let Some(b) = self.widgets.button_mut(self.tp) {
            b.active = ok && same;
        }
    }
}
