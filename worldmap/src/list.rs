//! The waypoint list screen (T14): pages of waypoints with visibility, edit and delete, and a sort button.
use crate::screen::Act;
use crate::waypoints::{order, Sort, Waypoints};
use mod_api::client::widgets::{Button, Outcome, WidgetId, Widgets};
use mod_api::Text;

/// Rows per page.
pub const PAGE: usize = 8;
/// Width of the widget column; the screen is centred on it.
pub const WIDTH: f32 = 260.0;
/// First row's top edge and the row pitch.
pub const TOP: f32 = 46.0;
pub const ROW: f32 = 22.0;
/// Pager and bottom button rows.
pub const PAGER_Y: f32 = TOP + PAGE as f32 * ROW + 4.0;
const BOTTOM_Y: f32 = PAGER_Y + 24.0;
/// Name button: its offset from the left edge and its width.
pub const NAME_DX: f32 = 30.0;
pub const NAME_W: f32 = 168.0;

#[derive(Clone, Copy)]
pub struct Row {
    /// Index in the waypoint list.
    pub index: usize,
    /// Rect of the name button, where the caller draws icon, name and distance.
    pub rect: [f32; 4],
    vis: WidgetId,
    name: WidgetId,
    tp: WidgetId,
    del: WidgetId,
}

struct Ids {
    rows: Vec<Row>,
    prev: WidgetId,
    next: WidgetId,
    new: WidgetId,
    sort: WidgetId,
    close: WidgetId,
}

pub struct List {
    pub widgets: Widgets,
    ids: Ids,
    x: f32,
    sort: Sort,
    page: usize,
    pages: usize,
    /// Waypoint whose delete was pressed once and waits for a second press.
    confirm: Option<usize>,
    /// Where the player stood when the screen opened, and its dimension: for distance and sorting.
    pub here: [i32; 3],
    pub dimension: String,
}

fn label(key: &str) -> Text {
    Text::translate(key, Vec::new())
}

impl List {
    /// Widgets centred on a screen `width` GUI pixels wide.
    pub fn new(width: f32, here: [i32; 3], dimension: String, wps: &Waypoints) -> Self {
        let x = (width / 2.0 - WIDTH / 2.0).floor();
        let mut view = View { x, sort: Sort::default(), page: 0, confirm: None, here, dimension: &dimension };
        let (widgets, ids, pages) = layout(&mut view, wps);
        Self { widgets, ids, x, sort: view.sort, page: view.page, pages, confirm: None, here, dimension }
    }

    /// Left edge of the widget column.
    pub fn left(&self) -> f32 {
        self.x
    }

    pub fn page(&self) -> (usize, usize) {
        (self.page + 1, self.pages)
    }

    pub fn rows(&self) -> &[Row] {
        &self.ids.rows
    }

    /// Lays the widgets out again for the current order, page and pending delete.
    fn rebuild(&mut self, wps: &Waypoints) {
        let mut view = View { x: self.x, sort: self.sort, page: self.page, confirm: self.confirm, here: self.here, dimension: &self.dimension };
        let (widgets, ids, pages) = layout(&mut view, wps);
        (self.page, self.pages) = (view.page, pages);
        (self.widgets, self.ids) = (widgets, ids);
    }

    pub fn act(&mut self, outcome: Outcome, wps: &mut Waypoints) -> Act {
        let mut act = Act::default();
        let Outcome::Pressed(id) = outcome else { return act };
        let pending = self.confirm.take();
        let row = self.ids.rows.iter().find(|r| r.vis == id || r.name == id || r.tp == id || r.del == id).copied();
        if id == self.ids.prev {
            self.page = (self.page + self.pages - 1) % self.pages;
        } else if id == self.ids.next {
            self.page = (self.page + 1) % self.pages;
        } else if id == self.ids.new {
            act.open = Some(None);
        } else if id == self.ids.sort {
            self.sort = self.sort.next();
            self.page = 0;
        } else if id == self.ids.close {
            act.close = true;
        } else if let Some(r) = row {
            if r.name == id {
                act.open = Some(Some(r.index));
            } else if r.tp == id {
                act.teleport = wps.list().get(r.index).map(|w| w.pos);
            } else if r.vis == id {
                if let Some(mut w) = wps.list().get(r.index).cloned() {
                    w.visible = !w.visible;
                    act.save = wps.edit(r.index, w);
                }
            } else if pending == Some(r.index) {
                act.save = wps.remove(r.index);
            } else {
                self.confirm = Some(r.index);
            }
        }
        self.rebuild(wps);
        act
    }
}

/// What the layout depends on.
struct View<'a> {
    x: f32,
    sort: Sort,
    /// Clamped to the last page by the layout.
    page: usize,
    confirm: Option<usize>,
    here: [i32; 3],
    dimension: &'a str,
}

/// Widgets for the current page; returns them with their ids and the page count.
fn layout(v: &mut View, wps: &Waypoints) -> (Widgets, Ids, usize) {
    let list = wps.list();
    let pages = list.len().div_ceil(PAGE).max(1);
    v.page = v.page.min(pages - 1);
    let (x, mut w) = (v.x, Widgets::new());
    let rows = order(list, v.sort, v.here, v.dimension)
        .into_iter()
        .skip(v.page * PAGE)
        .take(PAGE)
        .enumerate()
        .map(|(n, index)| {
            let y = TOP + n as f32 * ROW;
            let on = list[index].visible;
            let vis = w.add_button(Button::new([x, y, 28.0, 20.0], label(if on { "worldmap.wp.on" } else { "worldmap.wp.off" })));
            let rect = [x + NAME_DX, y, NAME_W, 20.0];
            let name = w.add_button(Button::new(rect, Text::literal("")));
            let same = list[index].dimension == v.dimension;
            let mut tp = Button::new([x + 202.0, y, 28.0, 20.0], Text::literal("TP"));
            tp.active = same;
            let tp = w.add_button(tp);
            let del = w.add_button(Button::new([x + 232.0, y, 28.0, 20.0], Text::literal(if v.confirm == Some(index) { "?" } else { "X" })));
            Row { index, rect, vis, name, tp, del }
        })
        .collect();
    let prev = w.add_button(Button::new([x, PAGER_Y, 20.0, 20.0], Text::literal("<")));
    let next = w.add_button(Button::new([x + WIDTH - 20.0, PAGER_Y, 20.0, 20.0], Text::literal(">")));
    let new = w.add_button(Button::new([x, BOTTOM_Y, 84.0, 20.0], label("worldmap.wp.new")));
    let sort_key = match v.sort {
        Sort::Created => "worldmap.wp.sort.created",
        Sort::Name => "worldmap.wp.sort.name",
        Sort::Distance => "worldmap.wp.sort.distance",
    };
    let sort = w.add_button(Button::new([x + 88.0, BOTTOM_Y, 84.0, 20.0], label(sort_key)));
    let close = w.add_button(Button::new([x + 176.0, BOTTOM_Y, 84.0, 20.0], label("worldmap.wp.close")));
    (w, Ids { rows, prev, next, new, sort, close }, pages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::waypoints::Waypoint;

    fn many(n: usize) -> Waypoints {
        let mut wps = Waypoints::default();
        for i in 0..n {
            wps.add(Waypoint { name: format!("Waypoint {i}"), pos: [i as i32 * 10, 64, 0], dimension: "minecraft:overworld".into(), color: 0xFF8800, icon: String::new(), visible: true });
        }
        wps
    }

    fn list(wps: &Waypoints) -> List {
        List::new(427.0, [0, 64, 0], "minecraft:overworld".into(), wps)
    }

    /// There is no scrollbar: the list pages at `PAGE` rows, so 20-50 waypoints give 3-7 pages.
    #[test]
    fn twenty_to_fifty_waypoints_page_and_every_row_fits_the_panel() {
        for (n, pages) in [(20, 3), (50, 7)] {
            let wps = many(n);
            let mut l = list(&wps);
            assert_eq!(l.page(), (1, pages));
            let mut seen = Vec::new();
            for p in 0..pages {
                assert_eq!(l.page(), (p + 1, pages));
                let rows = l.rows();
                assert_eq!(rows.len(), if p + 1 < pages { PAGE } else { n - PAGE * (pages - 1) });
                for r in rows {
                    assert!(r.rect[1] >= TOP && r.rect[1] + r.rect[3] <= PAGER_Y, "row inside the panel, above the pager");
                    seen.push(r.index);
                }
                let next = l.ids.next;
                l.act(Outcome::Pressed(next), &mut many(n));
            }
            seen.sort_unstable();
            assert_eq!(seen, (0..n).collect::<Vec<_>>(), "every waypoint is on exactly one page");
            assert_eq!(l.page(), (1, pages), "next from the last page wraps to the first");
        }
    }

    #[test]
    fn panel_fits_a_small_gui_and_prev_wraps_to_the_last_page() {
        assert!(BOTTOM_Y + 20.0 <= 360.0, "bottom buttons stay on a 640x360 GUI");
        let mut wps = many(50);
        let mut l = list(&wps);
        let prev = l.ids.prev;
        l.act(Outcome::Pressed(prev), &mut wps);
        assert_eq!(l.page(), (7, 7));
        assert_eq!(l.rows().len(), 2);
    }

    #[test]
    fn deleting_the_last_row_of_the_last_page_steps_back_a_page() {
        let mut wps = many(9);
        let mut l = list(&wps);
        let next = l.ids.next;
        l.act(Outcome::Pressed(next), &mut wps);
        assert_eq!(l.page(), (2, 2));
        for _ in 0..2 {
            let del = l.rows()[0].del;
            l.act(Outcome::Pressed(del), &mut wps);
        }
        assert_eq!(wps.list().len(), 8);
        assert_eq!(l.page(), (1, 1));
        assert_eq!(l.rows().len(), 8);
    }
}
