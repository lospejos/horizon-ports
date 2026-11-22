//! Built-in waypoint icons (T13): 7x7 bitmaps, drawn as filled rectangles so no image assets are needed.

/// Icon side in pixels.
pub const SIZE: i32 = 7;

/// (id, bitmap rows); `#` is a filled pixel. The first icon is the default.
const ICONS: [(&str, [&str; 7]); 7] = [
    ("square", [".......", ".#####.", ".#####.", ".#####.", ".#####.", ".#####.", "......."]),
    ("diamond", ["...#...", "..###..", ".#####.", "#######", ".#####.", "..###..", "...#..."]),
    ("cross", ["..###..", "..###..", "#######", "#######", "#######", "..###..", "..###.."]),
    ("triangle", ["...#...", "...#...", "..###..", "..###..", ".#####.", ".#####.", "#######"]),
    ("heart", [".##.##.", "#######", "#######", "#######", ".#####.", "..###..", "...#..."]),
    ("star", ["...#...", "...#...", "#######", ".#####.", "..###..", ".##.##.", ".#...#."]),
    ("circle", ["..###..", ".#####.", "#######", "#######", "#######", ".#####.", "..###.."]),
];

/// Ids of the built-in icons, in picker order.
pub fn ids() -> impl Iterator<Item = &'static str> {
    ICONS.iter().map(|(id, _)| *id)
}

/// The icon an id names; an empty or unknown id gives the default (the first icon).
pub fn canonical(id: &str) -> &'static str {
    ICONS.iter().find(|(i, _)| *i == id).map_or(ICONS[0].0, |(i, _)| *i)
}

/// Filled runs of an icon as `[x, y, width, 1]` in icon pixels.
pub fn rects(id: &str) -> Vec<[i32; 4]> {
    let rows = ICONS.iter().find(|(i, _)| *i == id).map_or(&ICONS[0].1, |(_, r)| r);
    let mut out = Vec::new();
    for (y, row) in rows.iter().enumerate() {
        let mut x = 0;
        for run in row.split('.') {
            if !run.is_empty() {
                out.push([x, y as i32, run.len() as i32, 1]);
            }
            x += run.len() as i32 + 1;
        }
    }
    out
}

/// Draws icon `id` centred on `at` with a black outline; `fill(rect, argb)` paints one rectangle.
pub fn draw(fill: &mut dyn FnMut([f32; 4], u32), id: &str, at: [f32; 2], color: u32) {
    let (x0, y0) = ((at[0] - SIZE as f32 / 2.0).round(), (at[1] - SIZE as f32 / 2.0).round());
    let runs = rects(id);
    for (dx, dy, argb) in [(-1, 0, 0xFF000000), (1, 0, 0xFF000000), (0, -1, 0xFF000000), (0, 1, 0xFF000000), (0, 0, 0xFF000000 | color)] {
        for [x, y, w, h] in &runs {
            fill([x0 + (x + dx) as f32, y0 + (y + dy) as f32, *w as f32, *h as f32], argb);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitmaps_are_7x7_and_ids_unique() {
        for (n, (id, rows)) in ICONS.iter().enumerate() {
            assert!(rows.iter().all(|r| r.len() == 7 && r.chars().all(|c| c == '#' || c == '.')), "{id}");
            assert!(ICONS[..n].iter().all(|(i, _)| i != id), "{id} twice");
        }
    }

    #[test]
    fn rects_cover_exactly_the_filled_pixels() {
        for (id, rows) in ICONS {
            let filled: i32 = rows.iter().map(|r| r.matches('#').count() as i32).sum();
            assert_eq!(rects(id).iter().map(|r| r[2]).sum::<i32>(), filled, "{id}");
        }
        assert_eq!(rects("heart")[0], [1, 0, 2, 1]);
    }

    #[test]
    fn unknown_id_is_the_default() {
        assert_eq!(canonical(""), "square");
        assert_eq!(canonical("nope"), "square");
        assert_eq!(canonical("star"), "star");
        assert_eq!(rects("nope"), rects("square"));
    }
}
