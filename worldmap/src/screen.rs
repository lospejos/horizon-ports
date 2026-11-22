//! The waypoint screen (T11): browse waypoints with the arrows, make a new one, edit, delete.
use crate::waypoints::{step, Waypoint, Waypoints};
use mod_api::client::widgets::{Button, Outcome, TextField, WidgetId, Widgets};
use mod_api::Text;

/// Colour of a waypoint made on this screen until T13 lets the player pick one.
const DEFAULT_COLOR: u32 = 0xFF8800;

pub struct Screen {
    pub widgets: Widgets,
    name: WidgetId,
    xyz: [WidgetId; 3],
    prev: WidgetId,
    next: WidgetId,
    new: WidgetId,
    save: WidgetId,
    delete: WidgetId,
    close: WidgetId,
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
}

fn label(key: &str) -> Text {
    Text::translate(key, Vec::new())
}

impl Screen {
    /// Widgets centred on a screen `width` GUI pixels wide, a new waypoint at `here`.
    pub fn new(width: f32, here: [i32; 3], dimension: String) -> Self {
        let x = (width / 2.0 - 100.0).floor();
        let mut w = Widgets::new();
        let prev = w.add_button(Button::new([x, 30.0, 20.0, 20.0], Text::literal("<")));
        let next = w.add_button(Button::new([x + 180.0, 30.0, 20.0, 20.0], Text::literal(">")));
        let name = w.add_text_field(TextField::new([x, 70.0, 200.0, 20.0], 32));
        let xyz = [0.0, 68.0, 136.0].map(|dx| w.add_text_field(TextField::new([x + dx, 105.0, 64.0, 20.0], 9)));
        let new = w.add_button(Button::new([x, 135.0, 98.0, 20.0], label("worldmap.wp.new")));
        let save = w.add_button(Button::new([x + 102.0, 135.0, 98.0, 20.0], label("worldmap.wp.save")));
        let delete = w.add_button(Button::new([x, 160.0, 98.0, 20.0], label("worldmap.wp.delete")));
        let close = w.add_button(Button::new([x + 102.0, 160.0, 98.0, 20.0], label("worldmap.wp.close")));
        let mut s = Self { widgets: w, name, xyz, prev, next, new, save, delete, close, index: None, confirm: false, here, dimension };
        s.show(None, &Waypoints::default());
        s
    }

    /// Captions over the fields: (text, x offset from the left edge, y).
    pub const CAPTIONS: [(&'static str, f32, f32); 4] = [("", 0.0, 59.0), ("X", 0.0, 94.0), ("Y", 68.0, 94.0), ("Z", 136.0, 94.0)];

    /// The waypoint number for the header, 1-based; None for a new one.
    pub fn number(&self) -> Option<usize> {
        self.index.map(|i| i + 1)
    }

    /// Fills the fields from waypoint `i`, or blank for a new one at the player.
    fn show(&mut self, i: Option<usize>, wps: &Waypoints) {
        self.index = i;
        self.confirm = false;
        let (name, pos) = match i.and_then(|i| wps.list().get(i)) {
            Some(w) => (w.name.clone(), w.pos),
            None => (String::new(), self.here),
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
        self.set_delete_label();
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
            // Editing the text cancels a pending delete.
            if matches!(outcome, Outcome::Changed(_)) && self.confirm {
                self.confirm = false;
                self.set_delete_label();
            }
            return act;
        };
        if id == self.prev || id == self.next {
            let to = step(self.index, wps.list().len(), id == self.next);
            self.show(to, wps);
        } else if id == self.new {
            self.show(None, wps);
        } else if id == self.save {
            let Some(pos) = self.coords() else { return act };
            let typed = self.widgets.text_field(self.name).map(|f| f.value().trim().to_owned()).unwrap_or_default();
            let name = if typed.is_empty() { "Waypoint".to_owned() } else { typed };
            // An edit keeps what the screen does not show yet (dimension, colour, icon, visibility).
            let old = self.index.and_then(|i| wps.list().get(i)).cloned();
            let w = Waypoint {
                name,
                pos,
                dimension: old.as_ref().map_or_else(|| self.dimension.clone(), |o| o.dimension.clone()),
                color: old.as_ref().map_or(DEFAULT_COLOR, |o| o.color),
                icon: old.as_ref().map(|o| o.icon.clone()).unwrap_or_default(),
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
                let len = wps.list().len();
                self.show((len > 0).then(|| i.min(len - 1)), wps);
                act.save = true;
            }
        } else if id == self.close {
            act.close = true;
        }
        act
    }

    /// Save is greyed out while a coordinate is not a number.
    pub fn refresh(&mut self) {
        let ok = self.coords().is_some();
        if let Some(b) = self.widgets.button_mut(self.save) {
            b.active = ok;
        }
    }
}
