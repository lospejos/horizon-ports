//! Explored-terrain tiles as pure logic (ADR 0003): 64x64 columns, 4 bytes a column, file format, memory LRU, stale rule.
// shortcut: the screen that uses some of these comes in T5-T8, so some items are unused until then.
#![allow(dead_code)]

use std::collections::HashMap;

/// Columns along one side of a tile.
pub const SIDE: usize = 64;
const COLUMNS: usize = SIDE * SIDE;
const BYTES_PER_COLUMN: usize = 4;
/// First bytes of a tile file; the last byte is the format version.
const MAGIC: [u8; 4] = *b"WMT1";
/// Size of a whole tile file.
pub const FILE_LEN: usize = MAGIC.len() + COLUMNS * BYTES_PER_COLUMN;
/// A known tile is rescanned only when older than this many ticks (30 s, ADR 0003).
pub const RESCAN_TICKS: u64 = 600;

/// Columns the explorer scans per tick (spec A17).
pub const BUDGET: usize = 2048;
/// How far from the player's tile unknown tiles are looked for; (2 * RADIUS + 1)^2 stays under the store capacity.
pub const RADIUS: i32 = 7;
/// Tiles within this distance of the player's tile count as near, so they are rescanned when old.
pub const NEAR: i32 = 2;
/// Tiles kept in memory (16 KB each): the explorer's area plus a full-size map view (17 x 17 tiles at 1 block a pixel).
pub const CAPACITY: usize = 320;

/// Tile coordinates (tile column, tile row).
pub type Key = (i32, i32);

/// Tile that holds world column `(x, z)`.
pub fn tile_of(x: i32, z: i32) -> Key {
    (x.div_euclid(SIDE as i32), z.div_euclid(SIDE as i32))
}

/// Index of world column `(x, z)` inside its tile (row-major, north to south).
fn index(x: i32, z: i32) -> usize {
    z.rem_euclid(SIDE as i32) as usize * SIDE + x.rem_euclid(SIDE as i32) as usize
}

/// World column of the north-west corner of tile `key`.
pub fn origin(key: Key) -> (i32, i32) {
    (key.0 * SIDE as i32, key.1 * SIDE as i32)
}

/// File name of a tile inside its world and dimension folder.
pub fn file_name(key: Key) -> String {
    format!("t_{}_{}.bin", key.0, key.1)
}

/// Data folder of a world and dimension (ADR 0003); the name of a tile file goes after it.
pub fn folder(world: &str, dimension: &str) -> String {
    format!("tiles/{}/{}", crate::waypoints::slug(world), crate::waypoints::slug(dimension))
}

/// What is stored for one column. `shade` 0 means not scanned yet; 1..=4 are the vanilla brightness steps.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Column {
    /// Map colour id (0 is none, e.g. air or void).
    pub color: u8,
    pub shade: u8,
    /// Surface height.
    pub height: i16,
}

/// Vanilla brightness for a `shade` code; None for 0 (unscanned) and unknown codes.
pub fn brightness(shade: u8) -> Option<u32> {
    [135, 180, 220, 255].get(usize::from(shade).checked_sub(1)?).copied()
}

/// Shade code for a vanilla brightness (the nearest step at or below it).
pub fn shade_of(brightness: u32) -> u8 {
    match brightness {
        255.. => 4,
        220..=254 => 3,
        180..=219 => 2,
        _ => 1,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tile {
    data: Vec<u8>,
    /// Changed since it was last written to disk.
    pub dirty: bool,
    /// Tick of the last scan; None for a tile read from disk (its age is unknown, so it counts as old).
    pub scanned_at: Option<u64>,
}

impl Default for Tile {
    fn default() -> Self {
        Self { data: vec![0; COLUMNS * BYTES_PER_COLUMN], dirty: false, scanned_at: None }
    }
}

impl Tile {
    /// The stored column at world `(x, z)`; the caller picks the tile with `tile_of`.
    pub fn get(&self, x: i32, z: i32) -> Column {
        let b = &self.data[index(x, z) * BYTES_PER_COLUMN..][..BYTES_PER_COLUMN];
        Column { color: b[0], shade: b[1], height: i16::from_le_bytes([b[2], b[3]]) }
    }

    /// Stores a column and marks the tile dirty when the value changed.
    pub fn set(&mut self, x: i32, z: i32, c: Column) {
        if self.get(x, z) == c {
            return;
        }
        let h = c.height.to_le_bytes();
        self.data[index(x, z) * BYTES_PER_COLUMN..][..BYTES_PER_COLUMN].copy_from_slice(&[c.color, c.shade, h[0], h[1]]);
        self.dirty = true;
    }

    /// Stored surface level north of `(x, z)` inside the same tile; None on the tile's first row or when unscanned.
    pub fn north_height(&self, x: i32, z: i32) -> Option<i32> {
        (z.rem_euclid(SIDE as i32) > 0).then(|| self.get(x, z - 1)).filter(|c| c.shade != 0).map(|c| i32::from(c.height))
    }

    /// True when every column has been scanned.
    pub fn is_complete(&self) -> bool {
        self.data.chunks_exact(BYTES_PER_COLUMN).all(|c| c[1] != 0)
    }

    /// A tile is rescanned only while the player is near and it is older than `RESCAN_TICKS`.
    pub fn needs_rescan(&self, now: u64, near: bool) -> bool {
        near && self.scanned_at.is_none_or(|t| now.saturating_sub(t) >= RESCAN_TICKS)
    }

    /// The file bytes: magic, then the columns.
    pub fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(FILE_LEN);
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&self.data);
        out
    }

    /// Reads file bytes; None for a wrong magic or length (a broken file counts as no tile).
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let data = bytes.strip_prefix(&MAGIC[..])?;
        (data.len() == COLUMNS * BYTES_PER_COLUMN).then(|| Self { data: data.to_vec(), dirty: false, scanned_at: None })
    }
}

/// Tiles in memory; the least recently used one goes when `capacity` is exceeded.
pub struct Store {
    capacity: usize,
    clock: u64,
    tiles: HashMap<Key, (Tile, u64)>,
}

impl Store {
    pub fn new(capacity: usize) -> Self {
        Self { capacity: capacity.max(1), clock: 0, tiles: HashMap::new() }
    }

    pub fn len(&self) -> usize {
        self.tiles.len()
    }

    /// Looks a tile up without counting as a use.
    pub fn peek(&self, key: Key) -> Option<&Tile> {
        self.tiles.get(&key).map(|(t, _)| t)
    }

    /// The tile for editing or reading; counts as a use.
    pub fn get_mut(&mut self, key: Key) -> Option<&mut Tile> {
        self.clock += 1;
        let clock = self.clock;
        self.tiles.get_mut(&key).map(|(t, used)| {
            *used = clock;
            t
        })
    }

    /// Adds a tile (replacing one with the same key). Returns the dirty tiles it pushed out, which the caller must write.
    pub fn insert(&mut self, key: Key, tile: Tile) -> Vec<(Key, Tile)> {
        self.clock += 1;
        self.tiles.insert(key, (tile, self.clock));
        let mut dropped = Vec::new();
        while self.tiles.len() > self.capacity {
            let oldest = self.tiles.iter().filter(|(k, _)| **k != key).min_by_key(|(_, (_, used))| *used).map(|(k, _)| *k);
            let Some(k) = oldest else { break };
            if let Some((t, _)) = self.tiles.remove(&k) {
                if t.dirty {
                    dropped.push((k, t));
                }
            }
        }
        dropped
    }

    /// Keys of the dirty tiles.
    pub fn dirty(&self) -> Vec<Key> {
        self.tiles.iter().filter(|(_, (t, _))| t.dirty).map(|(k, _)| *k).collect()
    }
}

/// The tile to scan next for a player at world column `player`: an unknown tile, or a near one that is old,
/// nearest first. None when everything within `RADIUS` is known and fresh.
pub fn pick(store: &Store, player: (i32, i32), now: u64) -> Option<Key> {
    let (px, pz) = tile_of(player.0, player.1);
    let offsets = (-RADIUS..=RADIUS).flat_map(|dz| (-RADIUS..=RADIUS).map(move |dx| (dx, dz)));
    offsets
        .filter(|&(dx, dz)| {
            let near = dx.abs().max(dz.abs()) <= NEAR;
            store.peek((px + dx, pz + dz)).is_none_or(|t| t.needs_rescan(now, near))
        })
        .min_by_key(|&(dx, dz)| (dx * dx + dz * dz, dz, dx))
        .map(|(dx, dz)| (px + dx, pz + dz))
}

/// One tile being scanned, `BUDGET` columns at a time, row by row.
pub struct Job {
    pub key: Key,
    next: usize,
}

impl Job {
    pub fn new(key: Key) -> Self {
        Self { key, next: 0 }
    }

    /// The next column indices of this tile, at most `BUDGET`.
    pub fn batch(&mut self) -> std::ops::Range<usize> {
        let start = self.next;
        self.next = (start + BUDGET).min(COLUMNS);
        start..self.next
    }

    /// True once every column was handed out.
    pub fn done(&self) -> bool {
        self.next >= COLUMNS
    }

    /// World column of index `i`.
    pub fn column(&self, i: usize) -> (i32, i32) {
        let (ox, oz) = origin(self.key);
        (ox + (i % SIDE) as i32, oz + (i / SIDE) as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn col(n: u8) -> Column {
        Column { color: n, shade: 3, height: -64 + i16::from(n) * 100 }
    }

    #[test]
    fn world_to_tile_maths_handles_negatives() {
        assert_eq!(tile_of(0, 0), (0, 0));
        assert_eq!(tile_of(63, 63), (0, 0));
        assert_eq!(tile_of(64, -1), (1, -1));
        assert_eq!(tile_of(-64, -65), (-1, -2));
        assert_eq!(origin((-1, 2)), (-64, 128));
        assert_eq!(index(-1, -1), COLUMNS - 1);
        assert_eq!(index(64, 64), 0);
        assert_eq!(file_name((-1, 2)), "t_-1_2.bin");
    }

    #[test]
    fn folder_is_a_valid_distinct_path() {
        let a = folder("my world/..\\x", "minecraft:the_nether");
        assert!(a.split('/').all(|n| !n.is_empty() && !n.contains([':', '\\', '?'])), "{a}");
        assert_ne!(folder("w", "minecraft:overworld"), folder("w", "minecraft:the_end"));
        assert_eq!(folder("w", "d"), folder("w", "d"));
    }

    #[test]
    fn set_get_and_dirty() {
        let mut t = Tile::default();
        assert_eq!(t.get(5, 5), Column::default());
        t.set(5, 5, Column::default());
        assert!(!t.dirty);
        t.set(-59, 69, col(7));
        assert!(t.dirty);
        assert_eq!(t.get(5, 5), col(7));
        assert_eq!(t.get(6, 5), Column::default());
        assert!(!t.is_complete());
    }

    #[test]
    fn shades_map_to_brightness() {
        assert_eq!(brightness(0), None);
        assert_eq!(brightness(5), None);
        for b in [135, 180, 220, 255] {
            assert_eq!(brightness(shade_of(b)), Some(b));
        }
    }

    #[test]
    fn round_trip() {
        let mut t = Tile::default();
        for i in 0..COLUMNS as i32 {
            t.set(i % 64, i / 64, Column { color: (i % 62) as u8, shade: 1 + (i % 4) as u8, height: (i - 2000) as i16 });
        }
        assert!(t.is_complete());
        let bytes = t.serialize();
        assert_eq!(bytes.len(), FILE_LEN);
        let back = Tile::parse(&bytes).unwrap();
        assert_eq!(back.get(63, 63), t.get(63, 63));
        assert!(!back.dirty && back.scanned_at.is_none());
        assert_eq!(back.data, t.data);
    }

    #[test]
    fn broken_input_is_no_tile() {
        let good = Tile::default().serialize();
        assert!(Tile::parse(&[]).is_none());
        assert!(Tile::parse(&good[..good.len() - 1]).is_none());
        assert!(Tile::parse(&[good.clone(), vec![0]].concat()).is_none());
        let mut bad_magic = good;
        bad_magic[3] = b'9';
        assert!(Tile::parse(&bad_magic).is_none());
    }

    #[test]
    fn stale_only_when_near_and_old() {
        let mut t = Tile::default();
        assert!(t.needs_rescan(0, true), "unknown age counts as old");
        assert!(!t.needs_rescan(0, false));
        t.scanned_at = Some(1000);
        assert!(!t.needs_rescan(1000 + RESCAN_TICKS - 1, true));
        assert!(t.needs_rescan(1000 + RESCAN_TICKS, true));
        assert!(!t.needs_rescan(1000 + RESCAN_TICKS, false));
        assert!(!t.needs_rescan(500, true), "clock going back is not old");
    }

    #[test]
    fn lru_drops_the_least_recently_used_and_returns_dirty_ones() {
        let mut s = Store::new(2);
        assert!(s.insert((0, 0), Tile::default()).is_empty());
        let mut dirty = Tile::default();
        dirty.set(0, 0, col(1));
        assert!(s.insert((1, 0), dirty).is_empty());
        s.get_mut((0, 0)); // (1,0) is now the oldest
        let out = s.insert((2, 0), Tile::default());
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0, (1, 0));
        assert_eq!(s.len(), 2);
        assert!(s.peek((1, 0)).is_none() && s.peek((0, 0)).is_some() && s.peek((2, 0)).is_some());
        // a clean one goes without being returned
        assert!(s.insert((3, 0), Tile::default()).is_empty());
        assert_eq!(s.len(), 2);
        assert_eq!(s.dirty(), Vec::<Key>::new());
    }

    #[test]
    fn capacity_one_keeps_the_new_tile() {
        let mut s = Store::new(0);
        s.insert((0, 0), Tile::default());
        s.insert((1, 1), Tile::default());
        assert_eq!(s.len(), 1);
        assert!(s.peek((1, 1)).is_some());
    }

    fn scanned(at: u64) -> Tile {
        Tile { scanned_at: Some(at), ..Tile::default() }
    }

    #[test]
    fn pick_goes_nearest_first_and_skips_known() {
        let mut s = Store::new(CAPACITY);
        assert_eq!(pick(&s, (10, 10), 0), Some((0, 0)));
        s.insert((0, 0), scanned(0));
        assert_eq!(pick(&s, (10, 10), 1), Some((0, -1)), "tie on distance goes north first");
        s.insert((0, -1), scanned(0));
        assert_eq!(pick(&s, (10, 10), 1), Some((-1, 0)));
        // negative player coordinates pick their own tile
        assert_eq!(pick(&Store::new(1), (-1, -1), 0), Some((-1, -1)));
    }

    #[test]
    fn pick_rescans_only_near_old_tiles_and_stops_when_all_known() {
        let mut s = Store::new(CAPACITY);
        for dz in -RADIUS..=RADIUS {
            for dx in -RADIUS..=RADIUS {
                s.insert((dx, dz), scanned(100));
            }
        }
        assert_eq!(pick(&s, (0, 0), 100 + RESCAN_TICKS - 1), None);
        assert_eq!(pick(&s, (0, 0), 100 + RESCAN_TICKS), Some((0, 0)));
        // a player far west: the old tiles back east are not near, so only the player's own tile is rescanned
        s.get_mut((0, 0)).unwrap().scanned_at = Some(100 + RESCAN_TICKS);
        let far = pick(&s, (-64 * RADIUS, 0), 100 + RESCAN_TICKS);
        assert_eq!(far, Some((-RADIUS, 0)), "the player's own tile is near and old");
        assert!(RADIUS > NEAR);
    }

    #[test]
    fn job_hands_out_every_column_within_budget() {
        let mut j = Job::new((-1, 2));
        let mut seen = 0;
        while !j.done() {
            let r = j.batch();
            assert!(r.len() <= BUDGET);
            seen += r.len();
        }
        assert_eq!(seen, COLUMNS);
        assert_eq!(j.column(0), (-64, 128));
        assert_eq!(j.column(COLUMNS - 1), (-1, 191));
        assert_eq!(tile_of(j.column(65).0, j.column(65).1), (-1, 2));
    }

    #[test]
    fn north_height_stays_inside_the_tile() {
        let mut t = Tile::default();
        t.set(5, 9, Column { color: 1, shade: 3, height: 70 });
        assert_eq!(t.north_height(5, 10), Some(70));
        assert_eq!(t.north_height(5, 11), None, "unscanned north");
        assert_eq!(t.north_height(5, 64), None, "first row of a tile");
        assert_eq!(t.north_height(5, 0), None);
    }
}
