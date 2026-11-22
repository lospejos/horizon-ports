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
