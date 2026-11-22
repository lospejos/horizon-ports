//! Terrain sampling as pure logic: which columns to read this tick and how a column becomes a pixel.

/// Vanilla `MapColor` base colours by id (0 is NONE).
const MAP_COLORS: [u32; 62] = [
    0x000000, 0x7FB238, 0xF7E9A3, 0xC7C7C7, 0xFF0000, 0xA0A0FF, 0xA7A7A7, 0x007C00, 0xFFFFFF, 0xA4A8B8,
    0x976D4D, 0x707070, 0x4040FF, 0x8F7748, 0xFFFCF5, 0xD87F33, 0xB24CD8, 0x6699D8, 0xE5E533, 0x7FCC19,
    0xF27FA5, 0x4C4C4C, 0x999999, 0x4C7F99, 0x7F3FB2, 0x334CB2, 0x664C33, 0x667F33, 0x993333, 0x191919,
    0xFAEE4D, 0x5CDBD5, 0x4A80FF, 0x00D93A, 0x815631, 0x700200, 0xD1B1A1, 0x9F5224, 0x95576C, 0x706C8A,
    0xBA8524, 0x677535, 0xA04D4E, 0x392923, 0x876B62, 0x575C5C, 0x7A4958, 0x4C3E5C, 0x4C3223, 0x4C522A,
    0x8E3C2E, 0x251610, 0xBD3031, 0x943F61, 0x5C191D, 0x167E86, 0x3A8E8C, 0x562C3E, 0x14B485, 0x646464,
    0xD8AF93, 0x7FA796,
];

/// Opaque RGBA of a map colour id at `brightness` (vanilla uses 135, 180, 220, 255); NONE and unknown ids stay transparent.
pub fn map_rgba(id: u8, brightness: u32) -> [u8; 4] {
    match MAP_COLORS.get(id as usize) {
        Some(&rgb) if id != 0 => {
            let ch = |shift: u32| (((rgb >> shift) & 0xFF) * brightness / 255) as u8;
            [ch(16), ch(8), ch(0), 255]
        }
        _ => [0; 4],
    }
}

/// Walks a `size` x `size` grid row by row, `budget` columns per call, wrapping at the end.
pub struct Scan {
    pub size: u32,
    next: u32,
}

impl Scan {
    pub fn new(size: u32) -> Self {
        Self { size, next: 0 }
    }

    /// The next grid indices (row-major), at most `budget` and at most up to the end of the grid.
    pub fn batch(&mut self, budget: u32) -> std::ops::Range<u32> {
        let total = self.size * self.size;
        if self.next >= total {
            self.next = 0;
        }
        let start = self.next;
        self.next = (start + budget).min(total);
        start..self.next
    }

    /// World column of grid index `i` for a map centred on `(cx, cz)`.
    pub fn column(&self, i: u32, cx: i32, cz: i32) -> (i32, i32) {
        let half = (self.size / 2) as i32;
        (cx - half + (i % self.size) as i32, cz - half + (i / self.size) as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors() {
        assert_eq!(map_rgba(1, 255), [0x7F, 0xB2, 0x38, 255]);
        assert_eq!(map_rgba(8, 220)[0], 220);
        assert_eq!(map_rgba(0, 220), [0; 4]);
        assert_eq!(map_rgba(200, 220), [0; 4]);
    }

    #[test]
    fn batches_cover_grid_in_budget_and_wrap() {
        let mut s = Scan::new(4);
        assert_eq!(s.batch(10), 0..10);
        assert_eq!(s.batch(10), 10..16);
        assert_eq!(s.batch(10), 0..10);
    }

    #[test]
    fn columns_centre_on_player() {
        let s = Scan::new(4);
        assert_eq!(s.column(0, 100, 200), (98, 198));
        assert_eq!(s.column(15, 100, 200), (101, 201));
    }
}
