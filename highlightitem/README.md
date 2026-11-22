# HighLightItem for Horizon

Highlights inventory items similar to the one hovered over.

A port of [HighLightItem](https://github.com/elmital/HighLightItem) 3.1.0 by elmital to [Horizon](https://github.com/horizon-rust/horizon) 26.3, under the original's licence (GPL-3.0-only, in `LICENSE`; see `NOTICE`).

A WebAssembly mod (`mod.wasm`, no mixins), built with `cargo horizon-mod build --wasm` against Horizon's exact-build SDK `2b77287de099c9316278ba7ac4d83590fdb7ef30`; the `highlightitem-wasm` workflow builds `highlightitem.hmod` and attaches it to a release for a tag `highlightitem-wasm-v*`.

Options > Mods > HighLightItem: on/off, comparator (material; material and amount; material and NBT; material, NBT and amount; name; name and amount; namespace), where it highlights (everywhere, not in creative, player inventory, player inventory without creative, containers only), how the hovered slot is drawn (the game's own, coloured, coloured when not empty), colour and opacity. Controls: keys to toggle and to cycle the comparator, unbound by default.

Not ported: the colour-picker screen, the `/highlightitem` commands, ModMenu, notifications, the key that colours the hovered slot. Needs the player's inventory screen not covered by the recipe book (nothing is drawn then).
