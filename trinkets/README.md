# Trinkets for Horizon

Extra equipment slots for trinkets, in a menu on G.

A port of [Trinkets](https://github.com/emilyploszaj/trinkets) 3.7.2 by Emi, C4 to [Horizon](https://github.com/horizon-rust/horizon) 26.3, under the original's licence (MIT, in `LICENSE`; see `NOTICE`).

This folder is the package source: one native mod. Its code uses Horizon's mod API and engine mixins (listed in `horizon-mod.toml`). Release [`trinkets-v0.4.2`](https://github.com/lavoide/horizon-ports/releases/tag/trinkets-v0.4.2) holds `trinkets.hmod`, built from this folder with Horizon's exact-build SDK `dd67b76ff23e28c6d8e21859df600d31572f81a0` (Horizon Mixins `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`). Earlier versions, if any, are in this folder's history.

## Installing and updating

Download the release into the game's mods folder (the `horizon-mods` folder of the instance or of the launcher's data directory; `--clobber` replaces the old file), then restart the game:

```powershell
gh release download trinkets-wasm-v0.5.2 -R lospejos/horizon-ports -p "*.hmod" -D "C:\Users\User\AppData\Roaming\horizon\horizon-mods" --clobber
```

For a newer version, change only the tag. `gh release list -R lospejos/horizon-ports` lists the tags. Or download `trinkets.hmod` from the release page by hand.

To follow the latest CI build instead (branch builds without a release too), use [horizon-mod-updater](https://github.com/lospejos/horizon-mod-updater): `.\update_mod.ps1 -Mod trinkets`.
