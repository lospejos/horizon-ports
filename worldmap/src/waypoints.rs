//! Waypoints as pure logic: the list, its text file format and the per-world file name.
// shortcut: the UI that calls add/edit/remove comes in T11-T15, so some items are unused until then.
#![allow(dead_code)]

/// First line of a waypoint file; the number is the format version.
const HEADER: &str = "worldmap-waypoints 1";

#[derive(Clone, Debug, PartialEq)]
pub struct Waypoint {
    pub name: String,
    pub pos: [i32; 3],
    /// Dimension id, `minecraft:overworld`.
    pub dimension: String,
    /// 0xRRGGBB.
    pub color: u32,
    /// Icon id from the built-in set (T13); empty is the default dot.
    pub icon: String,
    pub visible: bool,
}

/// The waypoints of one world, in creation order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Waypoints(Vec<Waypoint>);

impl Waypoints {
    pub fn list(&self) -> &[Waypoint] {
        &self.0
    }

    /// Adds `w` and returns its index.
    pub fn add(&mut self, w: Waypoint) -> usize {
        self.0.push(w);
        self.0.len() - 1
    }

    /// Replaces waypoint `i`; false when there is none.
    pub fn edit(&mut self, i: usize, w: Waypoint) -> bool {
        self.0.get_mut(i).map(|slot| *slot = w).is_some()
    }

    /// Removes waypoint `i`; false when there is none.
    pub fn remove(&mut self, i: usize) -> bool {
        if i < self.0.len() {
            self.0.remove(i);
            true
        } else {
            false
        }
    }

    /// The file text: a header, then one tab-separated line per waypoint.
    pub fn serialize(&self) -> String {
        let mut out = format!("{HEADER}\n");
        for w in &self.0 {
            out += &format!(
                "{}\t{}\t{}\t{}\t{}\t{:06X}\t{}\t{}\n",
                escape(&w.name),
                w.pos[0],
                w.pos[1],
                w.pos[2],
                escape(&w.dimension),
                w.color,
                escape(&w.icon),
                u8::from(w.visible)
            );
        }
        out
    }

    /// Reads file text; None when the header is wrong. A broken line is skipped, not fatal.
    pub fn parse(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        if lines.next()? != HEADER {
            return None;
        }
        Some(Self(lines.filter_map(parse_line).collect()))
    }
}

fn parse_line(line: &str) -> Option<Waypoint> {
    let f: Vec<&str> = line.split('\t').collect();
    let [name, x, y, z, dimension, color, icon, visible] = f[..] else { return None };
    Some(Waypoint {
        name: unescape(name),
        pos: [x.parse().ok()?, y.parse().ok()?, z.parse().ok()?],
        dimension: unescape(dimension),
        color: u32::from_str_radix(color, 16).ok().filter(|c| *c <= 0xFFFFFF)?,
        icon: unescape(icon),
        visible: visible == "1",
    })
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\t', "\\t").replace('\n', "\\n").replace('\r', "\\r")
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

/// Palette of the colour picker, two rows of eight.
pub const PALETTE: [u32; 16] = [
    0xFF0000, 0xFF8800, 0xFFDD00, 0x55DD00, 0x00AA00, 0x00DDDD, 0x3388FF, 0x0000CC, 0xAA00FF, 0xFF55FF, 0xFFFFFF, 0xBBBBBB, 0x777777, 0x333333,
    0x885522, 0xFF9999,
];

/// Reads `RRGGBB` (an optional leading `#`); None when it is not six hex digits.
pub fn parse_color(s: &str) -> Option<u32> {
    let s = s.trim().trim_start_matches('#');
    if s.len() == 6 && s.chars().all(|c| c.is_ascii_hexdigit()) {
        u32::from_str_radix(s, 16).ok()
    } else {
        None
    }
}

/// How the list screen orders waypoints.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Sort {
    #[default]
    Created,
    Name,
    Distance,
}

impl Sort {
    pub fn next(self) -> Self {
        match self {
            Sort::Created => Sort::Name,
            Sort::Name => Sort::Distance,
            Sort::Distance => Sort::Created,
        }
    }
}

/// Squared distance between two block positions.
pub fn dist2(a: [i32; 3], b: [i32; 3]) -> i64 {
    (0..3).map(|i| (i64::from(a[i]) - i64::from(b[i])).pow(2)).sum()
}

/// Indices of `list` in display order. Distance puts other dimensions last; ties keep creation order.
pub fn order(list: &[Waypoint], sort: Sort, here: [i32; 3], dimension: &str) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..list.len()).collect();
    match sort {
        Sort::Created => {}
        Sort::Name => idx.sort_by_key(|&i| list[i].name.to_lowercase()),
        Sort::Distance => idx.sort_by_key(|&i| (list[i].dimension != dimension, dist2(list[i].pos, here))),
    }
    idx
}

/// A file name for a world key (a save folder or a server address): safe characters stay, the rest become `_`.
/// The key's FNV-1a hash is appended so two keys that differ only in unsafe characters keep separate files.
pub fn file_name(key: &str) -> String {
    let safe: String = key.chars().take(40).map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '_') { c } else { '_' }).collect();
    let hash = key.bytes().fold(0xcbf29ce484222325_u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100000001b3));
    format!("waypoints-{safe}-{hash:016x}.txt")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wp(name: &str) -> Waypoint {
        Waypoint { name: name.into(), pos: [10, -64, -300], dimension: "minecraft:overworld".into(), color: 0xFF8800, icon: "star".into(), visible: true }
    }

    #[test]
    fn parse_color_reads_six_hex_digits() {
        assert_eq!(parse_color("ff8800"), Some(0xFF8800));
        assert_eq!(parse_color(" #00AaFF "), Some(0x00AAFF));
        assert_eq!(parse_color("ff88"), None);
        assert_eq!(parse_color("gg0000"), None);
        assert_eq!(parse_color("+f8800"), None);
    }

    #[test]
    fn add_edit_remove() {
        let mut w = Waypoints::default();
        assert_eq!(w.add(wp("home")), 0);
        assert_eq!(w.add(wp("mine")), 1);
        assert!(w.edit(0, wp("base")));
        assert!(!w.edit(5, wp("x")));
        assert_eq!(w.list()[0].name, "base");
        assert!(w.remove(0));
        assert!(!w.remove(1));
        assert_eq!(w.list().len(), 1);
        assert_eq!(w.list()[0].name, "mine");
    }

    #[test]
    fn round_trip_with_awkward_text() {
        let mut w = Waypoints::default();
        w.add(wp("tab\there\nnew \\ line"));
        let mut hidden = wp("тест");
        hidden.visible = false;
        hidden.icon = String::new();
        hidden.color = 0;
        w.add(hidden);
        assert_eq!(Waypoints::parse(&w.serialize()), Some(w));
    }

    #[test]
    fn parse_rejects_bad_header_and_skips_bad_lines() {
        assert_eq!(Waypoints::parse("nope\n"), None);
        assert_eq!(Waypoints::parse(""), None);
        let good = Waypoints(vec![wp("a")]).serialize();
        let text = format!("{good}garbage\na\t1\t2\tx\td\tFFFFFF\ti\t1\na\t1\t2\t3\td\t1000000\ti\t1\n");
        assert_eq!(Waypoints::parse(&text).unwrap().list().len(), 1);
    }

    #[test]
    fn order_sorts_by_name_and_distance() {
        let at = |name: &str, x: i32, dim: &str| Waypoint { name: name.into(), pos: [x, 0, 0], dimension: dim.into(), ..wp("") };
        let list = [at("b", 100, "o"), at("A", 5, "n"), at("c", 10, "o"), at("d", 50, "o")];
        assert_eq!(order(&list, Sort::Created, [0; 3], "o"), [0, 1, 2, 3]);
        assert_eq!(order(&list, Sort::Name, [0; 3], "o"), [1, 0, 2, 3]);
        assert_eq!(order(&list, Sort::Distance, [0; 3], "o"), [2, 3, 0, 1]);
        assert_eq!(Sort::Distance.next(), Sort::Created);
    }

    #[test]
    fn file_name_is_safe_and_distinct() {
        let a = file_name("my world/..\\x");
        assert!(ConfigNameOk::ok(&a), "{a}");
        assert_ne!(file_name("a:b"), file_name("a?b"));
        assert_eq!(file_name("same"), file_name("same"));
    }

    /// The host's name rule: no path separators or reserved characters.
    struct ConfigNameOk;
    impl ConfigNameOk {
        fn ok(n: &str) -> bool {
            !n.is_empty() && !n.ends_with(['.', ' ']) && !n.chars().any(|c| c.is_control() || matches!(c, '/' | '\\' | ':' | '<' | '>' | '"' | '|' | '?' | '*'))
        }
    }
}
